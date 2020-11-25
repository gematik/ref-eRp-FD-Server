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

use actix_rt::System;
use actix_web::{dev::Server, App, HttpServer};
use openssl::{ec::EcKey, pkey::PKey, x509::X509};
use tokio::task::LocalSet;

use crate::tasks::{PukToken, Tsl};

pub use error::{Error, RequestError};
use middleware::{ExtractAccessToken, HeaderCheck, Logging, Vau};
use misc::{Cms, SigCert, SigKey};
use routes::configure_routes;
use state::State;

pub struct Service {
    enc_key: PathBuf,
    enc_cert: PathBuf,
    sig_key: PathBuf,
    sig_cert: PathBuf,
    puk_token: PukToken,
    tsl: Tsl,
    bnetza: Tsl,
    addresses: Vec<SocketAddr>,
}

impl Service {
    pub fn new(
        enc_key: PathBuf,
        enc_cert: PathBuf,
        sig_key: PathBuf,
        sig_cert: PathBuf,
        puk_token: PukToken,
        tsl: Tsl,
        bnetza: Tsl,
    ) -> Self {
        Self {
            enc_key,
            enc_cert,
            sig_key,
            sig_cert,
            puk_token,
            tsl,
            bnetza,
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

        let enc_key = read(&self.enc_key)?;
        let enc_key = EcKey::private_key_from_pem(&enc_key).map_err(Error::OpenSslError)?;

        let enc_cert = read(&self.enc_cert)?;
        let enc_cert = X509::from_pem(&enc_cert)?;

        let sig_key = read(&self.sig_key)?;
        let sig_key = EcKey::private_key_from_pem(&sig_key).map_err(Error::OpenSslError)?;
        let sig_key = PKey::from_ec_key(sig_key)?;

        let sig_cert = read(&self.sig_cert)?;
        let sig_cert = X509::from_pem(&sig_cert)?;

        let puk_token = self.puk_token.clone();
        let tsl = self.tsl.clone();
        let cms = Cms::new(self.bnetza.clone());

        let mut server = HttpServer::new(move || {
            App::new()
                .wrap(ExtractAccessToken)
                .wrap(Vau::new(enc_key.clone(), enc_cert.clone()).unwrap())
                .wrap(HeaderCheck)
                .wrap(Logging)
                .data(state.clone())
                .data(tsl.clone())
                .data(cms.clone())
                .data(SigKey(sig_key.clone()))
                .data(SigCert(sig_cert.clone()))
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
