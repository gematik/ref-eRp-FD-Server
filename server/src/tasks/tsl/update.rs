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

use std::cmp::min;
use std::fs::read_to_string;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use log::{error, info, warn};
use openssl::x509::store::X509StoreBuilder;
use regex::{Regex, RegexBuilder};
use tokio::time::delay_for;
use url::Url;

use super::{
    super::misc::Client,
    error::Error,
    extract::{extract, TrustServiceStatusList},
    Inner, Tsl,
};

pub async fn update<F>(url: Url, tsl: Tsl, prepare: F, load_hash: bool)
where
    F: Fn(&mut TrustServiceStatusList) + Send + Sync,
{
    let client = match Client::new() {
        Ok(client) => client,
        Err(err) => {
            error!("Unable to create http client for TSL updates: {}", err);

            return;
        }
    };

    loop {
        let mut retry_timeout = 30u64;

        let next = loop {
            let xml = match fetch_data(&client, &url).await {
                Ok(xml) => xml,
                Err(err) => {
                    warn!("Unable to fetch TSL ({}): {}", &url, err);

                    delay_for(Duration::from_secs(retry_timeout)).await;
                    retry_timeout = min(15 * 60, (retry_timeout as f64 * 1.2) as u64);

                    continue;
                }
            };

            let sha_url = build_sha_url(&url);
            let sha2 = if load_hash {
                match fetch_data(&client, &sha_url).await {
                    Ok(sha2) => Some(sha2),
                    Err(err) => {
                        warn!("Unable to fetch SHA ({}): {}", &sha_url, err);

                        delay_for(Duration::from_secs(retry_timeout)).await;
                        retry_timeout = min(15 * 60, (retry_timeout as f64 * 1.2) as u64);

                        continue;
                    }
                }
            } else {
                None
            };

            let certs = match extract(&xml, &prepare) {
                Ok(certs) => certs,
                Err(err) => {
                    warn!("Unable to extract certificats from TSL ({}): {}", &url, err);

                    delay_for(Duration::from_secs(retry_timeout)).await;
                    retry_timeout = min(15 * 60, (retry_timeout as f64 * 1.2) as u64);

                    continue;
                }
            };

            let store = match X509StoreBuilder::new() {
                Ok(store) => store.build(),
                Err(err) => {
                    error!("Unable to create X509 store: {}", err);

                    return;
                }
            };
            let timestamp = Utc::now();

            break Inner {
                xml,
                sha2,
                certs,
                store,
                timestamp,
            };
        };

        tsl.0.store(Some(Arc::new(next)));

        info!("TSL updated: {}", url);

        delay_for(Duration::from_secs(12 * 60 * 60)).await; // 12 h
    }
}

async fn fetch_data(client: &Client, url: &Url) -> Result<String, Error> {
    let body = if url.scheme() == "file" {
        let filename = match url.host() {
            Some(host) => format!("{}{}", host, url.path()),
            None => url.path().into(),
        };

        read_to_string(filename)?
    } else {
        let res = client.get(url.clone())?.send().await?;

        let status = res.status();
        if status != 200 {
            return Err(Error::InvalidResponse(status.to_string()));
        }

        res.text().await?
    };

    Ok(body)
}

#[allow(clippy::trivial_regex)]
fn build_sha_url(url: &Url) -> Url {
    lazy_static! {
        static ref RX: Regex = RegexBuilder::new(r#"\.xml$"#)
            .case_insensitive(true)
            .build()
            .unwrap();
    }

    let path = if RX.is_match(url.path()) {
        RX.replace(url.path(), ".sha2").into_owned()
    } else {
        format!("{}.sha2", url.path())
    };

    let mut url = url.clone();
    url.set_path(&path);

    url
}
