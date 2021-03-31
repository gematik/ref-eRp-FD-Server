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

use std::fs::read;

use openssl::x509::X509;
use url::Url;

use super::{super::PkiStore, Error, PukToken};

pub fn from_file(store: &PkiStore, url: Url) -> Result<(), Error> {
    let filepath = match url.host() {
        Some(host) => format!("{}{}", host, url.path()),
        None => url.path().into(),
    };

    let cert = read(filepath)?;
    let cert = X509::from_pem(&cert)?;
    let public_key = cert.public_key()?;

    let puk_token = PukToken { cert, public_key };

    store.store_puk_token(puk_token);

    Ok(())
}
