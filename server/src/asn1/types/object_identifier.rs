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

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;

use asn1_der::{
    error::{Asn1DerError, Asn1DerErrorVariant},
    typed::{DerDecodable, DerTypeView},
    DerObject,
};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ObjectIdentifier(Vec<usize>);

impl Display for ObjectIdentifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let s = self
            .0
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(".");

        write!(f, "{}", s)
    }
}

impl FromStr for ObjectIdentifier {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(
            s.split('.')
                .map(|s| s.parse().map_err(|_| ()))
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}

impl<'a> DerTypeView<'a> for ObjectIdentifier {
    const TAG: u8 = 0x06;

    fn object(&self) -> DerObject<'a> {
        unimplemented!()
    }
}

impl<'a> DerDecodable<'a> for ObjectIdentifier {
    fn load(object: DerObject<'a>) -> Result<Self, Asn1DerError> {
        if object.tag() != Self::TAG {
            return Err(Asn1DerError::new(Asn1DerErrorVariant::InvalidData(
                "DER object is not an object identifier",
            )));
        }

        let mut value = object.value();
        if value.is_empty() {
            return Err(Asn1DerError::new(Asn1DerErrorVariant::InvalidData(
                "Invalid object identifier",
            )));
        }

        let mut parts: Vec<usize> = Vec::new();
        parts.push(value[0] as usize / 40);
        parts.push(value[0] as usize % 40);
        value = &value[1..];

        while !value.is_empty() {
            let mut val = 0usize;

            loop {
                match value[0] {
                    v if v & 0b1000_0000 == 0b1000_0000 => {
                        value = &value[1..];
                        if value.is_empty() {
                            return Err(Asn1DerError::new(Asn1DerErrorVariant::InvalidData(
                                "Invalid object identifier",
                            )));
                        }

                        val = add_value(val, (v & 0b0111_1111) as usize)?;
                    }
                    v => {
                        value = &value[1..];
                        val = add_value(val, v as usize)?;
                        break;
                    }
                }
            }

            parts.push(val);
        }

        Ok(ObjectIdentifier(parts))
    }
}

fn add_value(value: usize, add: usize) -> Result<usize, Asn1DerError> {
    Ok(value
        .checked_mul(128)
        .and_then(|v| v.checked_add(add))
        .ok_or_else(|| {
            Asn1DerError::new(Asn1DerErrorVariant::InvalidData(
                "Invalid object identifier",
            ))
        })?)
}
