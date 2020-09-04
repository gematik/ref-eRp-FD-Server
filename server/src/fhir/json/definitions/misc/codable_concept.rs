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

use resources::misc::Code;
use serde::{Deserialize, Serialize};

use super::CodingDef;

#[serde(rename_all = "camelCase")]
#[derive(Default, Serialize, Deserialize)]
pub struct CodableConceptDef {
    #[serde(default)]
    pub coding: Vec<CodingDef>,

    #[serde(default)]
    pub text: Option<String>,
}

impl From<Code> for CodableConceptDef {
    fn from(code: Code) -> Self {
        Self {
            coding: vec![CodingDef {
                system: Some(code.system),
                code: Some(code.code),
                ..Default::default()
            }],
            ..Default::default()
        }
    }
}

impl TryInto<Code> for CodableConceptDef {
    type Error = String;

    fn try_into(self) -> Result<Code, Self::Error> {
        let coding = self
            .coding
            .into_iter()
            .next()
            .ok_or_else(|| "Codable concept is missing the `coding` field!")?;

        Ok(Code {
            system: coding
                .system
                .ok_or_else(|| "Codable concept coding is missing the `system` field!")?,
            code: coding
                .code
                .ok_or_else(|| "Codable concept coding is missing the `code` field!")?,
        })
    }
}
