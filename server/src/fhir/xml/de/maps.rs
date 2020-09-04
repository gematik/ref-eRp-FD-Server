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

use std::collections::HashMap;
use std::io::BufRead;
use std::mem::take;

use quick_xml::{
    events::{
        attributes::{Attribute, Attributes},
        BytesStart, Event,
    },
    DeError as Error,
};
use serde::de::{DeserializeSeed, MapAccess as DeMapAccess};

use super::{values::ValueDeserializer, Deserializer};

pub struct MapAccess<'a, 'b, R: BufRead> {
    deserializer: &'a mut Deserializer<R>,
    fields: HashMap<String, Field>,
    attribs: Attributes<'b>,
    read_attribs_only: Option<bool>,
    value: Value,
}

#[allow(clippy::enum_variant_names)]
enum Value {
    None,
    Done,
    Tag(String),
    ValueTag(String),
    Attribute(Vec<u8>),
}

enum Field {
    Normal,
    Attribute,
    ValueTag,
}

impl Default for Value {
    fn default() -> Self {
        Self::None
    }
}

impl<'a, 'b, R: BufRead> MapAccess<'a, 'b, R> {
    pub fn new(
        deserializer: &'a mut Deserializer<R>,
        fields: &'static [&'static str],
        start: &'b BytesStart<'static>,
        read_attribs_only: bool,
    ) -> Result<Self, Error> {
        let attribs = start.attributes().to_owned();
        let fields = fields
            .iter()
            .map(|field| {
                if field.starts_with("attrib=") {
                    (field[7..].to_owned(), Field::Attribute)
                } else if field.starts_with("value-tag=") {
                    (field[10..].to_owned(), Field::ValueTag)
                } else {
                    (field.to_string(), Field::Normal)
                }
            })
            .collect();

        Ok(MapAccess {
            deserializer,
            fields,
            attribs,
            read_attribs_only: if read_attribs_only { Some(true) } else { None },
            value: Value::None,
        })
    }

    fn next_attrib(&mut self) -> Result<Option<Attribute>, Error> {
        let next_att = self.attribs.next();

        Ok(next_att.transpose()?)
    }

    fn get_key_name(&self, key: &str) -> String {
        match self.fields.get(key) {
            Some(Field::Attribute) => format!("attrib={}", key),
            Some(Field::ValueTag) => format!("value-tag={}", key),
            _ => key.to_owned(),
        }
    }
}

impl<'a, 'b, 'de, R: BufRead> DeMapAccess<'de> for MapAccess<'a, 'b, R> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        if let Some(attrib) = self.next_attrib()? {
            let value = attrib.value.into_owned();

            let key = attrib.key.to_owned();
            let key = self.deserializer.decode(&key)?;
            let key = self.get_key_name(key);

            self.value = Value::Attribute(value);

            seed.deserialize(ValueDeserializer::new(&key)).map(Some)
        } else if let Some(read_attribs_only) = self.read_attribs_only.as_mut() {
            if *read_attribs_only {
                *read_attribs_only = false;
                self.value = Value::Done;

                seed.deserialize(ValueDeserializer::new(&"flatten-take-name"))
                    .map(Some)
            } else {
                Ok(None)
            }
        } else {
            match self.deserializer.peek()? {
                Some(Event::Start(e)) => {
                    let mut is_value_tag = false;
                    let mut attribs = e.attributes();
                    if let Some(Ok(attrib)) = attribs.next() {
                        if attrib.key == &b"value"[..] && attribs.next().is_none() {
                            is_value_tag = true;
                        }
                    }

                    let tag = e.local_name().to_owned();
                    let tag = self.deserializer.decode(&tag)?;
                    let tag = self.get_key_name(tag);

                    let result = seed.deserialize(ValueDeserializer::new(&tag)).map(Some);

                    self.value = if tag.starts_with("value-tag=") {
                        Value::ValueTag(tag[10..].to_owned())
                    } else if is_value_tag {
                        Value::ValueTag(tag)
                    } else {
                        Value::Tag(tag)
                    };

                    result
                }
                _ => Ok(None),
            }
        }
    }

    fn next_value_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<K::Value, Self::Error> {
        match take(&mut self.value) {
            Value::None => Err(Error::EndOfAttributes),
            Value::Done => {
                self.deserializer.clear_expected_start_tag();

                seed.deserialize(&mut *self.deserializer)
            }
            Value::Tag(tag) => {
                self.deserializer.set_expected_start_tag(tag)?;

                let ret = seed.deserialize(&mut *self.deserializer)?;

                self.deserializer.clear_expected_start_tag();

                Ok(ret)
            }
            Value::ValueTag(tag) => {
                self.deserializer.set_expected_start_tag(tag)?;
                self.deserializer.set_value_tag(true);

                let ret = seed.deserialize(&mut *self.deserializer)?;

                self.deserializer.clear_expected_start_tag();
                self.deserializer.set_value_tag(false);

                Ok(ret)
            }
            Value::Attribute(value) => {
                let value = self.deserializer.decode(&value)?;
                seed.deserialize(ValueDeserializer::new(value))
            }
        }
    }
}
