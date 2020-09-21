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

use quick_xml::{events::BytesStart, DeError as Error};
use serde::{ser::SerializeStruct as SerSerializeStruct, Serialize};

use super::Serializer;

pub struct SerializeStruct<'a, W: Write> {
    serializer: &'a mut Serializer<W>,
    tag_id: Option<usize>,
}

impl<'a, W: Write> SerializeStruct<'a, W> {
    pub fn new(serializer: &'a mut Serializer<W>, tag_id: Option<usize>) -> Result<Self, Error> {
        Ok(Self { serializer, tag_id })
    }
}

impl<'a, W: Write> SerSerializeStruct for SerializeStruct<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        if key == "flatten-take-name" {
            self.serializer.update_name();

            value.serialize(&mut *self.serializer)?;
        } else if key.starts_with("attrib=") {
            self.serializer.attrib(Some(key[7..].into()))?;

            value.serialize(&mut *self.serializer)?;

            self.serializer.attrib(None)?;
        } else if key.starts_with("value-tag=") {
            self.serializer.attrib(Some(b"value".to_vec()))?;
            let tag_id = self
                .serializer
                .open_tag(BytesStart::owned_name(key[10..].as_bytes()), false);

            value.serialize(&mut *self.serializer)?;

            self.serializer.close_tag(tag_id)?;
            self.serializer.attrib(None)?;
        } else {
            let key = key.as_bytes();

            let tag_id = self
                .serializer
                .open_tag(BytesStart::borrowed_name(key), false);

            value.serialize(&mut *self.serializer)?;

            self.serializer.close_tag(tag_id)?;
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
