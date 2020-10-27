/*
 * Copyright (c) 2020 gematik GmbH
 * 
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 * 
 *    http://www.apache.org/licenses/LICENSE-2.0
 * 
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 */

use std::fmt::Display;
use std::io::Write;
use std::matches;
use std::mem::{replace, take};

use quick_xml::{
    events::{attributes::Attribute, BytesEnd, BytesStart, BytesText, Event},
    DeError as Error, Writer,
};
use serde::{
    ser::{Impossible, Serializer as SerSerializer},
    serde_if_integer128, Serialize,
};

use super::{
    maps::SerializeMap, sequences::SerializeSeq, structs::SerializeStruct, values::ValueSerializer,
};

pub struct Serializer<W: Write> {
    writer: Writer<W>,
    tags: Vec<StartTag>,
    attrib: Option<Vec<u8>>,
    update_name: bool,
}

enum StartTag {
    Pending(BytesStart<'static>, bool),
    Written(BytesEnd<'static>, bool),
    Dropped(bool),
}

impl<W: Write> Serializer<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer: Writer::new(writer),
            tags: Vec::new(),
            attrib: None,
            update_name: false,
        }
    }

    pub fn new_with_indent(writer: W, indent_char: u8, indent_size: usize) -> Self {
        Self {
            writer: Writer::new_with_indent(writer, indent_char, indent_size),
            tags: Vec::new(),
            attrib: None,
            update_name: false,
        }
    }

    pub fn attrib(&mut self, attrib: Option<Vec<u8>>) -> Result<(), Error> {
        if self.attrib.is_some() && attrib.is_some() {
            return Err(Error::Custom(
                "Can not write attribute within attribute!".into(),
            ));
        }

        self.attrib = attrib;

        Ok(())
    }

    pub fn update_name(&mut self) {
        self.update_name = true;
    }

    pub fn write_event(&mut self, event: Event) -> Result<(), Error> {
        self.writer.write_event(event)?;

        Ok(())
    }

    pub fn has_pending_tag(&self) -> bool {
        matches!(self.tags.last(), Some(StartTag::Pending(_, _)))
    }

    pub fn current_tag_id(&self) -> usize {
        self.tags.len()
    }

    pub fn open_tag(&mut self, tag: BytesStart<'static>, auto_close: bool) -> usize {
        self.tags.push(StartTag::Pending(tag, auto_close));

        self.current_tag_id()
    }

    pub fn close_tag(&mut self, id: usize) -> Result<(), Error> {
        if id < self.tags.len() {
            self.auto_close_tag()?;
        }

        if id != self.tags.len() {
            return Err(Error::Custom(format!("Invalid tag id: {}", id)));
        }

        self.pop_tag()?;

        Ok(())
    }

    pub fn drop_tag(&mut self) -> Result<(), Error> {
        if let Some(tag) = self.tags.last_mut() {
            match tag {
                StartTag::Written(_, _) => {
                    Err(Error::Custom("Star tag was already written!".into()))
                }
                StartTag::Pending(_, auto_close) => {
                    *tag = StartTag::Dropped(*auto_close);

                    Ok(())
                }
                StartTag::Dropped(auto_close) => {
                    *tag = StartTag::Dropped(*auto_close);

                    Ok(())
                }
            }
        } else {
            Err(Error::Custom("No pending start tag!".into()))
        }
    }

    pub fn get_start_tag(&self) -> Result<&[u8], Error> {
        match self.tags.last() {
            Some(StartTag::Pending(b, _)) => Ok(b.name()),
            Some(StartTag::Written(b, _)) => Ok(b.name()),
            _ => Err(Error::Custom("No start tag opened!".to_owned())),
        }
    }

    pub fn add_attribute(&mut self, attrib: Attribute) -> Result<(), Error> {
        if let Some(StartTag::Pending(b, _)) = self.tags.last_mut() {
            b.push_attribute(attrib);

            Ok(())
        } else {
            Err(Error::Custom("No pending tag!".to_owned()))
        }
    }

    fn auto_close_tag(&mut self) -> Result<(), Error> {
        while self.is_auto_close_tag() {
            self.pop_tag()?;
        }

        Ok(())
    }

    fn is_auto_close_tag(&self) -> bool {
        match self.tags.last() {
            Some(StartTag::Pending(_, auto_close)) => *auto_close,
            Some(StartTag::Written(_, auto_close)) => *auto_close,
            Some(StartTag::Dropped(auto_close)) => *auto_close,
            None => false,
        }
    }

    fn pop_tag(&mut self) -> Result<(), Error> {
        match self.tags.pop().unwrap() {
            StartTag::Pending(b, _) => {
                self.write_pending_tags()?;
                self.write_event(Event::Empty(b))?;
            }
            StartTag::Written(b, _) => {
                self.write_pending_tags()?;
                self.write_event(Event::End(b))?;
            }
            StartTag::Dropped(_) => (),
        }

        Ok(())
    }

    fn update_pending_name(&mut self, name: &str) -> Result<(), Error> {
        if take(&mut self.update_name) {
            if let Some(StartTag::Pending(tag, _)) = self.tags.last_mut() {
                if tag.name() != b"xml:placeholder" {
                    return Err(Error::Custom(
                        "Pending tag is not a placeholder!".to_owned(),
                    ));
                }

                tag.set_name(name.as_bytes());
            } else {
                return Err(Error::Custom("No pending tag!".to_owned()));
            }
        }

        Ok(())
    }

    fn serialize_primitive<V: Display>(&mut self, value: V) -> Result<(), Error> {
        match self.attrib.clone() {
            None => {
                self.write_pending_tags()?;

                let s = value.to_string();
                let t = BytesText::from_plain_str(&s);
                let e = Event::Text(t);

                self.write_event(e)
            }
            Some(attrib) => {
                let value = value.to_string();
                let attrib = Attribute::from((&attrib[..], value.as_bytes()));

                self.add_attribute(attrib)
            }
        }
    }

    fn write_pending_tags(&mut self) -> Result<(), Error> {
        let mut events = Vec::new();

        for tag in &mut self.tags {
            *tag = match replace(tag, StartTag::Dropped(false)) {
                StartTag::Pending(b, auto_close) => {
                    let end = BytesEnd::owned(b.name().to_owned());

                    events.push(Event::Start(b));

                    StartTag::Written(end, auto_close)
                }
                tag => tag,
            }
        }

        for event in events.into_iter() {
            self.write_event(event)?;
        }

        Ok(())
    }
}

impl<W: Write> Drop for Serializer<W> {
    fn drop(&mut self) {
        self.auto_close_tag().unwrap();
    }
}

impl<'a, W: Write> SerSerializer for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = SerializeSeq<'a, W>;
    type SerializeTuple = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;
    type SerializeMap = SerializeMap<'a, W>;
    type SerializeStruct = SerializeStruct<'a, W>;
    type SerializeStructVariant = Impossible<Self::Ok, Self::Error>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.serialize_primitive(if v { "true" } else { "false" })
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_primitive(v)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_primitive(v)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.serialize_primitive(v)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.serialize_primitive(v)
    }

    serde_if_integer128! {
        fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
            self.serialize_primitive(v)
        }
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_primitive(v)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_primitive(v)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.serialize_primitive(v)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.serialize_primitive(v)
    }

    serde_if_integer128! {
        fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error>  {
            self.serialize_primitive(v)
        }
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.serialize_primitive(v)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.serialize_primitive(v)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.serialize_primitive(v)
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.serialize_primitive(v)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        let b = BytesText::from_escaped(v);
        let e = Event::CData(b);

        self.write_event(e)?;

        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.drop_tag()?;

        Ok(())
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.update_pending_name(name)?;

        let n = BytesStart::borrowed_name(name.as_bytes());
        let e = Event::Empty(n);

        self.write_event(e)?;

        Ok(())
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        if name.starts_with("value-tag=") {
            let attrib = Attribute::from(("value", variant));
            self.add_attribute(attrib)
        } else {
            self.serialize_primitive(variant)
        }
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        if name.starts_with("value-tag=") {
            self.update_pending_name(&name[10..])?;

            let mut attr_ser = ValueSerializer::default();
            value.serialize(&mut attr_ser)?;

            if let Some(value) = attr_ser.value() {
                let attrib = Attribute::from((&b"value"[..], value));
                self.add_attribute(attrib)?;
            } else {
                self.drop_tag()?;
            }
        } else {
            self.update_pending_name(name)?;

            value.serialize(self)?;
        }

        Ok(())
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        self.open_tag(BytesStart::owned_name(variant), true);

        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        let start_tag = self.get_start_tag()?.to_owned();

        Ok(SerializeSeq::new(self, start_tag))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(Error::Unsupported("serialize_tuple"))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(Error::Unsupported("serialize_tuple_struct"))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(Error::Unsupported("serialize_tuple_variant"))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        let tag_id = if !self.has_pending_tag() {
            Some(self.open_tag(BytesStart::owned_name("xml:placeholder"), false))
        } else {
            None
        };

        Ok(SerializeMap::new(self, tag_id)?)
    }

    fn serialize_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        let tag_id = if !self.has_pending_tag() {
            Some(self.open_tag(BytesStart::owned_name(name), false))
        } else {
            self.update_pending_name(name)?;

            None
        };

        Ok(SerializeStruct::new(self, tag_id)?)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(Error::Unsupported("serialize_struct_variant"))
    }
}
