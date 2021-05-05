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

mod constants;
mod error;
mod header;
mod middleware;
mod misc;
mod routes;

use std::net::{SocketAddr, ToSocketAddrs};

use actix_rt::System;
use actix_web::{dev::Server, App, HttpServer};
use tokio::task::LocalSet;

use crate::{error::Error, pki_store::PkiStore, state::State};

pub use error::{
    AsAuditEventOutcome, IntoReqErr, IntoReqErrResult, RequestError, TypedRequestError,
    TypedRequestResult,
};
use middleware::{HeaderCheck, Logging, ReqResLogging, Vau};
use routes::configure_routes;
pub use routes::{
    audit_event::{AuditEventBuilder, AuditEvents, Loggable, LoggedIter, LoggedRef},
    communication::Communications,
    medication_dispense::MedicationDispenses,
    task::{TaskMeta, Tasks},
};

pub struct Service {
    state: State,
    pki_store: PkiStore,
    addresses: Vec<SocketAddr>,
}

impl Service {
    pub fn new(state: State, pki_store: PkiStore) -> Self {
        Self {
            state,
            pki_store,
            addresses: Vec::new(),
        }
    }

    pub fn listen<T: ToSocketAddrs>(mut self, addrs: T) -> Result<Self, Error> {
        for addr in addrs.to_socket_addrs()? {
            self.addresses.push(addr);
        }

        Ok(self)
    }

    pub fn run(self, local: &LocalSet) -> Result<Server, Error> {
        let Self {
            state,
            pki_store,
            addresses,
        } = self;

        let system = System::run_in_tokio("actix-web", &local);

        local.spawn_local(system);

        let mut server = HttpServer::new(move || {
            App::new()
                .wrap(Vau)
                .wrap(HeaderCheck)
                .wrap(Logging)
                .wrap(ReqResLogging)
                .data(state.clone())
                .data(pki_store.clone())
                .configure(configure_routes)
        });

        for addr in addresses {
            server = server.bind(addr)?;
        }

        let server = server.disable_signals().shutdown_timeout(10).run();

        Ok(server)
    }
}
