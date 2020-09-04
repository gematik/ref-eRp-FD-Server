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

use std::io::Write;
use std::mem::take;

use quick_xml::{
    events::{attributes::Attribute, BytesStart},
    DeError as Error,
};
use serde::{ser::SerializeMap as SerSerializeMap, Serialize};

use super::{values::ValueSerializer, Serializer};

pub struct SerializeMap<'a, W: Write> {
    serializer: &'a mut Serializer<W>,
    pending: Pending,
    tag_id: Option<usize>,
}

#[derive(PartialEq)]
enum Pending {
    None,
    Tag(Vec<u8>),
    Attrib(Vec<u8>),
    AttribTag(Vec<u8>),
}

impl Default for Pending {
    fn default() -> Self {
        Self::None
    }
}

impl<'a, W: Write> SerializeMap<'a, W> {
    pub fn new(serializer: &'a mut Serializer<W>, tag_id: Option<usize>) -> Result<Self, Error> {
        Ok(Self {
            serializer,
            pending: Pending::None,
            tag_id,
        })
    }
}

impl<'a, W: Write> SerSerializeMap for SerializeMap<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Self::Error> {
        let mut attr_ser = ValueSerializer::default();
        key.serialize(&mut attr_ser)?;

        let key = match attr_ser.value() {
            Some(key) => key.to_owned(),
            None => return Err(Error::Custom("Unable to serialize key of map!".to_owned())),
        };

        if self.pending != Pending::None {
            panic!("Can not serialize key of map while an operation is pending!");
        }

        if key.starts_with(&b"attrib="[..]) {
            self.pending = Pending::Attrib(key[7..].to_owned());
        } else if key.starts_with(&b"value-tag="[..]) {
            self.pending = Pending::AttribTag(key[10..].to_owned());
        } else {
            self.pending = Pending::Tag(key);
        }

        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        match take(&mut self.pending) {
            Pending::None => panic!("Can not serialize value of map without key!"),
            Pending::Tag(key) => {
                let tag_id = self.serializer.open_tag(BytesStart::owned_name(key), false);

                value.serialize(&mut *self.serializer)?;

                self.serializer.close_tag(tag_id)?;
            }
            Pending::Attrib(key) => {
                let mut attr_ser = ValueSerializer::default();
                value.serialize(&mut attr_ser)?;

                if let Some(value) = attr_ser.value() {
                    let attrib = Attribute::from((key.as_ref(), value));
                    self.serializer.add_attribute(attrib)?;
                }
            }
            Pending::AttribTag(key) => {
                let mut attr_ser = ValueSerializer::default();
                value.serialize(&mut attr_ser)?;

                if let Some(value) = attr_ser.value() {
                    let attrib = Attribute::from((&b"value"[..], value));
                    let tag_id = self.serializer.open_tag(BytesStart::owned_name(key), false);
                    self.serializer.add_attribute(attrib)?;
                    self.serializer.close_tag(tag_id)?;
                }
            }
        }

        Ok(())
    }

    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        if let Some(end_tag) = self.tag_id.take() {
            self.serializer.close_tag(end_tag)?;
        }

        Ok(())
    }
}
