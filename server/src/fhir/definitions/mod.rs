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

mod bundle;
mod capability_statement;
mod communication;
mod composition;
mod coverage;
mod kbv_bundle;
mod medication;
mod medication_dispense;
mod medication_request;
mod meta;
mod misc;
mod organization;
mod patient;
mod practitioner;
mod practitioner_role;
mod primitives;
mod task;
mod task_activate_parameters;
mod task_create_parameters;
mod types;

pub use bundle::{DecodeBundleResource, EncodeBundleResource};

pub use communication::{
    PROFILE_BASE as RESOURCE_PROFILE_COMMUNICATION,
    PROFILE_DISPENSE_REQ as RESOURCE_PROFILE_COMMUNICATION_DISPENSE_REQ,
    PROFILE_INFO_REQ as RESOURCE_PROFILE_COMMUNICATION_INFO_REQ,
    PROFILE_REPLY as RESOURCE_PROFILE_COMMUNICATION_REPLY,
    PROFILE_REPRESENTATIVE as RESOURCE_PROFILE_COMMUNICATION_REPRESENTATIVE,
};
pub use medication_dispense::PROFILE as RESOURCE_PROFILE_MEDICATION_DISPENSE;
pub use task::{
    OPERATION_ABORT as OPERATION_TASK_ABORT, OPERATION_ACTIVATE as OPERATION_TASK_ACTIVATE,
    OPERATION_CREATE as OPERATION_TASK_CREATE, PROFILE as RESOURCE_PROFILE_TASK,
};
