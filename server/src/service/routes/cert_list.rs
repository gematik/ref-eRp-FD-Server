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

use actix_web::{
    error::Error as ActixError,
    web::{get, resource, Data, ServiceConfig},
    HttpResponse,
};
use base64::encode;
use log::warn;
use openssl::x509::X509;
use serde::Serialize;

use crate::{
    service::EncCert,
    tasks::{PukToken, Tsl},
};

#[derive(Default, Serialize)]
struct CertList {
    add_roots: Vec<String>,
    ca_certs: Vec<String>,
    ee_certs: Vec<String>,
}

pub fn configure_routes(cfg: &mut ServiceConfig) {
    cfg.service(resource("/CertList").route(get().to(get_cert_list)));
}

async fn get_cert_list(
    enc_cert: Data<EncCert>,
    puk_token: Data<PukToken>,
    tsl: Data<Tsl>,
) -> Result<HttpResponse, ActixError> {
    let mut cert_list = CertList::default();

    add_to_list(&mut cert_list.ee_certs, &enc_cert);
    if let Some(puk_token) = puk_token.load().as_ref() {
        add_to_list(&mut cert_list.ee_certs, &puk_token.cert);
    }

    if let Some(tsl) = tsl.load().as_ref() {
        for certs in tsl.certs.entries().values() {
            for cert in certs {
                add_to_list(&mut cert_list.ca_certs, &cert);
            }
        }
    }

    Ok(HttpResponse::Ok().json(cert_list))
}

fn add_to_list(list: &mut Vec<String>, cert: &X509) {
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
