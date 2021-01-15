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
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwapOption;
use log::{error, info, warn};
use miscellaneous::jwt::verify;
use openssl::{
    pkey::{PKey, Public},
    x509::X509,
};
use serde::Deserialize;
use tokio::{spawn, time::delay_for};
use url::Url;

use super::{super::misc::Client, Error, Inner, PukToken};

pub fn from_web(url: Url) -> Result<PukToken, Error> {
    let puk_token = PukToken(Arc::new(ArcSwapOption::from(None)));

    spawn(update_task(url, puk_token.clone()));

    Ok(puk_token)
}

#[derive(Deserialize)]
struct DiscoveryDocument {
    puk_uri_token: String,
}

#[derive(Deserialize)]
struct Jwks {
    keys: Vec<JwksKey>,
}

#[derive(Deserialize)]
struct JwksKey {
    x5c: Vec<String>,
}

async fn update_task(url: Url, pub_token: PukToken) {
    let client = match Client::new() {
        Ok(client) => client,
        Err(err) => {
            error!(
                "Unable to create http client for PUK_TOKEN updates: {}",
                err
            );

            return;
        }
    };

    loop {
        let mut retry_timeout = 30u64;

        let next = loop {
            let discovery_document = match fetch_discovery_document(&client, url.clone()).await {
                Ok(discovery_document) => discovery_document,
                Err(err) => {
                    warn!("Unable to fetch discovery document: {}", err);

                    delay_for(Duration::from_secs(retry_timeout)).await;
                    retry_timeout = min(15 * 60, (retry_timeout as f64 * 1.2) as u64);

                    continue;
                }
            };

            let url = match Url::parse(&discovery_document.puk_uri_token) {
                Ok(url) => url,
                Err(err) => {
                    warn!(
                        "Invalid puk_uri_token ({}): {}",
                        &discovery_document.puk_uri_token, err
                    );

                    delay_for(Duration::from_secs(retry_timeout)).await;
                    retry_timeout = min(15 * 60, (retry_timeout as f64 * 1.2) as u64);

                    continue;
                }
            };

            let public_key = match fetch_pub_key(&client, url).await {
                Ok(public_key) => public_key,
                Err(err) => {
                    warn!("Unable to fetch PUK_TOKEN: {}", err);

                    delay_for(Duration::from_secs(retry_timeout)).await;
                    retry_timeout = min(15 * 60, (retry_timeout as f64 * 1.2) as u64);

                    continue;
                }
            };

            break Inner { public_key };
        };

        pub_token.0.store(Some(Arc::new(next)));

        info!("PUK_TOKEN updated");

        delay_for(Duration::from_secs(12 * 60 * 60)).await; // 12 h
    }
}

async fn fetch_discovery_document(client: &Client, url: Url) -> Result<DiscoveryDocument, Error> {
    let res = client.get(url)?.send().await?;
    if res.status() != 200 {
        return Err(Error::FetchDiscoveryDocumentFailed(res.status()));
    }

    let discovery_document = res.text().await?;
    let discovery_document = verify(&discovery_document, None)?;

    Ok(discovery_document)
}

async fn fetch_pub_key(client: &Client, url: Url) -> Result<PKey<Public>, Error> {
    let res = client.get(url)?.send().await?;
    if res.status() != 200 {
        return Err(Error::FetchPukTokenFailed(res.status()));
    }

    let jwks = res.json::<Jwks>().await?;
    let cert = jwks
        .keys
        .first()
        .ok_or(Error::MissingCert)?
        .x5c
        .first()
        .ok_or(Error::MissingCert)?;
    let cert = format!(
        "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----",
        cert
    );
    let cert = X509::from_pem(cert.as_bytes())?;

    let pub_key = cert.public_key()?;

    Ok(pub_key)
}
