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

use std::convert::TryFrom;
use std::fmt::{Display, Formatter, Result as FmtResult};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum FlowType {
    #[serde(alias = "PharmaceuticalDrugs")]
    ApothekenpflichtigeArzneimittel,
    Sanitaetsbedarf,
    Heilmittel,
    Hilfsmittel,
    Sprechstundenbedarf,
    Betaeubungsmittel,
    TRezepte,
    DirekteZuweisung,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum PerformerType {
    PublicPharmacy,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum DocumentType {
    EPrescription,
    PatientReceipt,
    Receipt,
}

impl Into<usize> for FlowType {
    fn into(self) -> usize {
        match self {
            Self::ApothekenpflichtigeArzneimittel => 160,
            Self::Sanitaetsbedarf => 161,
            Self::Heilmittel => 162,
            Self::Hilfsmittel => 163,
            Self::Sprechstundenbedarf => 164,
            Self::Betaeubungsmittel => 165,
            Self::TRezepte => 166,
            Self::DirekteZuweisung => 169,
        }
    }
}

impl Into<u64> for FlowType {
    fn into(self) -> u64 {
        match self {
            Self::ApothekenpflichtigeArzneimittel => 160,
            Self::Sanitaetsbedarf => 161,
            Self::Heilmittel => 162,
            Self::Hilfsmittel => 163,
            Self::Sprechstundenbedarf => 164,
            Self::Betaeubungsmittel => 165,
            Self::TRezepte => 166,
            Self::DirekteZuweisung => 169,
        }
    }
}

impl TryFrom<usize> for FlowType {
    type Error = usize;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            160 => Ok(Self::ApothekenpflichtigeArzneimittel),
            161 => Ok(Self::Sanitaetsbedarf),
            162 => Ok(Self::Heilmittel),
            163 => Ok(Self::Hilfsmittel),
            164 => Ok(Self::Sprechstundenbedarf),
            165 => Ok(Self::Betaeubungsmittel),
            166 => Ok(Self::TRezepte),
            169 => Ok(Self::DirekteZuweisung),
            value => Err(value),
        }
    }
}

impl TryFrom<u64> for FlowType {
    type Error = u64;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            160 => Ok(Self::ApothekenpflichtigeArzneimittel),
            161 => Ok(Self::Sanitaetsbedarf),
            162 => Ok(Self::Heilmittel),
            163 => Ok(Self::Hilfsmittel),
            164 => Ok(Self::Sprechstundenbedarf),
            165 => Ok(Self::Betaeubungsmittel),
            166 => Ok(Self::TRezepte),
            169 => Ok(Self::DirekteZuweisung),
            value => Err(value),
        }
    }
}

impl Display for FlowType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::ApothekenpflichtigeArzneimittel => {
                write!(f, "Muster 16 (Apothekenpflichtige Arzneimittel)")
            }
            Self::Sanitaetsbedarf => write!(f, "Muster 16 (Sanitätsbedarf)"),
            Self::Heilmittel => write!(f, "Muster 16 (Heilmittel)"),
            Self::Hilfsmittel => write!(f, "Muster 16 (Hilfsmittel)"),
            Self::Sprechstundenbedarf => write!(f, "Muster 16 (Sprechstundenbedarf)"),
            Self::Betaeubungsmittel => write!(f, "Muster 16 (Betäubungsmittel)"),
            Self::TRezepte => write!(f, "Muster 16 (T-Rezepte)"),
            Self::DirekteZuweisung => write!(f, "Muster 16 (Direkte Zuweisung)"),
        }
    }
}

impl Display for PerformerType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::PublicPharmacy => write!(f, "Öffentliche Apotheke"),
        }
    }
}

impl Display for DocumentType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::EPrescription => write!(f, "Health Care Provider Prescription"),
            Self::PatientReceipt => write!(f, "Patient Confirmation"),
            Self::Receipt => write!(f, "Receipt"),
        }
    }
}
