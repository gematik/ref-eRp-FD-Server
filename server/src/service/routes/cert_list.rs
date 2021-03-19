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

use std::sync::Arc;

use actix_web::{
    error::Error as ActixError,
    web::{get, resource, Data, ServiceConfig},
    HttpResponse,
};
use base64::encode;
use chrono::{naive::NaiveDateTime, DateTime, Utc};
use log::warn;
use openssl::x509::X509Ref;
use serde::Serialize;
use tokio::sync::Mutex;

use crate::{
    service::EncCert,
    tasks::{
        tsl::{Certs, Inner as TslInner, Tsl},
        PukToken,
    },
};

#[derive(Clone)]
pub struct CertList(Arc<Mutex<Inner>>);

#[derive(Serialize)]
struct Inner {
    #[serde(skip)]
    timestamp: DateTime<Utc>,
    add_roots: Vec<String>,
    ca_certs: Vec<String>,
    ee_certs: Vec<String>,
}

impl Default for CertList {
    fn default() -> Self {
        let inner = Inner {
            timestamp: DateTime::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc),
            add_roots: Default::default(),
            ca_certs: Default::default(),
            ee_certs: Default::default(),
        };

        Self(Arc::new(Mutex::new(inner)))
    }
}

impl Inner {
    fn clear(&mut self) {
        self.add_roots.clear();
        self.ca_certs.clear();
        self.ee_certs.clear();
    }
}

pub fn configure_routes(cfg: &mut ServiceConfig) {
    cfg.service(resource("/CertList").route(get().to(get_cert_list)));
}

async fn get_cert_list(
    enc_cert: Data<EncCert>,
    puk_token: Data<PukToken>,
    tsl: Data<Tsl>,
    cert_list: Data<CertList>,
) -> Result<HttpResponse, ActixError> {
    let mut cert_list = cert_list.0.lock().await;

    let mut need_update = false;
    if let Some(tsl) = tsl.load().as_deref() {
        need_update |= tsl.timestamp > cert_list.timestamp;
    }
    if let Some(puk_token) = puk_token.load().as_ref() {
        need_update |= puk_token.timestamp > cert_list.timestamp;
    }

    if need_update {
        cert_list.clear();

        let tsl = tsl.load();
        let tsl = &*tsl;
        let tsl = tsl.as_deref();

        add_to_list(&mut cert_list.ee_certs, &enc_cert);
        if let Some(cert) = find_ca_cert(tsl, &enc_cert) {
            add_to_list(&mut cert_list.ca_certs, cert);
        }

        if let Some(puk_token) = puk_token.load().as_ref() {
            add_to_list(&mut cert_list.ee_certs, &puk_token.cert);
            if let Some(cert) = find_ca_cert(tsl, &puk_token.cert) {
                add_to_list(&mut cert_list.ca_certs, cert);
            }
        }

        cert_list.timestamp = Utc::now();
    }

    Ok(HttpResponse::Ok().json2(&*cert_list))
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

    list.push(cert);
}

fn find_ca_cert<'a>(tsl: Option<&'a TslInner>, cert: &X509Ref) -> Option<&'a X509Ref> {
    match tsl?.verify(&cert) {
        Ok(ca_cert) => Some(&ca_cert),
        Err(err) => {
            let key = Certs::key(cert.subject_name()).unwrap_or_else(|_| "<unknown>".to_owned());

            warn!("Unable to find issuer certificate for {}: {}", &key, &err);

            None
        }
    }
}
