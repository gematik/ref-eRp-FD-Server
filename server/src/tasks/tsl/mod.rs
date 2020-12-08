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

mod certs;
mod error;
mod extract;
mod update;

use std::sync::Arc;

use arc_swap::{ArcSwapOption, Guard};
use chrono::{DateTime, Utc};
use openssl::{
    pkcs7::{Pkcs7, Pkcs7Flags},
    stack::Stack,
    x509::{store::X509Store, X509VerifyResult},
};
use tokio::task::spawn;
use url::Url;

pub use error::Error;
pub use update::update;

use certs::Certs;
use extract::TrustServiceStatusList;

#[derive(Clone)]
pub struct Tsl(Arc<ArcSwapOption<Inner>>);

pub struct Inner {
    pub xml: String,
    pub sha2: Option<String>,
    pub certs: Certs,

    store: X509Store,
}

impl Tsl {
    pub fn from_url<F>(url: Option<Url>, prepare: F, load_hash: bool) -> Self
    where
        F: Fn(&mut TrustServiceStatusList) + Send + Sync + 'static,
    {
        let tsl = Self(Arc::new(ArcSwapOption::from(None)));

        if let Some(url) = url {
            spawn(update(url, tsl.clone(), prepare, load_hash));
        }

        tsl
    }

    pub fn load(&self) -> Guard<'static, Option<Arc<Inner>>> {
        self.0.load()
    }

    pub fn verify_pkcs7(&self, pkcs7: Pkcs7) -> Result<Vec<u8>, Error> {
        let flags = Pkcs7Flags::empty();
        let certs = Stack::new()?;
        let signer_certs = pkcs7.signer_certs(&certs, flags)?;

        let inner = self.load();
        let inner = match &*inner {
            Some(inner) => inner,
            None => return Err(Error::UnknownIssuerCert),
        };

        let mut is_valid = false;
        'signer_loop: for signer_cert in signer_certs {
            let key = Certs::key(signer_cert.issuer_name())?;

            if let Some(ca_certs) = inner.certs.entries().get(&key) {
                for ca_cert in ca_certs {
                    if ca_cert.issued(signer_cert) == X509VerifyResult::OK {
                        let pub_key = ca_cert.public_key()?;

                        if signer_cert.verify(&pub_key)? {
                            is_valid = true;

                            break 'signer_loop;
                        }
                    }
                }
            }
        }

        if !is_valid {
            return Err(Error::UnknownIssuerCert);
        }

        let mut data = Vec::new();
        pkcs7.verify(
            &certs,
            &inner.store,
            None,
            Some(&mut data),
            Pkcs7Flags::NOVERIFY,
        )?;

        Ok(data)
    }
}

pub fn prepare_tsl(tsl: &mut TrustServiceStatusList) {
    const IDENT: &str = "http://uri.etsi.org/TrstSvc/Svctype/CA/PKC";
    const STATUS: &str = "http://uri.etsi.org/TrstSvc/Svcstatus/inaccord";
    const EXT_OID: &str = "1.2.276.0.76.4.203";
    const EXT_VALUE: &str = "oid_fd_sig";

    let now = Utc::now();

    for provider in &mut tsl.provider_list.provider {
        provider.services.service.retain(|service| {
            let info = &service.infos;
            if info.ident != IDENT {
                return false;
            }

            if info.status != STATUS {
                return false;
            }

            let start_time = match DateTime::parse_from_rfc3339(&info.starting_time) {
                Ok(start_time) => start_time,
                Err(_) => return false,
            };

            if start_time > now {
                return false;
            }

            let mut has_ext = false;
            if let Some(extensions) = &info.extensions {
                for ex in &extensions.extension {
                    if ex.oid.as_deref() == Some(EXT_OID) && ex.value.as_deref() == Some(EXT_VALUE)
                    {
                        has_ext = true;

                        break;
                    }
                }
            }

            if !has_ext {
                return false;
            }

            true
        })
    }
}

pub fn prepare_no_op(_: &mut TrustServiceStatusList) {}
