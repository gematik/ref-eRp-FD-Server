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

mod get;

use actix_web::web::{get, resource, ServiceConfig};
use proc_macros::capability_statement_resource;
use resources::capability_statement::{Interaction, Type};

use get::{get_all, get_one};

use crate::fhir::definitions::RESOURCE_PROFILE_MEDICATION_DISPENSE;

#[derive(Default)]
pub struct MedicationDispenseRoutes;

#[capability_statement_resource(
    type = Type::MedicationDispense,
    profile = RESOURCE_PROFILE_MEDICATION_DISPENSE)]
impl MedicationDispenseRoutes {
    #[interaction(Interaction::Read)]
    fn configure_all(&self, cfg: &mut ServiceConfig) {
        cfg.service(resource("/MedicationDispense").route(get().to(get_all)));
        cfg.service(resource("/MedicationDispense/{id}").route(get().to(get_one)));
    }
}
