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

pub mod misc;
pub mod primitives;

mod bundle;
mod capability_statement;
mod communication;
mod composition;
mod coverage;
#[cfg(feature = "interface-supplier")]
mod kbv_bundle;
mod medication;
mod medication_request;
mod organization;
mod patient;
mod practitioner;
mod practitioner_role;
mod task;

pub use bundle::{BundleCow, BundleDef, BundleRoot};
pub use capability_statement::{CapabilityStatementDef, CapabilityStatementRoot};
pub use communication::{CommunicationCow, CommunicationDef, CommunicationRoot};
pub use composition::CompositionDef;
pub use coverage::CoverageDef;
#[cfg(feature = "interface-supplier")]
pub use kbv_bundle::KbvBundleDef;
pub use medication::MedicationDef;
pub use medication_request::MedicationRequestDef;
pub use organization::OrganizationDef;
pub use patient::PatientDef;
pub use practitioner::PractitionerDef;
pub use practitioner_role::PractitionerRoleDef;
#[cfg(feature = "interface-supplier")]
pub use task::{
    TaskActivateParametersDef, TaskActivateParametersRoot, TaskCreateParametersDef,
    TaskCreateParametersRoot,
};
pub use task::{TaskCow, TaskDef, TaskRoot};
