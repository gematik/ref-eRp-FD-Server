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

use super::{
    misc::{Kvnr, PrescriptionId, TelematikId},
    primitives::{DateTime, Id},
};

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MedicationDispense {
    pub id: Option<Id>,
    pub prescription_id: PrescriptionId,
    pub medication: String,
    pub subject: Kvnr,
    pub supporting_information: Vec<String>,
    pub performer: TelematikId,
    pub when_prepared: Option<DateTime>,
    pub when_handed_over: DateTime,
    pub dosage_instruction: Vec<DosageInstruction>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct DosageInstruction {
    pub text: Option<String>,
}
