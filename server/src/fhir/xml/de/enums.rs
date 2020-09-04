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

use quick_xml::{events::Event, DeError as Error};
use serde::de::{
    DeserializeSeed, Deserializer as SerdeDeserializer, EnumAccess as DeEnumAccess,
    VariantAccess as DeVariantAccess, Visitor,
};

use super::{values::ValueDeserializer, Deserializer};

pub struct EnumAccess<'a, R: BufRead> {
    deserializer: &'a mut Deserializer<R>,
}

pub struct VariantAccess<'a, R: BufRead> {
    deserializer: &'a mut Deserializer<R>,
}

impl<'a, R: BufRead> EnumAccess<'a, R> {
    pub fn new(deserializer: &'a mut Deserializer<R>) -> Self {
        EnumAccess { deserializer }
    }
}

impl<'de, 'a, R: 'a + BufRead> DeEnumAccess<'de> for EnumAccess<'a, R> {
    type Error = Error;
    type Variant = VariantAccess<'a, R>;

    fn variant_seed<V: DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, VariantAccess<'a, R>), Error> {
        let is_value_tag = self.deserializer.is_value_tag();

        let (value, is_attrib_tag) = match self.deserializer.peek()? {
            Some(Event::Text(t)) => {
                let value = t.escaped().to_owned();

                (
                    self.deserializer.decode(&value)?.as_bytes().to_owned(),
                    false,
                )
            }
            Some(Event::Start(e)) => {
                if is_value_tag {
                    let mut attribs = e.attributes();

                    let attrib = if let Some(Ok(attrib)) = attribs.next() {
                        attrib
                    } else {
                        return Err(Error::Custom(format!("Invalid value tag: {:?}", e)));
                    };

                    if attrib.key != &b"value"[..] || attribs.next().is_some() {
                        return Err(Error::Custom(format!("Invalid value tag: {:?}", e)));
                    }

                    (attrib.value.into_owned(), false)
                } else {
                    let mut is_attrib_tag = false;
                    let mut attribs = e.attributes();
                    if let Some(Ok(attrib)) = attribs.next() {
                        if attrib.key == &b"value"[..] && attribs.next().is_none() {
                            is_attrib_tag = true;
                        }
                    }

                    let value = e.name().to_owned();

                    (
                        self.deserializer.decode(&value)?.as_bytes().to_owned(),
                        is_attrib_tag,
                    )
                }
            }
            Some(e) => return Err(Error::InvalidEnum(e.to_owned())),
            None => return Err(Error::Eof),
        };

        let value = self.deserializer.decode(&value)?;
        let deserializer = ValueDeserializer::new(value);
        let name = seed.deserialize(deserializer)?;

        if is_attrib_tag {
            self.deserializer.set_value_tag(true);
        }

        Ok((
            name,
            VariantAccess {
                deserializer: self.deserializer,
            },
        ))
    }
}

impl<'de, 'a, R: BufRead> DeVariantAccess<'de> for VariantAccess<'a, R> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        match self.deserializer.next(&mut Vec::new())? {
            Event::Start(_) => Ok(()),
            Event::Text(_) => Ok(()),
            _ => unreachable!(),
        }
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Error> {
        seed.deserialize(&mut *self.deserializer)
    }

    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value, Error> {
        self.deserializer.deserialize_tuple(len, visitor)
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserializer.deserialize_struct("", fields, visitor)
    }
}
