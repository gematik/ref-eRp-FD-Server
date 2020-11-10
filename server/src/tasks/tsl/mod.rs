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

mod extract;
mod update;

use std::sync::Arc;

use arc_swap::{ArcSwapOption, Guard};
use openssl::x509::X509;
use tokio::task::spawn;
use url::Url;

pub use update::update;

#[derive(Clone)]
pub struct Tsl(Arc<ArcSwapOption<Inner>>);

pub struct Inner {
    pub xml: String,
    pub sha2: String,
    pub certs: Vec<X509>,
}

impl Tsl {
    pub fn from_url(url: Option<Url>) -> Self {
        let tsl = Self(Arc::new(ArcSwapOption::from(None)));

        if let Some(url) = url {
            spawn(update(url, tsl.clone()));
        }

        tsl
    }

    pub fn load(&self) -> Guard<'static, Option<Arc<Inner>>> {
        self.0.load()
    }
}
