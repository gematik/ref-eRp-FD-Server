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

use resources::types::{DocumentType, FlowType, PerformerType};

use super::primitives::{CodeEx, CodeableConceptEx, CodingEx};

impl CodeEx for FlowType {
    fn from_parts(value: String) -> Result<Self, String> {
        match value.as_str() {
            "160" => Ok(Self::PharmaceuticalDrugs),
            _ => Err(value),
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::PharmaceuticalDrugs => "160",
        }
    }
}

impl CodingEx for FlowType {
    type Code = Self;

    fn from_parts(code: Self::Code) -> Self {
        code
    }

    fn code(&self) -> &Self::Code {
        &self
    }

    fn display(&self) -> Option<&'static str> {
        match self {
            Self::PharmaceuticalDrugs => Some("Muster 16 (Apothekenpflichtige Arzneimittel)"),
        }
    }

    fn system() -> Option<&'static str> {
        Some("https://gematik.de/fhir/CodeSystem/Flowtype")
    }
}

impl CodeableConceptEx for FlowType {
    type Coding = Self;

    fn from_parts(coding: Self::Coding, _text: Option<String>) -> Self {
        coding
    }

    fn coding(&self) -> &Self::Coding {
        &self
    }
}

impl CodeEx for PerformerType {
    fn from_parts(value: String) -> Result<Self, String> {
        match value.as_str() {
            "urn:oid:1.2.276.0.76.4.54" => Ok(Self::PublicPharmacy),
            _ => Err(value),
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::PublicPharmacy => "urn:oid:1.2.276.0.76.4.54",
        }
    }
}

impl CodingEx for PerformerType {
    type Code = Self;

    fn from_parts(code: Self::Code) -> Self {
        code
    }

    fn code(&self) -> &Self::Code {
        &self
    }

    fn display(&self) -> Option<&'static str> {
        match self {
            Self::PublicPharmacy => Some("Öffentliche Apotheke"),
        }
    }

    fn system() -> Option<&'static str> {
        Some("urn:ietf:rfc:3986")
    }
}

impl CodeableConceptEx for PerformerType {
    type Coding = Self;

    fn from_parts(coding: Self::Coding, _text: Option<String>) -> Self {
        coding
    }

    fn coding(&self) -> &Self::Coding {
        &self
    }
}

impl CodeEx for DocumentType {
    fn from_parts(value: String) -> Result<Self, String> {
        match value.as_str() {
            "1" => Ok(Self::EPrescription),
            "2" => Ok(Self::PatientReceipt),
            "3" => Ok(Self::Receipt),
            _ => Err(value),
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::EPrescription => "1",
            Self::PatientReceipt => "2",
            Self::Receipt => "3",
        }
    }
}

impl CodingEx for DocumentType {
    type Code = Self;

    fn from_parts(code: Self::Code) -> Self {
        code
    }

    fn code(&self) -> &Self::Code {
        &self
    }

    fn display(&self) -> Option<&'static str> {
        match self {
            Self::EPrescription => Some("Health Care Provider Prescription"),
            Self::PatientReceipt => Some("Patient Confirmation"),
            Self::Receipt => Some("Receipt"),
        }
    }

    fn system() -> Option<&'static str> {
        Some("https://gematik.de/fhir/CodeSystem/Documenttype")
    }
}

impl CodeableConceptEx for DocumentType {
    type Coding = Self;

    fn from_parts(coding: Self::Coding, _text: Option<String>) -> Self {
        coding
    }

    fn coding(&self) -> &Self::Coding {
        &self
    }
}
