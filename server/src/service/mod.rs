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

mod constants;
mod error;
mod header;
mod middleware;
mod misc;
mod routes;
mod state;

use std::fs::read;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::Arc;

use actix_rt::System;
use actix_web::{dev::Server, App, HttpServer};
use arc_swap::ArcSwapOption;
use openssl::{ec::EcKey, x509::X509};
use tokio::task::LocalSet;
use url::Url;

use crate::tsl::Tsl;

pub use error::{Error, RequestError};
use middleware::{HeaderCheck, Logging, Vau};
use misc::{Cms, PukToken};
use routes::configure_routes;
use state::State;

pub struct Service {
    key: PathBuf,
    cert: PathBuf,
    qes_cert: PathBuf,
    puk_token: Url,
    tsl: Arc<ArcSwapOption<Tsl>>,
    addresses: Vec<SocketAddr>,
}

impl Service {
    pub fn new(
        key: PathBuf,
        cert: PathBuf,
        qes_cert: PathBuf,
        puk_token: Url,
        tsl: Arc<ArcSwapOption<Tsl>>,
    ) -> Self {
        Self {
            key,
            cert,
            qes_cert,
            puk_token,
            tsl,
            addresses: Vec::new(),
        }
    }

    pub fn listen<T: ToSocketAddrs>(mut self, addrs: T) -> Result<Self, Error> {
        for addr in addrs.to_socket_addrs()? {
            self.addresses.push(addr);
        }

        Ok(self)
    }

    pub fn run(&self, local: &LocalSet) -> Result<Server, Error> {
        let state = State::default();
        let system = System::run_in_tokio("actix-web", &local);

        local.spawn_local(system);

        let key = read(&self.key)?;
        let key = EcKey::private_key_from_pem(&key).map_err(Error::OpenSslError)?;

        let cert = read(&self.cert)?;
        let cert = X509::from_pem(&cert)?;

        let qes_cert = read(&self.qes_cert)?;
        let qes_cert = X509::from_pem(&qes_cert)?;
        let cms = Cms::new(qes_cert)?;

        let puk_token = PukToken::from_url(self.puk_token.clone())?;

        let tsl = self.tsl.clone();

        let mut server = HttpServer::new(move || {
            App::new()
                .wrap(Vau::new(key.clone(), cert.clone()).unwrap())
                .wrap(HeaderCheck)
                .wrap(Logging)
                .data(state.clone())
                .data(tsl.clone())
                .data(cms.clone())
                .app_data(puk_token.clone())
                .configure(configure_routes)
        });

        for addr in &self.addresses {
            server = server.bind(addr)?;
        }

        let server = server.disable_signals().shutdown_timeout(10).run();

        Ok(server)
    }
}
