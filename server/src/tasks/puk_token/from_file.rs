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
use std::sync::Arc;

use arc_swap::ArcSwapOption;
use chrono::Utc;
use openssl::x509::X509;
use url::Url;

use super::{Error, Inner, PukToken};

pub fn from_file(url: Url) -> Result<PukToken, Error> {
    let filepath = match url.host() {
        Some(host) => format!("{}{}", host, url.path()),
        None => url.path().into(),
    };

    let cert = read(filepath)?;
    let cert = X509::from_pem(&cert)?;
    let public_key = cert.public_key()?;
    let timestamp = Utc::now();

    let inner = Inner {
        cert,
        public_key,
        timestamp,
    };

    let puk_token = PukToken(Arc::new(ArcSwapOption::from(Some(Arc::new(inner)))));

    Ok(puk_token)
}
