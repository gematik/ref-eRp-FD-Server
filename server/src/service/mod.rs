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

mod error;
mod header;
mod idp_client;
mod middleware;
mod routes;
mod state;

use std::fs::read;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;

use actix_rt::System;
use actix_web::{App, HttpServer};
use openssl::{ec::EcKey, x509::X509};
use tokio::task::LocalSet;

pub use error::Error;
use middleware::{CharsetUtf8, Logging, Vau};
use routes::configure_routes;
use state::State;

pub struct Service {
    vau_key: PathBuf,
    vau_cert: PathBuf,
    addresses: Vec<SocketAddr>,
}

impl Service {
    pub fn new(vau_key: PathBuf, vau_cert: PathBuf) -> Self {
        Self {
            vau_key,
            vau_cert,
            addresses: Vec::new(),
        }
    }

    pub fn listen<T: ToSocketAddrs>(mut self, addrs: T) -> Result<Self, Error> {
        for addr in addrs.to_socket_addrs()? {
            self.addresses.push(addr);
        }

        Ok(self)
    }

    pub async fn run(self) -> Result<Self, Error> {
        let state = State::default();
        let local = LocalSet::new();
        let _system = System::run_in_tokio("actix-web", &local);

        let vau_key = read(&self.vau_key)?;
        let vau_key = EcKey::private_key_from_pem(&vau_key).map_err(Error::OpenSslError)?;

        let vau_cert = read(&self.vau_cert)?;
        let vau_cert = X509::from_pem(&vau_cert)?;

        let mut server = HttpServer::new(move || {
            App::new()
                .wrap(Vau::new(vau_key.clone(), vau_cert.clone()).unwrap())
                .wrap(CharsetUtf8)
                .wrap(Logging)
                .data(state.clone())
                .configure(configure_routes)
        });

        for addr in &self.addresses {
            server = server.bind(addr)?;
        }

        server.run().await?;

        Ok(self)
    }
}
