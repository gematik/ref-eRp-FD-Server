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

use std::env::var;

use glob::Pattern;
use log::warn;
use reqwest::{Client as HttpClient, Error, IntoUrl, Proxy, RequestBuilder};

pub struct Client {
    http_proxy: HttpClient,
    http_no_proxy: HttpClient,
    no_proxy: Vec<Pattern>,
}

impl Client {
    pub fn new() -> Result<Self, Error> {
        let no_proxy = if let Ok(no_proxy) = var("no_proxy") {
            no_proxy
                .split(',')
                .map(Pattern::new)
                .filter_map(|pattern| match pattern {
                    Ok(pattern) => Some(pattern),
                    Err(err) => {
                        warn!("Invalid pattern in NO_PROXY environment variable: {}", err);

                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        let mut http = HttpClient::builder();

        if let Ok(http_proxy) = var("http_proxy") {
            http = http.proxy(Proxy::http(&http_proxy)?);
        }

        if let Ok(https_proxy) = var("https_proxy") {
            http = http.proxy(Proxy::https(&https_proxy)?);
        }

        let http_proxy = http.build()?;
        let http_no_proxy = HttpClient::builder().no_proxy().build()?;

        Ok(Self {
            http_proxy,
            http_no_proxy,
            no_proxy,
        })
    }

    pub fn get<U: IntoUrl>(&self, url: U) -> Result<RequestBuilder, Error> {
        let url = url.into_url()?;
        let domain = url.domain();
        let no_proxy = match domain {
            Some(domain) => self.no_proxy.iter().any(|p| p.matches(domain)),
            None => false,
        };

        let http = if no_proxy {
            &self.http_no_proxy
        } else {
            &self.http_proxy
        };

        Ok(http.get(url))
    }
}
