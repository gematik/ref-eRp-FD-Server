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

use super::primitives::{Date, DateTime, Id};

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
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

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Extension {
    pub emergency_service_fee: bool,
    pub bvg: bool,
    pub co_payment: Option<CoPayment>,
    pub accident_information: Option<AccidentInformation>,
    pub multi_prescription: Option<MultiPrescription>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MultiPrescription {
    pub series_element: SeriesElement,
    pub time_range: TimeRange,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SeriesElement {
    pub numerator: usize,
    pub denominator: usize,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: Option<DateTime>,
    pub end: Option<DateTime>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct AccidentInformation {
    pub cause: AccidentCause,
    pub date: Date,
    pub business: Option<String>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Dosage {
    pub dosage_mark: Option<bool>,
    pub text: Option<String>,
    pub patient_instruction: Option<String>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct DispenseRequest {
    pub quantity: usize,
    pub validity_period_start: Option<DateTime>,
    pub validity_period_end: Option<DateTime>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum CoPayment {
    NotExceptFrom,
    ExceptFrom,
    ArtificialFertilization,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum AccidentCause {
    Accident,
    WorkAccident,
    SupplyProblem,
}
