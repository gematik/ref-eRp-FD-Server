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

use std::convert::TryInto;

use resources::{
    misc::Code,
    primitives::{Date, DateTime},
};
use serde::{Deserialize, Serialize};

use super::{
    super::primitives::{DateDef, DateTimeDef},
    CodingDef, IdentifierDef,
};

#[derive(Serialize, Deserialize)]
pub enum ValueDef {
    #[serde(with = "DateTimeDef")]
    #[serde(rename = "valueDateTime")]
    DateTime(DateTime),

    #[serde(with = "DateDef")]
    #[serde(rename = "valueDate")]
    Date(Date),

    #[serde(rename = "valueCoding")]
    Coding(CodingDef),

    #[serde(rename = "valueCode")]
    Code(String),

    #[serde(rename = "valueIdentifier")]
    Identifier(IdentifierDef),

    #[serde(rename = "valueString")]
    String(String),

    #[serde(rename = "valueBoolean")]
    Boolean(bool),
}

impl From<Code> for ValueDef {
    fn from(code: Code) -> Self {
        Self::Coding(CodingDef {
            system: Some(code.system),
            code: Some(code.code),
            ..Default::default()
        })
    }
}

impl TryInto<Code> for ValueDef {
    type Error = String;

    fn try_into(self) -> Result<Code, Self::Error> {
        match self {
            ValueDef::Coding(coding) => Ok(Code {
                system: coding
                    .system
                    .ok_or_else(|| "Value coding is missing the `system` field!")?,
                code: coding
                    .code
                    .ok_or_else(|| "Value coding is missing the `code` field!")?,
            }),
            _ => Err("Value is missing the `valueCoding` field!".to_owned()),
        }
    }
}

impl TryInto<bool> for ValueDef {
    type Error = &'static str;

    fn try_into(self) -> Result<bool, Self::Error> {
        if let ValueDef::Boolean(value) = self {
            Ok(value)
        } else {
            Err("Value does not contain a boolean!")
        }
    }
}
