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
use miscellaneous::jwt::{extract, verify};
use openssl::x509::X509;
use serde::Deserialize;
use tokio::{spawn, time::delay_for};
use url::Url;

use super::{
    super::{misc::Client, Tsl},
    Error, Inner, PukToken,
};

pub fn from_web(tsl: Tsl, url: Url) -> Result<PukToken, Error> {
    let puk_token = PukToken(Arc::new(ArcSwapOption::from(None)));

    spawn(update_task(tsl, url, puk_token.clone()));

    Ok(puk_token)
}

#[derive(Deserialize)]
struct DiscoveryDocument {
    puk_uri_token: String,
    puk_uri_disc: String,
}

#[derive(Deserialize)]
struct Jwks {
    x5c: Vec<String>,
}

async fn update_task(tsl: Tsl, url: Url, pub_token: PukToken) {
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
            let tsl = tsl.load();
            let tsl = match &*tsl {
                Some(tsl) => tsl,
                None => {
                    warn!("Unable to fetch PUK_TOKEN: TSL was not fetched yet");

                    delay_for(Duration::from_secs(retry_timeout)).await;
                    retry_timeout = min(15 * 60, (retry_timeout as f64 * 1.2) as u64);

                    continue;
                }
            };

            let (raw, discovery_document) = ok!(
                fetch_discovery_document(&client, url.clone()).await,
                "Unable to fetch discovery document: {}"
            );

            let uri_disc = ok!(
                Url::parse(&discovery_document.puk_uri_disc),
                "Invalid puk_uri_disc ({}): {}",
                &discovery_document.puk_uri_disc
            );

            let cert_disc = ok!(
                fetch_cert(&client, uri_disc).await,
                "Unable to fetch PUK_DISC_KEY: {}"
            );

            ok!(tsl.verify(&cert_disc), "Unable to verify PUK_DISC_KEY: {}");

            let key_disc = ok!(
                cert_disc.public_key(),
                "Unable to extract public key from PUK_DISC_KEY: {}"
            );

            let discovery_document = ok!(
                verify::<DiscoveryDocument>(&raw, Some(key_disc)),
                "Unable to verify discovery document: {}"
            );

            let uri_token = ok!(
                Url::parse(&discovery_document.puk_uri_token),
                "Invalid puk_uri_token ({}): {}",
                &discovery_document.puk_uri_token
            );

            let cert_token = ok!(
                fetch_cert(&client, uri_token).await,
                "Unable to fetch PUK_TOKEN_KEY: {}"
            );

            ok!(
                tsl.verify(&cert_token),
                "Unable to verify PUK_TOKEN_KEY: {}"
            );

            let cert = cert_disc;
            let public_key = ok!(
                cert.public_key(),
                "Unable to extract public key from PUK_TOKEN_KEY: {}"
            );

            break Inner { cert, public_key };
        };

        pub_token.0.store(Some(Arc::new(next)));

        info!("PUK_TOKEN updated");

        delay_for(Duration::from_secs(12 * 60 * 60)).await; // 12 h
    }
}

async fn fetch_discovery_document(
    client: &Client,
    url: Url,
) -> Result<(String, DiscoveryDocument), Error> {
    let res = client.get(url)?.send().await?;

    if res.status() != 200 {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();

        return Err(Error::FetchFailed(status, text));
    }

    let raw = res.text().await?;
    let parsed = extract(&raw)?;

    Ok((raw, parsed))
}

async fn fetch_cert(client: &Client, url: Url) -> Result<X509, Error> {
    let res = client.get(url)?.send().await?;
    if res.status() != 200 {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();

        return Err(Error::FetchFailed(status, text));
    }

    let jwks = res.json::<Jwks>().await?;
    let cert = jwks.x5c.first().ok_or(Error::MissingCert)?;
    let cert = format!(
        "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----",
        cert
    );
    let cert = X509::from_pem(cert.as_bytes())?;

    Ok(cert)
}
