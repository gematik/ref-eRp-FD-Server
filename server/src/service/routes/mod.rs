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

pub mod audit_event;
pub mod capabilty_statement;
pub mod cert_list;
pub mod communication;
pub mod device;
pub mod medication_dispense;
pub mod ocsp_list;
pub mod random;
pub mod task;
pub mod tsl;

use actix_web::web::ServiceConfig;
use proc_macros::capability_statement;

use audit_event::AutidEventRoutes;
use capabilty_statement::{create as capability_statement_create, get as capability_statement_get};
use cert_list::configure_routes as cert_list_configure_routes;
use communication::CommunicationRoutes;
use device::DeviceRoutes;
use medication_dispense::MedicationDispenseRoutes;
use ocsp_list::configure_routes as ocsp_list_configure_routes;
use random::configure_routes as random_configure_routes;
use task::TaskRoutes;
use tsl::configure_routes as tsl_configure_routes;

#[capability_statement(
    init = capability_statement_create,
    handler = capability_statement_get)]
pub struct Routes {
    #[resource]
    task: TaskRoutes,

    #[resource]
    communication: CommunicationRoutes,

    #[resource]
    medication_dispense: MedicationDispenseRoutes,

    #[resource]
    audit_event: AutidEventRoutes,

    #[resource]
    device: DeviceRoutes,
}

pub fn configure_routes(cfg: &mut ServiceConfig) {
    ROUTES.configure_routes(cfg);

    tsl_configure_routes(cfg);
    random_configure_routes(cfg);
    cert_list_configure_routes(cfg);
    ocsp_list_configure_routes(cfg);
}

lazy_static! {
    static ref ROUTES: Routes = Routes::default();
}
