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

#[cfg(feature = "interface-supplier")]
mod activate;
#[cfg(feature = "interface-supplier")]
mod create;
mod get;
mod misc;

#[cfg(feature = "interface-supplier")]
use activate::activate;
#[cfg(feature = "interface-supplier")]
use create::create;
use get::{get_all, get_one};

#[cfg(feature = "interface-supplier")]
use actix_web::web::post;
use actix_web::web::{get, resource, ServiceConfig};
use proc_macros::capability_statement_resource;
use resources::capability_statement::{Interaction, Type};

use crate::fhir::constants::RESOURCE_PROFILE_TASK;
#[cfg(feature = "interface-supplier")]
use crate::fhir::constants::{OPERATION_TASK_ACTIVATE, OPERATION_TASK_CREATE};

#[derive(Default)]
pub struct TaskRoutes;

#[capability_statement_resource(
    type = Type::Task,
    profile = RESOURCE_PROFILE_TASK)]
impl TaskRoutes {
    #[cfg(feature = "interface-supplier")]
    #[operation(name="create", definition = OPERATION_TASK_CREATE)]
    #[operation(name="activate", definition = OPERATION_TASK_ACTIVATE)]
    fn configure_supplier(&self, cfg: &mut ServiceConfig) {
        cfg.service(resource("/Task/$create").route(post().to(create)));
        cfg.service(resource("/Task/{id}/$activate").route(post().to(activate)));
    }

    #[interaction(Interaction::Read)]
    fn configure_all(&self, cfg: &mut ServiceConfig) {
        cfg.service(resource("/Task").route(get().to(get_all)));
        cfg.service(resource("/Task/{id}").route(get().to(get_one)));
    }
}
