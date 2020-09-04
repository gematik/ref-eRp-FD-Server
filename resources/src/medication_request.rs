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
    misc::{Decode, Encode},
    primitives::{Date, DateTime, Id},
};

#[derive(Clone, PartialEq, Debug)]
pub struct MedicationRequest {
    pub id: Id,
    pub extension: Extension,
    pub medication: String,
    pub subject: String,
    pub authored_on: DateTime,
    pub requester: String,
    pub insurance: String,
    pub note: Option<String>,
    pub dosage: Option<Dosage>,
    pub dispense_request: DispenseRequest,
    pub substitution_allowed: bool,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Extension {
    pub emergency_service_fee: bool,
    pub bvg: bool,
    pub co_payment: Option<CoPayment>,
    pub accident_information: Option<AccidentInformation>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct AccidentInformation {
    pub cause: AccidentCause,
    pub date: Date,
    pub business: Option<String>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Dosage {
    pub dosage_mark: Option<bool>,
    pub text: Option<String>,
    pub patient_instruction: Option<String>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct DispenseRequest {
    pub quantity: usize,
    pub validity_period_start: Option<DateTime>,
    pub validity_period_end: Option<DateTime>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum CoPayment {
    NotExceptFrom,
    ExceptFrom,
    ArtificialFertilization,
}

#[derive(Clone, PartialEq, Debug)]
pub enum AccidentCause {
    Accident,
    WorkAccident,
    SupplyProblem,
}

impl Decode for CoPayment {
    type Code = usize;
    type Auto = ();

    fn decode(code: Self::Code) -> Result<Self, Self::Code> {
        match code {
            0 => Ok(Self::NotExceptFrom),
            1 => Ok(Self::ExceptFrom),
            2 => Ok(Self::ArtificialFertilization),
            _ => Err(code),
        }
    }
}

impl Encode for CoPayment {
    type Code = usize;
    type Auto = ();

    fn encode(&self) -> Self::Code {
        match self {
            Self::NotExceptFrom => 0,
            Self::ExceptFrom => 1,
            Self::ArtificialFertilization => 2,
        }
    }
}

impl Decode for AccidentCause {
    type Code = usize;
    type Auto = ();

    fn decode(code: Self::Code) -> Result<Self, Self::Code> {
        match code {
            1 => Ok(Self::Accident),
            2 => Ok(Self::WorkAccident),
            3 => Ok(Self::SupplyProblem),
            _ => Err(code),
        }
    }
}

impl Encode for AccidentCause {
    type Code = usize;
    type Auto = ();

    fn encode(&self) -> Self::Code {
        match self {
            Self::Accident => 1,
            Self::WorkAccident => 2,
            Self::SupplyProblem => 3,
        }
    }
}
