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
use std::mem::replace;

use quick_xml::{events::BytesStart, DeError as Error};
use serde::{ser::SerializeSeq as SerSerializeSeq, Serialize};

use super::Serializer;

pub struct SerializeSeq<'a, W: Write> {
    serializer: &'a mut Serializer<W>,
    tag: Vec<u8>,
    tag_id: usize,
    is_first: bool,
}

impl<'a, W: Write> SerializeSeq<'a, W> {
    pub fn new(serializer: &'a mut Serializer<W>, tag: Vec<u8>) -> Self {
        let tag_id = serializer.current_tag_id();

        Self {
            serializer,
            tag,
            tag_id,
            is_first: true,
        }
    }
}

impl<'a, W: Write> SerSerializeSeq for SerializeSeq<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(
        &mut self,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        let is_first = replace(&mut self.is_first, false);

        if !is_first {
            self.serializer.close_tag(self.tag_id)?;
            self.serializer
                .open_tag(BytesStart::owned_name(self.tag.clone()), false);
        }

        value.serialize(&mut *self.serializer)?;

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        if self.is_first {
            self.serializer.drop_tag()?;
        }
        Ok(())
    }
}
