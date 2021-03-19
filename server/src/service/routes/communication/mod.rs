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

mod create;
mod delete;
mod error;
mod get;
mod state;

pub use error::Error;
pub use state::CommunicationRefMut;

use create::create;
use delete::delete_one;
use get::{get_all, get_one};

use actix_web::web::{delete, get, post, resource, ServiceConfig};
use proc_macros::capability_statement_resource;
use resources::capability_statement::{Interaction, SearchParamType, Type};

use crate::fhir::definitions::{
    RESOURCE_PROFILE_COMMUNICATION, RESOURCE_PROFILE_COMMUNICATION_DISPENSE_REQ,
    RESOURCE_PROFILE_COMMUNICATION_INFO_REQ, RESOURCE_PROFILE_COMMUNICATION_REPLY,
    RESOURCE_PROFILE_COMMUNICATION_REPRESENTATIVE,
};

#[derive(Default)]
pub struct CommunicationRoutes;

#[capability_statement_resource(
    type = Type::Communication,
    profile = RESOURCE_PROFILE_COMMUNICATION,
    supported_profiles = [
        RESOURCE_PROFILE_COMMUNICATION_INFO_REQ,
        RESOURCE_PROFILE_COMMUNICATION_REPLY,
        RESOURCE_PROFILE_COMMUNICATION_DISPENSE_REQ,
        RESOURCE_PROFILE_COMMUNICATION_REPRESENTATIVE,
    ])]
impl CommunicationRoutes {
    #[interaction(Interaction::Create)]
    #[interaction(Interaction::Read)]
    #[interaction(Interaction::Delete)]
    #[search_param(name="sent", type=SearchParamType::Date)]
    #[search_param(name="received", type=SearchParamType::Date)]
    #[search_param(name="sender", type=SearchParamType::String)]
    #[search_param(name="recipient", type=SearchParamType::String)]
    fn configure_all(&self, cfg: &mut ServiceConfig) {
        cfg.service(
            resource("/Communication")
                .route(get().to(get_all))
                .route(post().to(create)),
        );
        cfg.service(
            resource("/Communication/{id}")
                .route(get().to(get_one))
                .route(delete().to(delete_one)),
        );
    }
}
