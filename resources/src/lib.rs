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

#[macro_use]
extern crate lazy_static;

pub mod bundle;
pub mod capability_statement;
pub mod composition;
pub mod coverage;
pub mod kbv_bundle;
pub mod medication;
pub mod medication_request;
pub mod misc;
pub mod organization;
pub mod patient;
pub mod practitioner;
pub mod practitioner_role;
pub mod primitives;
pub mod signed_data;
pub mod task;
pub mod types;

pub use capability_statement::CapabilityStatement;
pub use composition::Composition;
pub use coverage::Coverage;
pub use kbv_bundle::KbvBundle;
pub use medication::Medication;
pub use medication_request::MedicationRequest;
pub use organization::Organization;
pub use patient::Patient;
pub use practitioner::Practitioner;
pub use practitioner_role::PractitionerRole;
pub use signed_data::SignedData;
pub use task::Task;
