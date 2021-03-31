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

mod cert_list;
mod error;
mod misc;
mod ocsp_list;
mod ocsp_vau;
mod puk_token;
mod tsl;

use std::sync::Arc;

use arc_swap::{ArcSwapOption, Guard as ArcSwapGuard};
use chrono::{DateTime, Utc};
use openssl::{
    cms::{CMSOptions, CmsContentInfo},
    ec::EcKey,
    ocsp::OcspResponse,
    pkey::Private,
    stack::Stack,
    x509::{
        store::{X509Store, X509StoreBuilder},
        X509Ref, X509,
    },
};
use tokio::sync::watch::channel;
use url::Url;

pub use error::Error;
pub use puk_token::PukToken;
pub use tsl::Tsl;

use cert_list::CertList;
use misc::asn1_to_chrono;
use ocsp_list::OcspList;

#[derive(Clone)]
pub struct PkiStore(Arc<Inner>);

struct Inner {
    enc_key: EcKey<Private>,
    enc_cert: X509,
    tsl: ArcSwapOption<Tsl>,
    bnetza: ArcSwapOption<Tsl>,
    puk_token: ArcSwapOption<PukToken>,
    cert_list: CertList,
    ocsp_list: OcspList,
    ocsp_vau: ArcSwapOption<OcspResponse>,
    dummy_store: X509Store,
}

impl PkiStore {
    pub fn new(
        enc_key: EcKey<Private>,
        enc_cert: X509,
        tsl: Url,
        bnetza: Url,
        puk_token: Url,
    ) -> Result<Self, Error> {
        let (cert_list_sender, cert_list_receiver) = channel(());
        let (ocsp_list_sender, ocsp_list_receiver) = channel(());

        let cert_list = CertList::new(cert_list_sender);
        let ocsp_list = OcspList::new(ocsp_list_sender);
        let dummy_store = X509StoreBuilder::new()?.build();

        let inner = Inner {
            enc_key,
            enc_cert,
            tsl: ArcSwapOption::from(None),
            bnetza: ArcSwapOption::from(None),
            puk_token: ArcSwapOption::from(None),
            cert_list,
            ocsp_list,
            ocsp_vau: ArcSwapOption::from(None),
            dummy_store,
        };

        let store = Self(Arc::new(inner));

        store.spawn_tsl_task(tsl);
        store.spawn_bnetza_task(bnetza);
        store.spawn_puk_token_task(puk_token)?;
        store.spawn_cert_list_task(cert_list_receiver);
        store.spawn_ocsp_list_task(ocsp_list_receiver);
        store.spawn_ocsp_vau_task();

        Ok(store)
    }

    pub fn enc_key(&self) -> &EcKey<Private> {
        &self.0.enc_key
    }

    pub fn enc_cert(&self) -> &X509Ref {
        &self.0.enc_cert
    }

    pub fn puk_token(&self) -> ArcSwapGuard<Option<Arc<PukToken>>> {
        self.0.puk_token.load()
    }

    pub fn tsl(&self) -> ArcSwapGuard<Option<Arc<Tsl>>> {
        self.0.tsl.load()
    }

    pub fn bnetza(&self) -> ArcSwapGuard<Option<Arc<Tsl>>> {
        self.0.tsl.load()
    }

    pub fn ocsp_vau(&self) -> ArcSwapGuard<Option<Arc<OcspResponse>>> {
        self.0.ocsp_vau.load()
    }

    pub fn cert_list(&self) -> &CertList {
        &self.0.cert_list
    }

    pub fn ocsp_list(&self) -> &OcspList {
        &self.0.ocsp_list
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
        let bnetza = self.0.bnetza.load();
        let bnetza = match &*bnetza {
            Some(bnetza) => bnetza,
            None => return Err(Error::UnknownIssuerCert),
        };

        /* verify the cms container
         * (this will also set the 'signers' of the signers info) */
        let certs = Stack::new()?;
        let mut data = Vec::new();
        cms.verify(
            &certs,
            &self.0.dummy_store,
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

            bnetza.verify_cert(&signer_cert, true)?;
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

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::fs::{read, read_to_string};

    use libxml::Doc;
    use openssl::{ec::EcGroup, nid::Nid};
    use xmlsec::Node;

    use super::tsl::{extract, prepare_no_op};

    #[test]
    fn test_cms_verify_gematik() {
        verify_cms(
            "./examples/cms.pem",
            "./examples/kbv_bundle.xml",
            DateTime::parse_from_rfc3339("2021-02-17T20:42:48Z")
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

    #[test]
    fn test_tsl_verify() {
        let doc = Doc::from_file("./examples/Pseudo-BNetzA-VL.xml").unwrap();
        let node_root = doc.root().unwrap();
        let node_signature = node_root
            .search(&mut |n| n.name().unwrap() == "Signature")
            .unwrap();
        let node_signed_props = node_signature
            .search(&mut |n| n.name().unwrap() == "SignedProperties")
            .unwrap();
        let verified_nodes = node_root.verify().unwrap();

        assert!(verified_nodes.contains(node_root, None));
        assert!(!verified_nodes.contains(node_signature, None));
        assert!(verified_nodes.contains(node_signed_props, None));
    }

    fn verify_cms(cms: &str, content: &str, signing_time: DateTime<Utc>) {
        let expected_data = read(content).unwrap();
        let expected_signing_time = signing_time;

        let cms = read_to_string(cms).unwrap();
        let store = create_store();
        load_bnetza(&store);

        let (actual_data, actual_signing_time) = store.verify_cms(&cms).unwrap();

        assert_eq!(actual_data, expected_data);
        assert_eq!(actual_signing_time, expected_signing_time);
    }

    fn create_store() -> PkiStore {
        let (cert_list_sender, _) = channel(());
        let (ocsp_list_sender, _) = channel(());

        let group = EcGroup::from_curve_name(Nid::from_raw(927)).unwrap();
        let inner = Inner {
            enc_key: EcKey::generate(&group).unwrap(),
            enc_cert: X509::builder().unwrap().build(),
            tsl: ArcSwapOption::from(None),
            bnetza: ArcSwapOption::from(None),
            puk_token: ArcSwapOption::from(None),
            cert_list: CertList::new(cert_list_sender),
            ocsp_list: OcspList::new(ocsp_list_sender),
            ocsp_vau: ArcSwapOption::from(None),
            dummy_store: X509StoreBuilder::new().unwrap().build(),
        };

        PkiStore(Arc::new(inner))
    }

    fn load_bnetza(pki_store: &PkiStore) {
        let bnetza = read_to_string("./examples/Pseudo-BNetzA-VL-seq24.xml").unwrap();
        let items = extract(&bnetza, &prepare_no_op).unwrap();

        let mut store = X509StoreBuilder::new().unwrap();
        for items in items.values() {
            for item in items {
                store.add_cert(item.cert.clone()).unwrap();
            }
        }
        let store = store.build();

        let bnetza = Tsl {
            xml: Default::default(),
            sha2: None,
            items,
            store,
        };

        pki_store.0.bnetza.store(Some(Arc::new(bnetza)));
    }
}
