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

mod from_file;
mod from_web;

use std::sync::Arc;

use openssl::{
    pkey::{PKey, Public},
    x509::X509,
};
use url::Url;

use super::{Error, PkiStore};

use from_file::from_file;
use from_web::from_web;

#[derive(Clone)]
pub struct PukToken {
    pub cert: X509,
    pub public_key: PKey<Public>,
}

impl PkiStore {
    pub(super) fn spawn_puk_token_task(&self, url: Url) -> Result<(), Error> {
        match url.scheme() {
            "http" | "https" => from_web(self, url),
            "file" => from_file(self, url),
            _ => Err(Error::InvalidUrl(url.to_string())),
        }
    }

    fn store_puk_token(&self, value: PukToken) {
        self.0.puk_token.store(Some(Arc::new(value)));
        self.cert_list().update();
        self.ocsp_list().update();
    }
}
