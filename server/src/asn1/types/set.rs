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

use asn1_der::{
    error::{Asn1DerError, Asn1DerErrorVariant},
    typed::{DerDecodable, DerTypeView},
    DerObject,
};

pub struct Set<'a>(DerObject<'a>);

pub struct SetIterator<'a> {
    buffer: &'a [u8],
    offset: usize,
}

impl<'a> DerTypeView<'a> for Set<'a> {
    const TAG: u8 = 0x31;

    fn object(&self) -> DerObject<'a> {
        unimplemented!()
    }
}

impl<'a> DerDecodable<'a> for Set<'a> {
    fn load(object: DerObject<'a>) -> Result<Self, Asn1DerError> {
        if object.tag() != Self::TAG {
            return Err(Asn1DerError::new(Asn1DerErrorVariant::InvalidData(
                "DER object is not a set",
            )));
        }

        Ok(Self(object))
    }
}

impl<'a> IntoIterator for Set<'a> {
    type Item = Result<DerObject<'a>, Asn1DerError>;
    type IntoIter = SetIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            buffer: self.0.value(),
            offset: 0,
        }
    }
}

impl<'a> Iterator for SetIterator<'a> {
    type Item = Result<DerObject<'a>, Asn1DerError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.buffer.len() {
            return None;
        }

        match DerObject::decode_at(self.buffer, self.offset) {
            Err(err) => {
                self.offset = self.buffer.len();

                Some(Err(err))
            }
            Ok(der) => {
                self.offset += der.header().len();
                self.offset += der.value().len();

                Some(Ok(der))
            }
        }
    }
}
