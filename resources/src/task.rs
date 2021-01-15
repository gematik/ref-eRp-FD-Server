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

use super::{
    misc::{Kvnr, PrescriptionId},
    primitives::{DateTime, Id},
    types::{FlowType, PerformerType},
};

#[derive(Clone, Debug, PartialEq)]
pub struct Task {
    pub id: Option<Id>,
    pub extension: Extension,
    pub identifier: Identifier,
    pub status: Status,
    pub for_: Option<Kvnr>,
    pub authored_on: Option<DateTime>,
    pub last_modified: Option<DateTime>,
    pub performer_type: Vec<PerformerType>,
    pub input: Input,
    pub output: Output,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TaskCreateParameters {
    pub flow_type: FlowType,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TaskActivateParameters {
    pub data: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Extension {
    pub flow_type: FlowType,
    pub accept_date: Option<DateTime>,
    pub expiry_date: Option<DateTime>,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Identifier {
    pub prescription_id: Option<PrescriptionId>,
    pub access_code: Option<String>,
    pub secret: Option<String>,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Input {
    pub e_prescription: Option<Id>,
    pub patient_receipt: Option<Id>,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Output {
    pub receipt: Option<Id>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Status {
    Draft,
    Requested,
    Received,
    Accepted,
    Rejected,
    Ready,
    Cancelled,
    InProgress,
    OnHold,
    Failed,
    Completed,
    EnteredInError,
}
