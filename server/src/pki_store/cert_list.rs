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

use std::ops::Deref;

use base64::encode;
use log::warn;
use openssl::x509::X509Ref;
use serde::Serialize;
use tokio::spawn;
use tokio::sync::{
    watch::{Receiver, Sender},
    RwLock, RwLockReadGuard,
};

use super::{PkiStore, TimeCheck, Tsl};

pub struct CertList {
    data: RwLock<Data>,
    notify: Sender<()>,
}

#[derive(Serialize)]
pub struct Data {
    add_roots: Vec<String>,
    ca_certs: Vec<String>,
    ee_certs: Vec<String>,
}

impl PkiStore {
    pub(super) fn spawn_cert_list_task(&self, notify: Receiver<()>) {
        let store = self.clone();

        spawn(update_task(store, notify));
    }
}

impl CertList {
    pub fn new(notify: Sender<()>) -> Self {
        let data = Data {
            add_roots: Default::default(),
            ca_certs: Default::default(),
            ee_certs: Default::default(),
        };
        let data = RwLock::new(data);

        Self { data, notify }
    }

    pub async fn data(&self) -> RwLockReadGuard<'_, Data> {
        self.data.read().await
    }

    pub fn update(&self) {
        let _ = self.notify.broadcast(());
    }
}

async fn update_task(store: PkiStore, mut notify: Receiver<()>) {
    while let Some(()) = notify.recv().await {
        let mut data = store.0.cert_list.data.write().await;

        data.add_roots.clear();
        data.ee_certs.clear();
        data.ca_certs.clear();

        let tsl = store.tsl();
        let tsl = tsl.deref().as_deref();

        add_to_list(&mut data.ee_certs, &store.0.enc_cert);
        if let Some(cert) = find_ca_cert(tsl, &store.0.enc_cert) {
            add_to_list(&mut data.ca_certs, cert);
        }

        if let Some(puk_token) = store.0.puk_token.load().as_ref() {
            add_to_list(&mut data.ee_certs, &puk_token.cert);
            if let Some(cert) = find_ca_cert(tsl, &puk_token.cert) {
                add_to_list(&mut data.ca_certs, cert);
            }
        }
    }
}

fn add_to_list(list: &mut Vec<String>, cert: &X509Ref) {
    let cert = match cert.to_der() {
        Ok(cert) => cert,
        Err(err) => {
            warn!("Unable to convert X509 to DER: {}", err);

            return;
        }
    };

    let cert = encode(&cert);

    if !list.contains(&cert) {
        list.push(cert);
    }
}

fn find_ca_cert<'a>(tsl: Option<&'a Tsl>, cert: &X509Ref) -> Option<&'a X509Ref> {
    match tsl?.verify_cert(&cert, TimeCheck::None) {
        Ok(ca_item) => Some(&ca_item.cert),
        Err(err) => {
            let key = Tsl::cert_key(cert.subject_name()).unwrap_or_else(|_| "<unknown>".to_owned());

            warn!("Unable to find issuer certificate for {}: {}", &key, err);

            None
        }
    }
}
