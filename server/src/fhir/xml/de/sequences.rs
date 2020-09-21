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
use serde::de::{DeserializeSeed, SeqAccess as DeSeqAccess};

use super::Deserializer;

pub struct SeqAccess<'a, R: BufRead> {
    deserializer: &'a mut Deserializer<R>,
    max_size: Option<usize>,
    is_value_tag: bool,
    tag: Tag,
}

struct Tag(Option<String>);

impl<'a, R: BufRead> SeqAccess<'a, R> {
    pub fn new(
        deserializer: &'a mut Deserializer<R>,
        tag: Option<String>,
        max_size: Option<usize>,
    ) -> Result<Self, Error> {
        let tag = if let Some(tag) = tag {
            Tag(Some(tag))
        } else if let Some(Event::Start(e)) = deserializer.peek()? {
            let name = e.name().to_owned();

            Tag(Some(deserializer.decode(&name)?.to_owned()))
        } else {
            Tag(None)
        };

        let is_value_tag = deserializer.is_value_tag();

        Ok(SeqAccess {
            deserializer,
            max_size,
            is_value_tag,
            tag,
        })
    }
}

impl Tag {
    fn is_valid(&self, tag: &[u8]) -> bool {
        self.0.as_ref().map(|t| t.as_bytes() == tag).unwrap_or(true)
    }

    fn get(&self) -> Option<String> {
        self.0.clone()
    }
}

impl<'de, 'a, R: 'a + BufRead> DeSeqAccess<'de> for SeqAccess<'a, R> {
    type Error = Error;

    fn size_hint(&self) -> Option<usize> {
        self.max_size
    }

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        if let Some(s) = self.max_size.as_mut() {
            if *s == 0 {
                return Ok(None);
            }

            *s -= 1;
        }

        match self.deserializer.peek()? {
            None | Some(Event::Eof) | Some(Event::End(_)) => Ok(None),
            Some(Event::Start(e)) if !self.tag.is_valid(e.name()) => Ok(None),
            _ => {
                if let Some(tag) = self.tag.get() {
                    self.deserializer.set_expected_start_tag(tag)?;
                }

                if self.is_value_tag {
                    self.deserializer.set_value_tag(true);
                }

                let result = seed.deserialize(&mut *self.deserializer).map(Some);

                self.deserializer.clear_expected_start_tag();

                result
            }
        }
    }
}
