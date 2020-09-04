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

use std::str::from_utf8;

use asn1_der::{
    error::{Asn1DerError, Asn1DerErrorVariant},
    typed::{DerDecodable, DerTypeView},
    DerObject,
};

#[derive(Debug, PartialEq, Eq)]
pub struct Utf8String<'a>(pub &'a str);

impl<'a> DerTypeView<'a> for Utf8String<'a> {
    const TAG: u8 = 0x0C;

    fn object(&self) -> DerObject<'a> {
        unimplemented!()
    }
}

impl<'a> DerDecodable<'a> for Utf8String<'a> {
    fn load(object: DerObject<'a>) -> Result<Self, Asn1DerError> {
        if object.tag() != Self::TAG {
            return Err(Asn1DerError::new(Asn1DerErrorVariant::InvalidData(
                "DER object is not a UTF-8 string",
            )));
        }

        let value = from_utf8(object.value()).map_err(|_| {
            Asn1DerError::new(Asn1DerErrorVariant::InvalidData(
                "DER object is not a valid UTF-8 string",
            ))
        })?;

        Ok(Self(value))
    }
}
