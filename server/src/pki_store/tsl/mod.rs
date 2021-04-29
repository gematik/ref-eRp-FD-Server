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

mod extract;
mod update;

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use openssl::{
    stack::Stack,
    x509::{store::X509Store, X509NameRef, X509Ref, X509VerifyResult, X509},
};
use tokio::spawn;
use url::Url;

pub use extract::extract;
pub use update::update;

use extract::{ServiceInformation, TrustServiceStatusList};

use super::{misc::check_cert_time, Error, PkiStore};

pub struct Tsl {
    pub xml: String,
    pub sha2: Option<String>,
    pub items: HashMap<String, Vec<Item>>,
    pub store: X509Store,
    pub stack: Stack<X509>,
}

pub struct Item {
    pub cert: X509,
    pub supply_points: Vec<String>,
}

pub enum TimeCheck {
    None,
    Now,
    Time(DateTime<Utc>),
}

impl PkiStore {
    pub(super) fn spawn_tsl_task(&self, url: Url) {
        let store = self.clone();

        spawn(update(url, true, prepare_tsl, move |tsl| {
            store.0.tsl.store(Some(Arc::new(tsl)));
            store.cert_list().update();
            store.ocsp_list().update();
        }));
    }

    pub(super) fn spawn_bnetza_task(&self, url: Url) {
        let store = self.clone();

        spawn(update(url, false, prepare_no_op, move |tsl| {
            store.0.bnetza.store(Some(Arc::new(tsl)));
        }));
    }
}

impl Tsl {
    pub fn cert_key(name: &X509NameRef) -> Result<String, Error> {
        let mut key = String::new();

        for part in name.entries() {
            let nid = part.object().nid();
            let value = part.data().as_utf8()?;

            key = if key.is_empty() {
                format!("{}={}", nid.short_name()?, value)
            } else {
                format!("{};{}={}", key, nid.short_name()?, value)
            };
        }

        if key.is_empty() {
            Err(Error::EmptyCertKey)
        } else {
            Ok(key)
        }
    }

    pub fn verify_cert<'a>(
        &'a self,
        cert: &X509Ref,
        time_check: TimeCheck,
    ) -> Result<&'a Item, Error> {
        let key = Self::cert_key(cert.issuer_name())?;
        let ca_items = self.items.get(&key).ok_or(Error::UnknownIssuerCert)?;

        match &time_check {
            TimeCheck::None => (),
            TimeCheck::Now => check_cert_time(cert, None)?,
            TimeCheck::Time(t) => check_cert_time(cert, Some(t))?,
        }

        for ca_item in ca_items {
            if ca_item.cert.issued(cert) == X509VerifyResult::OK {
                match &time_check {
                    TimeCheck::Now if check_cert_time(&ca_item.cert, None).is_err() => continue,
                    TimeCheck::Time(t) if check_cert_time(&ca_item.cert, Some(t)).is_err() => {
                        continue
                    }
                    _ => (),
                }

                let pub_key = ca_item.cert.public_key()?;
                if cert.verify(&pub_key)? {
                    return Ok(&ca_item);
                }
            }
        }

        Err(Error::UnknownIssuerCert)
    }
}

pub fn prepare_tsl(tsl: &mut TrustServiceStatusList) -> Result<(), Error> {
    let now = Utc::now();

    for provider in &mut tsl.provider_list.provider {
        provider.services.service.retain(|service| {
            let info = &service.infos;
            if !is_pkc_service(&info) && !is_ocsp_service(&info) {
                return false;
            }

            let start_time = match DateTime::parse_from_rfc3339(&info.starting_time) {
                Ok(start_time) => start_time,
                Err(_) => return false,
            };

            if start_time > now {
                return false;
            }

            true
        });

        for service in &provider.services.service {
            if service
                .infos
                .supply_points
                .as_ref()
                .ok_or(Error::MissingServiceSupplyPoints)?
                .supply_point
                .is_empty()
            {
                return Err(Error::MissingServiceSupplyPoints);
            }
        }
    }

    Ok(())
}

pub fn prepare_no_op(_: &mut TrustServiceStatusList) -> Result<(), Error> {
    Ok(())
}

fn is_pkc_service(info: &ServiceInformation) -> bool {
    const IDENT: &str = "http://uri.etsi.org/TrstSvc/Svctype/CA/PKC";
    const STATUS: &str = "http://uri.etsi.org/TrstSvc/Svcstatus/inaccord";
    const EXT_OID: &str = "1.2.276.0.76.4.203";
    const EXT_VALUE: &str = "oid_fd_sig";

    if info.ident != IDENT {
        return false;
    }

    if info.status != STATUS {
        return false;
    }

    let mut has_ext = false;
    if let Some(extensions) = &info.extensions {
        for ex in &extensions.extension {
            if ex.oid.as_deref() == Some(EXT_OID) && ex.value.as_deref() == Some(EXT_VALUE) {
                has_ext = true;

                break;
            }
        }
    }

    if !has_ext {
        return false;
    }

    true
}

fn is_ocsp_service(info: &ServiceInformation) -> bool {
    const IDENT: &str = "http://uri.etsi.org/TrstSvc/Svctype/Certstatus/OCSP";

    if info.ident != IDENT {
        return false;
    }

    true
}
