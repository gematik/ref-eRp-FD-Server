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

mod error;
mod get;

pub use error::Error;

use get::{get_all, get_one};

use actix_web::web::{get, resource, ServiceConfig};
use proc_macros::capability_statement_resource;
use resources::capability_statement::{Interaction, Type};

use crate::fhir::definitions::RESOURCE_PROFILE_AUDIT_EVENT;

#[derive(Default)]
pub struct AutidEventRoutes;

#[capability_statement_resource(
    type = Type::AuditEvent,
    profile = RESOURCE_PROFILE_AUDIT_EVENT,
)]
impl AutidEventRoutes {
    #[interaction(Interaction::Read)]
    fn configure_all(&self, cfg: &mut ServiceConfig) {
        cfg.service(resource("/AuditEvent").route(get().to(get_all)));
        cfg.service(resource("/AuditEvent/{id}").route(get().to(get_one)));
    }
}
