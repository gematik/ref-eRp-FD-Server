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

mod certs;
mod error;
mod extract;
mod update;

use std::sync::Arc;

use arc_swap::{ArcSwapOption, Guard};
use chrono::{DateTime, Duration, Utc};
use openssl::{
    asn1::{Asn1Time, Asn1TimeRef},
    cms::{CMSOptions, CmsContentInfo},
    stack::Stack,
    x509::{store::X509Store, X509Ref, X509VerifyResult},
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
    pub fn from_url<F>(url: Url, prepare: F, load_hash: bool) -> Self
    where
        F: Fn(&mut TrustServiceStatusList) + Send + Sync + 'static,
    {
        let tsl = Self(Arc::new(ArcSwapOption::from(None)));

        spawn(update(url, tsl.clone(), prepare, load_hash));

        tsl
    }

    pub fn load(&self) -> Guard<'static, Option<Arc<Inner>>> {
        self.0.load()
    }

    pub fn verify_cms(&self, pem: &str) -> Result<(Vec<u8>, DateTime<Utc>), Error> {
        /* check and prepare the pem data */
        let cms = if pem.starts_with("-----BEGIN PKCS7-----") {
            CmsContentInfo::from_pem(pem.as_bytes())?
        } else {
            let pem = format!("-----BEGIN PKCS7-----\n{}\n-----END PKCS7-----", pem.trim());

            CmsContentInfo::from_pem(pem.as_bytes())?
        };

        /* get the actual TSL data */
        let inner = self.load();
        let inner = match &*inner {
            Some(inner) => inner,
            None => return Err(Error::UnknownIssuerCert),
        };

        /* verify the cms container
         * (this will also set the 'signers' of the signers info) */
        let certs = Stack::new()?;
        let mut data = Vec::new();
        cms.verify(
            &certs,
            &inner.store,
            None,
            Some(&mut data),
            CMSOptions::NOVERIFY,
        )?;

        /* get verified signers */
        let mut signer_count = 0;
        let mut signing_time = Utc::now();
        let signer_infos = cms.signer_infos()?;
        for signer_info in signer_infos {
            // 'signer' is only set if the CMS container
            // was verified with that certificate before!
            let signer_cert = match signer_info.signer() {
                Ok(signer) => signer,
                Err(_) => continue,
            };

            inner.verify(&signer_cert)?;
            signer_count += 1;

            let st = signer_info
                .signing_time()?
                .ok_or(Error::UnknownSigningTime)?;
            let st = asn1_to_chrono(st);
            if signing_time > st {
                signing_time = st;
            }
        }

        /* dobule check that at least one signer certificate was used */
        if signer_count == 0 {
            return Err(Error::UnknownIssuerCert);
        }

        Ok((data, signing_time))
    }
}

impl Inner {
    pub fn verify(&self, cert: &X509Ref) -> Result<(), Error> {
        let key = Certs::key(cert.issuer_name())?;
        let ca_certs = self
            .certs
            .entries()
            .get(&key)
            .ok_or(Error::UnknownIssuerCert)?;

        check_cert_time(cert)?;

        for ca_cert in ca_certs {
            if ca_cert.issued(cert) == X509VerifyResult::OK {
                if check_cert_time(ca_cert).is_err() {
                    continue;
                }

                let pub_key = ca_cert.public_key()?;
                if cert.verify(&pub_key)? {
                    return Ok(());
                }
            }
        }

        Err(Error::UnknownIssuerCert)
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

fn asn1_to_chrono(time: &Asn1TimeRef) -> DateTime<Utc> {
    let now = Utc::now();
    let asn1_now = Asn1Time::from_unix(now.timestamp()).unwrap();
    let diff = time.diff(&asn1_now).unwrap();

    now - Duration::days(diff.days as _)
        - Duration::seconds(diff.secs as _)
        - Duration::nanoseconds(now.timestamp_subsec_nanos() as _)
}

fn check_cert_time(cert: &X509Ref) -> Result<(), Error> {
    let now = Utc::now();
    let not_after = asn1_to_chrono(cert.not_after());
    let not_before = asn1_to_chrono(cert.not_before());

    if now < not_before {
        return Err(Error::CertNotValidYet);
    } else if now > not_after {
        return Err(Error::CertNotValidAnymore);
    }

    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::fs::{read, read_to_string};

    use openssl::x509::store::X509StoreBuilder;

    use super::extract::extract;

    #[test]
    fn test_cms_verify_gematik() {
        verify_cms(
            "./examples/cms.pem",
            "./examples/kbv_bundle.xml",
            DateTime::parse_from_rfc3339("2021-01-15T11:24:43Z")
                .unwrap()
                .into(),
        );
    }

    #[test]
    fn test_cms_verify_github_issue_12() {
        verify_cms(
            "./examples/cms_github_issue_12.pem",
            "./examples/kbv_bundle_github_issue_12.xml",
            DateTime::parse_from_rfc3339("2021-01-06T14:28:34Z")
                .unwrap()
                .into(),
        );
    }

    fn verify_cms(cms: &str, content: &str, signing_time: DateTime<Utc>) {
        let expected_data = read(content).unwrap();
        let expected_signing_time = signing_time;

        let cms = read_to_string(cms).unwrap();
        let tsl = create_tsl();

        let (actual_data, actual_signing_time) = tsl.verify_cms(&cms).unwrap();

        assert_eq!(actual_data, expected_data);
        assert_eq!(actual_signing_time, expected_signing_time);
    }

    fn create_tsl() -> Tsl {
        let bnetza = read_to_string("./examples/Pseudo-BNetzA-VL-seq24.xml").unwrap();
        let certs = extract(&bnetza, &prepare_no_op).unwrap();
        let store = X509StoreBuilder::new().unwrap().build();

        let inner = Inner {
            xml: Default::default(),
            sha2: None,
            certs,
            store,
        };

        Tsl(Arc::new(ArcSwapOption::from_pointee(inner)))
    }
}
