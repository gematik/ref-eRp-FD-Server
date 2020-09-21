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

use super::misc::{Decode, Encode};

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

impl Decode for FlowType {
    type Code = usize;
    type Auto = ();

    fn decode(code: Self::Code) -> Result<Self, Self::Code> {
        match code {
            160 => Ok(Self::PharmaceuticalDrugs),
            _ => Err(code),
        }
    }
}

impl Encode for FlowType {
    type Code = usize;
    type Auto = ();

    fn encode(&self) -> Self::Code {
        match self {
            Self::PharmaceuticalDrugs => 160,
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

impl Decode for PerformerType {
    type Code = String;
    type Auto = bool;

    fn decode(code: Self::Code) -> Result<Self, Self::Code> {
        match code.as_str() {
            "urn:oid:1.2.276.0.76.4.54" => Ok(Self::PublicPharmacy),
            _ => Err(code),
        }
    }
}

impl Encode for PerformerType {
    type Code = &'static str;
    type Auto = ();

    fn encode(&self) -> Self::Code {
        match self {
            Self::PublicPharmacy => "urn:oid:1.2.276.0.76.4.54",
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

impl Decode for DocumentType {
    type Code = String;
    type Auto = bool;

    fn decode(code: Self::Code) -> Result<Self, Self::Code> {
        match code.as_str() {
            "1" => Ok(Self::EPrescription),
            "2" => Ok(Self::PatientReceipt),
            "3" => Ok(Self::Receipt),
            _ => Err(code),
        }
    }
}

impl Encode for DocumentType {
    type Code = &'static str;
    type Auto = ();

    fn encode(&self) -> &'static str {
        match self {
            Self::EPrescription => "1",
            Self::PatientReceipt => "2",
            Self::Receipt => "3",
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
