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

mod error;
mod from_file;
mod from_web;

pub use error::Error;

use std::sync::Arc;

use arc_swap::{ArcSwapOption, Guard};
use openssl::pkey::{PKey, Public};
use thiserror::Error;
use url::Url;

use from_file::from_file;
use from_web::from_web;

#[derive(Clone)]
pub struct PukToken(Arc<ArcSwapOption<Inner>>);

pub struct Inner {
    pub public_key: PKey<Public>,
}

impl PukToken {
    pub fn from_url(url: Url) -> Result<Self, Error> {
        match url.scheme() {
            "http" | "https" => from_web(url),
            "file" => from_file(url),
            s => Err(Error::UnsupportedScheme(s.into())),
        }
    }

    pub fn load(&self) -> Guard<'static, Option<Arc<Inner>>> {
        self.0.load()
    }
}
