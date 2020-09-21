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

mod capabilty_statement;
mod communication;
mod task;

use actix_web::web::ServiceConfig;
use proc_macros::capability_statement;

use capabilty_statement::{create as capability_statement_create, get as capability_statement_get};
use communication::CommunicationRoutes;
use task::TaskRoutes;

#[capability_statement(
    init = capability_statement_create,
    handler = capability_statement_get)]
pub struct Routes {
    #[resource]
    task: TaskRoutes,

    #[resource]
    communication: CommunicationRoutes,
}

pub fn configure_routes(cfg: &mut ServiceConfig) {
    ROUTES.configure_routes(cfg);
}

lazy_static! {
    static ref ROUTES: Routes = Routes::default();
}
