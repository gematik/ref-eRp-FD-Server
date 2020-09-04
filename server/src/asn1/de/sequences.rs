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

use asn1_der::{error::Asn1DerError, DerObject};
use serde::de::{DeserializeSeed, SeqAccess as DeSeqAccess};

use super::{
    deserializer::{Arguments, Root},
    Deserializer, Error,
};

pub struct SeqAccess<'a, I> {
    root: Root<'a>,
    iter: I,
    fields: Option<&'static [&'static str]>,
    peek: Option<DerObject<'a>>,
    pos: usize,
}

impl<'a, I> SeqAccess<'a, I>
where
    I: Iterator<Item = Result<DerObject<'a>, Asn1DerError>>,
{
    pub fn new(root: Root<'a>, iter: I, fields: Option<&'static [&'static str]>) -> Self {
        Self {
            root,
            iter,
            fields,
            peek: None,
            pos: 0,
        }
    }
}

impl<'a, I> DeSeqAccess<'a> for SeqAccess<'a, I>
where
    I: Iterator<Item = Result<DerObject<'a>, Asn1DerError>>,
{
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'a>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        let der = if let Some(der) = self.peek.take() {
            der
        } else {
            match self.iter.next() {
                Some(der) => der?,
                None => {
                    if let Some(fields) = self.fields {
                        if self.pos < fields.len() {
                            DerObject::decode(NULL_DATA)?
                        } else {
                            return Ok(None);
                        }
                    } else {
                        return Ok(None);
                    }
                }
            }
        };

        let name = if let Some(fields) = self.fields {
            if fields.len() <= self.pos {
                return Ok(None);
            }

            fields[self.pos]
        } else {
            ""
        };

        self.pos += 1;

        let args = Arguments::from_str(name)?;
        let der = self.root.get_oid_wrapped(der, args.oid)?;

        let mut de = Deserializer::new(self.root, der);
        de.accepted_tags(args.tags);

        let ret = seed.deserialize(&mut de)?;

        self.peek = de.extract();

        Ok(Some(ret))
    }
}

const NULL_DATA: &[u8] = &[0x05, 0x00];
