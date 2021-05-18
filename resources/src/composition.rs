/*
 * Copyright (c) 2021 gematik GmbH
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

use serde::{Deserialize, Serialize};

use super::primitives::{DateTime, Id};

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Composition {
    pub id: Id,
    pub extension: Extension,
    pub subject: Option<String>,
    pub date: DateTime,
    pub author: Author,
    pub attester: Option<String>,
    pub custodian: String,
    pub section: Section,
}

#[derive(Clone, PartialEq, Default, Debug, Serialize, Deserialize)]
pub struct Extension {
    pub legal_basis: Option<LegalBasis>,
    pub pkv: Option<PKV>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Author {
    pub doctor: String,
    pub prf: Option<String>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Section {
    pub prescription: Option<String>,
    pub practice_supply: Option<String>,
    pub coverage: Option<String>,
    pub practitioner_role: Option<String>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
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

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum PKV {
    Standard,
    Basic,
    Individual,
    Emergency,
}
