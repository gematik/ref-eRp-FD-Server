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

mod abort;
mod accept;
#[cfg(feature = "interface-supplier")]
mod activate;
mod close;
#[cfg(feature = "interface-supplier")]
mod create;
mod error;
mod get;
mod misc;
mod reject;

pub use error::Error;

use abort::abort;
#[cfg(feature = "interface-supplier")]
use actix_web::web::post;
use actix_web::web::{get, resource, ServiceConfig};
use proc_macros::capability_statement_resource;
use resources::capability_statement::{Interaction, Type};

use crate::fhir::definitions::{
    OPERATION_TASK_ABORT, OPERATION_TASK_ACCEPT, OPERATION_TASK_ACTIVATE, OPERATION_TASK_CLOSE,
    OPERATION_TASK_CREATE, OPERATION_TASK_REJECT, RESOURCE_PROFILE_TASK,
};

#[cfg(feature = "interface-supplier")]
use accept::accept;
#[cfg(feature = "interface-supplier")]
use activate::activate;
#[cfg(feature = "interface-supplier")]
use close::close;
#[cfg(feature = "interface-supplier")]
use create::create;
use get::{get_all, get_one, get_version};
#[cfg(feature = "interface-supplier")]
use reject::reject;

#[derive(Default)]
pub struct TaskRoutes;

#[capability_statement_resource(
    type = Type::Task,
    profile = RESOURCE_PROFILE_TASK)]
impl TaskRoutes {
    #[cfg(feature = "interface-supplier")]
    #[operation(name="create", definition = OPERATION_TASK_CREATE)]
    #[operation(name="activate", definition = OPERATION_TASK_ACTIVATE)]
    #[operation(name="accept", definition = OPERATION_TASK_ACCEPT)]
    #[operation(name="reject", definition = OPERATION_TASK_REJECT)]
    #[operation(name="close", definition = OPERATION_TASK_CLOSE)]
    fn configure_supplier(&self, cfg: &mut ServiceConfig) {
        cfg.service(resource("/Task/$create").route(post().to(create)));
        cfg.service(resource("/Task/{id:[A-Za-z0-9-]+}/$activate").route(post().to(activate)));
        cfg.service(resource("/Task/{id:[A-Za-z0-9-]+}/$accept").route(post().to(accept)));
        cfg.service(resource("/Task/{id:[A-Za-z0-9-]+}/$reject").route(post().to(reject)));
        cfg.service(resource("/Task/{id:[A-Za-z0-9-]+}/$close").route(post().to(close)));
    }

    #[interaction(Interaction::Read)]
    #[operation(name="abort", definition = OPERATION_TASK_ABORT)]
    fn configure_all(&self, cfg: &mut ServiceConfig) {
        cfg.service(resource("/Task").route(get().to(get_all)));
        cfg.service(resource("/Task/{id:[A-Za-z0-9-]+}").route(get().to(get_one)));
        cfg.service(resource("/Task/{id:[A-Za-z0-9-]+}/$abort").route(post().to(abort)));
        cfg.service(
            resource("/Task/{id:[A-Za-z0-9-]+}/_history/{version:[0-9]+}")
                .route(get().to(get_version)),
        );
    }
}
