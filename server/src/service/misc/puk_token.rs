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

use std::fs::read;

use openssl::pkey::{PKey, Public};
use url::Url;

use crate::service::Error;

#[derive(Clone)]
pub struct PukToken(pub PKey<Public>);

impl PukToken {
    pub fn from_url(url: &Url) -> Result<Self, Error> {
        match url.scheme() {
            "http" | "https" => Self::load_from_web(url),
            "file" => Self::load_from_file(url),
            s => Err(Error::UnsupportedScheme(s.into())),
        }
    }

    fn load_from_web(_url: &Url) -> Result<Self, Error> {
        unimplemented!()
    }

    fn load_from_file(url: &Url) -> Result<Self, Error> {
        let filepath = match url.host() {
            Some(host) => format!("{}{}", host, url.path()),
            None => url.path().into(),
        };

        let ret = read(filepath)?;
        let ret = PKey::public_key_from_pem(&ret)?;
        let ret = Self(ret);

        Ok(ret)
    }
}
