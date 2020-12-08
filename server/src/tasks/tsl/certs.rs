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

use std::collections::HashMap;

use openssl::x509::{X509NameRef, X509};

use super::error::Error;

#[derive(Default)]
pub struct Certs {
    entries: HashMap<String, Vec<X509>>,
}

impl Certs {
    pub fn entries(&self) -> &HashMap<String, Vec<X509>> {
        &self.entries
    }

    pub fn add_cert(&mut self, cert: X509) -> Result<(), Error> {
        let key = Self::key(cert.subject_name())?;

        self.entries.entry(key).or_default().push(cert);

        Ok(())
    }

    pub fn key(name: &X509NameRef) -> Result<String, Error> {
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
}
