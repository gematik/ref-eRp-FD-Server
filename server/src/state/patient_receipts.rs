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

use std::collections::hash_map::{Entry, HashMap};

use openssl::{
    pkey::{PKey, Private},
    x509::X509,
};
use resources::{primitives::Id, KbvBundle, SignatureType};

use crate::fhir::security::{Signed, SignedError};

pub struct PatientReceipts {
    sig_key: PKey<Private>,
    sig_cert: X509,

    by_id: HashMap<Id, Signed<KbvBundle>>,
}

impl PatientReceipts {
    pub fn new(sig_key: PKey<Private>, sig_cert: X509) -> Self {
        Self {
            sig_key,
            sig_cert,
            by_id: Default::default(),
        }
    }

    pub fn get_by_id(&self, id: &Id) -> Option<&Signed<KbvBundle>> {
        self.by_id.get(id)
    }

    pub fn insert_signed(&mut self, signed: Signed<KbvBundle>) {
        let id = signed.id.clone();
        match self.by_id.entry(id) {
            Entry::Occupied(e) => {
                panic!("Patient receipt with this ID ({}) already exists!", e.key());
            }
            Entry::Vacant(e) => {
                e.insert(signed);
            }
        };
    }

    pub fn insert_kbv_bundle(&mut self, bundle: KbvBundle) -> Result<(), SignedError> {
        let mut patient_receipt = Signed::new(bundle);
        patient_receipt.sign_json(
            SignatureType::AuthorsSignature,
            "Device/software".into(),
            &self.sig_key,
            &self.sig_cert,
        )?;

        self.insert_signed(patient_receipt);

        Ok(())
    }

    pub fn remove_by_id(&mut self, id: &Id) {
        self.by_id.remove(id).expect("Patient Receipt not found!");
    }

    pub fn iter(&self) -> impl Iterator<Item = &Signed<KbvBundle>> {
        self.by_id.values()
    }
}
