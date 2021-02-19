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
use std::ops::Deref;

use actix_rt::System;
use actix_web::{dev::Server, App, HttpServer};
use openssl::{ec::EcKey, pkey::Private, x509::X509};
use tokio::task::LocalSet;

use crate::{
    error::Error,
    state::State,
    tasks::{PukToken, Tsl},
};

pub use error::{
    AsAuditEventOutcome, AsReqErr, AsReqErrResult, RequestError, TypedRequestError,
    TypedRequestResult,
};
use middleware::{ExtractAccessToken, HeaderCheck, Logging, Vau};
use misc::Cms;
use routes::configure_routes;

pub struct EncCert(pub X509);

pub struct Service {
    puk_token: PukToken,
    tsl: Tsl,
    bnetza: Tsl,
    state: State,
    enc_key: EcKey<Private>,
    enc_cert: X509,
    addresses: Vec<SocketAddr>,
}

impl Deref for EncCert {
    type Target = X509;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Service {
    pub fn new(
        puk_token: PukToken,
        tsl: Tsl,
        bnetza: Tsl,
        state: State,
        enc_key: EcKey<Private>,
        enc_cert: X509,
    ) -> Self {
        Self {
            puk_token,
            tsl,
            bnetza,
            state,
            enc_key,
            enc_cert,
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
            puk_token,
            tsl,
            bnetza,
            state,
            enc_key,
            enc_cert,
            addresses,
        } = self;

        let cms = Cms::new(bnetza);
        let system = System::run_in_tokio("actix-web", &local);

        local.spawn_local(system);

        let mut server = HttpServer::new(move || {
            App::new()
                .wrap(ExtractAccessToken)
                .wrap(Vau::new(enc_key.clone(), enc_cert.clone()).unwrap())
                .wrap(HeaderCheck)
                .wrap(Logging)
                .data(state.clone())
                .data(tsl.clone())
                .data(cms.clone())
                .data(puk_token.clone())
                .data(EncCert(enc_cert.clone()))
                .configure(configure_routes)
        });

        for addr in addresses {
            server = server.bind(addr)?;
        }

        let server = server.disable_signals().shutdown_timeout(10).run();

        Ok(server)
    }
}
