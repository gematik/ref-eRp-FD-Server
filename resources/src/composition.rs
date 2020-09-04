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

use super::{
    misc::{Decode, Encode, EncodeStr},
    primitives::{DateTime, Id},
};

#[derive(Clone, PartialEq, Debug)]
pub struct Composition {
    pub id: Id,
    pub extension: Extension,
    pub subject: Option<String>,
    pub date: DateTime,
    pub author: Author,
    pub title: String,
    pub attester: Option<String>,
    pub custodian: String,
    pub section: Section,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Extension {
    pub legal_basis: Option<LegalBasis>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Author {
    pub doctor: String,
    pub prf: Option<String>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Section {
    pub regulation: String,
    pub health_insurance_relationship: Option<String>,
    pub asv_performance: Option<String>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum LegalBasis {
    None,
    Asv,
    DischargeManagement,
    Tss,
    SubstituteRegulation,
    SubstituteRegulationWithAsv,
    SubstituteRegulationWithDischargeManagement,
    SubstituteRegulationWithTss,
}

impl Decode for LegalBasis {
    type Code = usize;
    type Auto = ();

    fn decode(code: Self::Code) -> Result<Self, Self::Code> {
        match code {
            0 => Ok(Self::None),
            1 => Ok(Self::Asv),
            4 => Ok(Self::DischargeManagement),
            7 => Ok(Self::Tss),
            10 => Ok(Self::SubstituteRegulation),
            11 => Ok(Self::SubstituteRegulationWithAsv),
            14 => Ok(Self::SubstituteRegulationWithDischargeManagement),
            17 => Ok(Self::SubstituteRegulationWithTss),
            _ => Err(code),
        }
    }
}

impl Encode for LegalBasis {
    type Code = usize;
    type Auto = bool;

    fn encode(&self) -> Self::Code {
        match self {
            Self::None => 0,
            Self::Asv => 1,
            Self::DischargeManagement => 4,
            Self::Tss => 7,
            Self::SubstituteRegulation => 10,
            Self::SubstituteRegulationWithAsv => 11,
            Self::SubstituteRegulationWithDischargeManagement => 14,
            Self::SubstituteRegulationWithTss => 17,
        }
    }
}

impl EncodeStr for LegalBasis {
    fn encode_str(&self) -> String {
        format!("{:02}", self.encode())
    }
}
