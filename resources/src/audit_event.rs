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

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::{
    misc::{Kvnr, ParticipantId, PrescriptionId},
    primitives::{Id, Instant},
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Id,
    pub text: Option<String>,
    pub sub_type: SubType,
    pub action: Action,
    pub recorded: Instant,
    pub outcome: Outcome,
    pub outcome_description: Option<String>,
    pub agent: Agent,
    pub source: Source,
    pub entity: Entity,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Agent {
    pub type_: ParticipationRoleType,
    pub who: Option<ParticipantId>,
    pub name: String,
    pub requestor: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub observer: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entity {
    pub what: Id,
    pub name: Kvnr,
    pub description: PrescriptionId,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Action {
    Create,
    Read,
    Update,
    Delete,
    Execute,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Outcome {
    Success,
    MinorFailure,
    SeriousFailure,
    MajorFailure,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ParticipationRoleType {
    HumanUser,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum SubType {
    Read,
    VRead,
    Update,
    Patch,
    Delete,
    History,
    HistoryInstance,
    HistoryType,
    HistorySystem,
    Create,
    Search,
    SearchType,
    SearchSystem,
    Capabilities,
    Transaction,
    Batch,
    Operation,
    ApplicationStart,
    ApplicationStop,
    Login,
    Logout,
    Attach,
    Detach,
    NodeAuthentication,
    EmergencyOverrideStarted,
    NetworkConfiguration,
    SecurityConfiguration,
    HardwareConfiguration,
    SoftwareConfiguration,
    UseOfRestrictedFunction,
    AuditRecordingStopped,
    AuditRecordingStarted,
    ObjectSecurityAttributesChanged,
    SecurityRolesChanged,
    UserSecurityAttributesChanged,
    EmergencyOverrideStopped,
    RemoteServiceOperationStarted,
    RemoteServiceOperationStopped,
    LocalServiceOperationStarted,
    LocalServiceOperationStopped,
}

impl FromStr for SubType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "read" => Ok(Self::Read),
            "vread" => Ok(Self::VRead),
            "update" => Ok(Self::Update),
            "patch" => Ok(Self::Patch),
            "delete" => Ok(Self::Delete),
            "history" => Ok(Self::History),
            "history-instance" => Ok(Self::HistoryInstance),
            "history-type" => Ok(Self::HistoryType),
            "history-system" => Ok(Self::HistorySystem),
            "create" => Ok(Self::Create),
            "search" => Ok(Self::Search),
            "search-type" => Ok(Self::SearchType),
            "search-system" => Ok(Self::SearchSystem),
            "capabilities" => Ok(Self::Capabilities),
            "transaction" => Ok(Self::Transaction),
            "batch" => Ok(Self::Batch),
            "operation" => Ok(Self::Operation),
            "110120" => Ok(Self::ApplicationStart),
            "110121" => Ok(Self::ApplicationStop),
            "110122" => Ok(Self::Login),
            "110123" => Ok(Self::Logout),
            "110124" => Ok(Self::Attach),
            "110125" => Ok(Self::Detach),
            "110126" => Ok(Self::NodeAuthentication),
            "110127" => Ok(Self::EmergencyOverrideStarted),
            "110128" => Ok(Self::NetworkConfiguration),
            "110129" => Ok(Self::SecurityConfiguration),
            "110130" => Ok(Self::HardwareConfiguration),
            "110131" => Ok(Self::SoftwareConfiguration),
            "110132" => Ok(Self::UseOfRestrictedFunction),
            "110133" => Ok(Self::AuditRecordingStopped),
            "110134" => Ok(Self::AuditRecordingStarted),
            "110135" => Ok(Self::ObjectSecurityAttributesChanged),
            "110136" => Ok(Self::SecurityRolesChanged),
            "110137" => Ok(Self::UserSecurityAttributesChanged),
            "110138" => Ok(Self::EmergencyOverrideStopped),
            "110139" => Ok(Self::RemoteServiceOperationStarted),
            "110140" => Ok(Self::RemoteServiceOperationStopped),
            "110141" => Ok(Self::LocalServiceOperationStarted),
            "110142" => Ok(Self::LocalServiceOperationStopped),
            _ => Err(format!("Invalid sub type: {}", s)),
        }
    }
}
