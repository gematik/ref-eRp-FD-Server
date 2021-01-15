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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FlowType {
    PharmaceuticalDrugs,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PerformerType {
    PublicPharmacy,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DocumentType {
    EPrescription,
    PatientReceipt,
    Receipt,
}

impl Into<usize> for FlowType {
    fn into(self) -> usize {
        match self {
            Self::PharmaceuticalDrugs => 160,
        }
    }
}

impl Into<u64> for FlowType {
    fn into(self) -> u64 {
        match self {
            Self::PharmaceuticalDrugs => 160,
        }
    }
}

impl TryFrom<usize> for FlowType {
    type Error = usize;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            160 => Ok(Self::PharmaceuticalDrugs),
            value => Err(value),
        }
    }
}

impl TryFrom<u64> for FlowType {
    type Error = u64;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            160 => Ok(Self::PharmaceuticalDrugs),
            value => Err(value),
        }
    }
}

impl Display for FlowType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::PharmaceuticalDrugs => write!(f, "Muster 16 (Apothekenpflichtige Arzneimittel)"),
        }
    }
}

impl Display for PerformerType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::PublicPharmacy => write!(f, "Apotheke"),
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
