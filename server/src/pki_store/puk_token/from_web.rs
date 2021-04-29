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
use std::env::var;
use std::time::Duration;

use log::{error, info, warn};
use miscellaneous::jwt::{verify, VerifyMode};
use openssl::x509::X509;
use reqwest::RequestBuilder;
use serde::Deserialize;
use tokio::{spawn, time::delay_for};
use url::Url;

use super::{
    super::{misc::Client, PkiStore, TimeCheck},
    Error, PukToken,
};

pub fn from_web(store: &PkiStore, url: Url) -> Result<(), Error> {
    spawn(update_task(store.clone(), url));

    Ok(())
}

#[derive(Deserialize)]
struct DiscoveryDocument {
    uri_puk_idp_sig: String,
}

#[derive(Deserialize)]
struct Jwks {
    x5c: Vec<String>,
}

trait RequestBuilderEx {
    fn add_idp_auth_header(self) -> Self;
}

impl RequestBuilderEx for RequestBuilder {
    fn add_idp_auth_header(self) -> Self {
        if let Ok(api_key) = var("IDP_API_KEY") {
            self.header("X-Authorization", api_key)
        } else if let Ok(api_key) = var("idp_api_key") {
            self.header("X-Authorization", api_key)
        } else {
            self
        }
    }
}

async fn update_task(store: PkiStore, url: Url) {
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

    // wait until we have fetched the TSL
    delay_for(Duration::from_secs(5)).await;

    loop {
        let mut retry_timeout = 10u64;

        macro_rules! ok {
            ($e:expr, $msg:tt $(, $args:expr)*) => {
                match $e {
                    Ok(value) => value,
                    Err(err) => {
                        warn!($msg, $($args,)* err);

                        delay_for(Duration::from_secs(retry_timeout)).await;
                        retry_timeout = min(15 * 60, (retry_timeout as f64 * 1.2) as u64);

                        continue;
                    }
                }
            };
        }

        let next = loop {
            let tsl = store.tsl();
            let tsl = match &*tsl {
                Some(tsl) => tsl,
                None => {
                    warn!("Unable to fetch PUK_TOKEN: TSL was not fetched yet");

                    delay_for(Duration::from_secs(retry_timeout)).await;
                    retry_timeout = min(15 * 60, (retry_timeout as f64 * 1.2) as u64);

                    continue;
                }
            };

            let (dd_cert, discovery_document) = ok!(
                fetch_discovery_document(&client, url.clone()).await,
                "Unable to fetch discovery document: {}"
            );

            ok!(
                tsl.verify_cert(&dd_cert, TimeCheck::Now),
                "Unable to verify discovery document: {}"
            );

            let uri_token = ok!(
                Url::parse(&discovery_document.uri_puk_idp_sig),
                "Invalid uri_puk_idp_sig ({}): {}",
                &discovery_document.uri_puk_idp_sig
            );

            let cert = ok!(
                fetch_cert(&client, uri_token).await,
                "Unable to fetch PUK_TOKEN_KEY: {}"
            );

            ok!(
                tsl.verify_cert(&cert, TimeCheck::Now),
                "Unable to verify PUK_TOKEN_KEY: {}"
            );

            let public_key = ok!(
                cert.public_key(),
                "Unable to extract public key from PUK_TOKEN_KEY: {}"
            );

            break PukToken { cert, public_key };
        };

        store.store_puk_token(next);

        info!("PUK_TOKEN updated");

        delay_for(Duration::from_secs(12 * 60 * 60)).await; // 12 h
    }
}

async fn fetch_discovery_document(
    client: &Client,
    url: Url,
) -> Result<(X509, DiscoveryDocument), Error> {
    let res = client.get(url)?.add_idp_auth_header().send().await?;

    if res.status() != 200 {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();

        return Err(Error::InvalidResponse(status, text));
    }

    let mut cert = None;
    let raw = res.text().await?;
    let parsed = verify(&raw, VerifyMode::CertOut(&mut cert))?;

    let cert = cert.unwrap();

    Ok((cert, parsed))
}

async fn fetch_cert(client: &Client, url: Url) -> Result<X509, Error> {
    let res = client.get(url)?.add_idp_auth_header().send().await?;
    if res.status() != 200 {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();

        return Err(Error::InvalidResponse(status, text));
    }

    let jwks = res.json::<Jwks>().await?;
    let cert = jwks.x5c.first().ok_or(Error::UnknownSignerCert)?;
    let cert = format!(
        "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----",
        cert
    );
    let cert = X509::from_pem(cert.as_bytes())?;

    Ok(cert)
}
