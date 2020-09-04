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

use std::io::BufRead;
use std::mem::take;

use quick_xml::{
    events::{BytesText, Event},
    DeError as Error, Reader,
};
use serde::{
    de::{Deserializer as DeDeserializer, Visitor},
    serde_if_integer128,
};

use super::{enums::EnumAccess, maps::MapAccess, sequences::SeqAccess};

pub struct Deserializer<R: BufRead> {
    reader: Reader<R>,
    peek: Option<Event<'static>>,
    expected_start_tag: Option<String>,
    is_value_tag: bool,
}

impl<R: BufRead> Deserializer<R> {
    pub fn new(reader: Reader<R>) -> Self {
        Deserializer {
            reader,
            peek: None,
            expected_start_tag: None,
            is_value_tag: false,
        }
    }

    pub fn is_value_tag(&self) -> bool {
        self.is_value_tag
    }

    pub fn set_value_tag(&mut self, value: bool) {
        self.is_value_tag = value;
    }

    pub fn set_expected_start_tag(&mut self, tag: String) -> Result<(), Error> {
        if self.expected_start_tag.is_some() {
            return Err(Error::Custom(
                "Expected start tag is already assigned!".into(),
            ));
        }

        self.expected_start_tag = Some(tag);

        Ok(())
    }

    pub fn clear_expected_start_tag(&mut self) {
        self.expected_start_tag = None;
    }

    pub fn from_reader(reader: R) -> Self {
        let mut reader = Reader::from_reader(reader);
        reader
            .expand_empty_elements(true)
            .check_end_names(true)
            .trim_text(true);

        Self::new(reader)
    }

    pub fn peek(&mut self) -> Result<Option<&Event<'static>>, Error> {
        if self.peek.is_none() {
            self.peek = Some(self.next(&mut Vec::new())?);
        }

        Ok(self.peek.as_ref())
    }

    pub fn put_back(&mut self, e: Event<'static>) -> Result<(), Error> {
        if self.peek.is_some() {
            return Err(Error::Custom(
                "Can not put back more than one event!".to_owned(),
            ));
        }

        self.peek = Some(e);

        Ok(())
    }

    pub fn next<'a>(&mut self, buf: &'a mut Vec<u8>) -> Result<Event<'static>, Error> {
        if let Some(e) = self.peek.take() {
            return Ok(e);
        }

        let e = self.reader.read_event(buf)?;

        Ok(e.into_owned())
    }

    pub fn read_to_end(&mut self, name: &[u8]) -> Result<(), Error> {
        let mut buf = Vec::new();
        match self.next(&mut buf)? {
            Event::Start(e) => self.reader.read_to_end(e.name(), &mut Vec::new())?,
            Event::End(e) if e.name() == name => return Ok(()),
            _ => buf.clear(),
        }

        self.is_value_tag = false;

        Ok(self.reader.read_to_end(name, &mut buf)?)
    }

    pub fn decode<'a>(&self, bytes: &'a [u8]) -> Result<&'a str, Error> {
        Ok(self.reader.decode(bytes)?)
    }

    fn next_bytes(&mut self) -> Result<BytesText<'static>, Error> {
        match self.next(&mut Vec::new())? {
            Event::CData(e) => Ok(e),
            Event::Start(e) => {
                let inner = self.next(&mut Vec::new())?;
                let t = match inner {
                    Event::CData(t) => t,
                    Event::End(end) if end.name() == e.name() => {
                        BytesText::from_escaped(&[] as &[u8])
                    }
                    Event::Eof => return Err(Error::Eof),
                    e => return Err(Error::Custom(format!("Unexpected event: {:?}", e))),
                };

                self.read_to_end(e.name())?;

                Ok(t)
            }
            Event::End(e) => {
                self.put_back(Event::End(e))?;

                Ok(BytesText::from_escaped(&[] as &[u8]))
            }
            Event::Eof => Err(Error::Eof),
            e => Err(Error::Custom(format!("Unexpected event: {:?}", e))),
        }
    }

    fn next_text(&mut self) -> Result<BytesText<'static>, Error> {
        match self.next(&mut Vec::new())? {
            Event::Text(e) | Event::CData(e) => Ok(e),
            Event::Start(e) => {
                let inner = self.next(&mut Vec::new())?;
                let t = match inner {
                    Event::Text(t) | Event::CData(t) => t,
                    Event::Start(s) => {
                        return Err(Error::Custom(format!(
                            "Unexpected start element: {:?}",
                            s.name()
                        )))
                    }
                    Event::End(end) if end.name() == e.name() => {
                        if take(&mut self.is_value_tag) {
                            let attrib = e.attributes().find(|attrib| match attrib {
                                Ok(attrib) if attrib.key == b"value" => true,
                                _ => false,
                            });

                            if let Some(Ok(attrib)) = attrib {
                                return Ok(BytesText::from_escaped(attrib.value.into_owned()));
                            } else {
                                return Err(Error::Custom(
                                    "Value tag does not contain an valid value attribute!".into(),
                                ));
                            }
                        } else {
                            return Ok(BytesText::from_escaped(&[] as &[u8]));
                        }
                    }
                    Event::End(_) => return Err(Error::End),
                    Event::Eof => return Err(Error::Eof),
                    e => return Err(Error::Custom(format!("Unexpected event: {:?}", e))),
                };
                self.read_to_end(e.name())?;
                Ok(t)
            }
            Event::End(e) => {
                self.put_back(Event::End(e))?;

                Ok(BytesText::from_escaped(&[] as &[u8]))
            }
            Event::Eof => Err(Error::Eof),
            e => Err(Error::Custom(format!("Unexpected event: {:?}", e))),
        }
    }
}

macro_rules! deserialize_type {
    ($deserialize:ident, $visit:ident) => {
        fn $deserialize<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
            let value = self.next_text()?;
            let value = self.decode(&*value)?.parse()?;

            visitor.$visit(value)
        }
    };
}

impl<'de, 'a, R: BufRead> DeDeserializer<'de> for &'a mut Deserializer<R> {
    type Error = Error;

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let value = self.next_text()?;
        let value = self.decode(&value)?;

        match value {
            "true" | "1" | "True" | "TRUE" | "t" | "Yes" | "YES" | "yes" | "y" => {
                visitor.visit_bool(true)
            }
            "false" | "0" | "False" | "FALSE" | "f" | "No" | "NO" | "no" | "n" => {
                visitor.visit_bool(false)
            }
            _ => Err(Error::InvalidBoolean(value.into())),
        }
    }

    deserialize_type!(deserialize_i8, visit_i8);
    deserialize_type!(deserialize_i16, visit_i16);
    deserialize_type!(deserialize_i32, visit_i32);
    deserialize_type!(deserialize_i64, visit_i64);

    deserialize_type!(deserialize_u8, visit_u8);
    deserialize_type!(deserialize_u16, visit_u16);
    deserialize_type!(deserialize_u32, visit_u32);
    deserialize_type!(deserialize_u64, visit_u64);

    deserialize_type!(deserialize_f32, visit_f32);
    deserialize_type!(deserialize_f64, visit_f64);

    serde_if_integer128! {
        deserialize_type!(deserialize_i128, visit_i128);
        deserialize_type!(deserialize_u128, visit_u128);
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let value = self.next_text()?;
        let value = self.decode(&*value)?;

        visitor.visit_str(value)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let value = self.next_text()?;
        let value = self.decode(&*value)?.to_string();

        visitor.visit_string(value)
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let bytes = self.next_bytes()?;

        visitor.visit_bytes(bytes.escaped())
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let bytes = self.next_bytes()?;

        visitor.visit_byte_buf(bytes.escaped().to_owned())
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.peek()? {
            Some(Event::Text(t)) if t.is_empty() => visitor.visit_none(),
            None | Some(Event::Eof) => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let mut buf = Vec::new();
        match self.next(&mut buf)? {
            Event::Start(s) => {
                self.read_to_end(s.name())?;

                visitor.visit_unit()
            }
            e => Err(Error::InvalidUnit(format!("{:?}", e))),
        }
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(Error::Custom(
            "Not supported: deserialize_unit_struct".into(),
        ))
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let tag = self.expected_start_tag.take();

        visitor.visit_seq(SeqAccess::new(self, tag, None)?)
    }

    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(Error::Custom("Not supported: deserialize_tuple".into()))
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(Error::Custom(
            "Not supported: deserialize_tuple_struct".into(),
        ))
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_struct("", &[], visitor)
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let mut buf = Vec::new();

        loop {
            match self.next(&mut buf)? {
                Event::Start(e) => {
                    let tag = e.name().to_vec();

                    check_tag_name(
                        self.expected_start_tag.take().as_deref().unwrap_or(&name),
                        &tag,
                    )?;

                    let read_attribs_only = name == "xml:placeholder";
                    if read_attribs_only {
                        self.put_back(Event::Start(e.to_owned()))?;
                    }

                    let map = MapAccess::new(self, fields, &e, read_attribs_only)?;

                    let value = visitor.visit_map(map)?;

                    if !read_attribs_only {
                        self.read_to_end(&tag)?;
                    }

                    return Ok(value);
                }
                Event::End(e) => {
                    if name.as_bytes() != e.name() {
                        return Err(Error::End);
                    }
                }
                Event::Eof => return Err(Error::Eof),
                _ => buf.clear(),
            }
        }
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        match self.next(&mut Vec::new())? {
            Event::Start(e) => {
                let tag = e.name().to_vec();
                let mut attribs = e.attributes();

                if let Some(Ok(attrib)) = attribs.next() {
                    if attrib.key == &b"value"[..] && attribs.next().is_none() {
                        self.is_value_tag = true;
                    }
                }

                check_tag_name(
                    self.expected_start_tag.take().as_deref().unwrap_or(&name),
                    &tag,
                )?;

                if self.is_value_tag {
                    self.put_back(Event::Start(e))?;
                }

                let ret = visitor.visit_enum(EnumAccess::new(self))?;

                self.read_to_end(&tag)?;

                Ok(ret)
            }
            e => Err(Error::Custom(format!("Unexpected event: {:?}", e))),
        }
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        if name.starts_with("value-tag=") {
            let name = &name[10..];
            let expected_tag = self.expected_start_tag.take();

            if let Event::Start(e) = self.peek()?.ok_or(Error::Eof)? {
                check_tag_name(expected_tag.as_deref().unwrap_or(&name), &e.name())?;

                if let Some(expected) = expected_tag {
                    self.set_expected_start_tag(expected)?;
                }

                self.set_value_tag(true);

                visitor.visit_newtype_struct(self)
            } else {
                Err(Error::Custom(format!("Expected start tag: {}", name)))
            }
        } else {
            visitor.visit_newtype_struct(self)
        }
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_string(visitor)
    }

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let is_value_tag = self.is_value_tag;
        match self.peek()?.ok_or(Error::Eof)? {
            Event::Start(_) if !is_value_tag => self.deserialize_map(visitor),
            Event::End(_) => self.deserialize_unit(visitor),
            _ => {
                let value = self.next_text()?;
                let value = self.decode(&value)?;

                match value {
                    "true" | "True" | "TRUE" | "Yes" | "YES" | "yes" => visitor.visit_bool(true),
                    "false" | "False" | "FALSE" | "No" | "NO" | "no" => visitor.visit_bool(false),
                    value => visitor.visit_str(value),
                }
            }
        }
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.next(&mut Vec::new())? {
            Event::Start(e) => self.read_to_end(e.name())?,
            Event::End(_) => return Err(Error::End),
            _ => (),
        }

        visitor.visit_unit()
    }
}

use std::str::from_utf8;

fn check_tag_name(expected: &str, actual: &[u8]) -> Result<(), Error> {
    if !expected.is_empty() && expected.as_bytes() != actual && expected != "xml:placeholder" {
        return Err(Error::Custom(format!(
            "Invalid start tag (actual={:?}, expected={:?})",
            from_utf8(actual).unwrap(),
            expected
        )));
    }

    Ok(())
}
