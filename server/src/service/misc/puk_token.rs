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

use std::cmp::min;
use std::fs::read;
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwapOption;
use log::{error, info};
use openssl::{
    error::ErrorStack as OpenSslError,
    pkey::{PKey, Public},
    x509::X509,
};
use reqwest::{Client, Error as ReqwestError, StatusCode};
use serde_json::Value;
use thiserror::Error;
use tokio::{spawn, time::delay_for};
use url::Url;

use crate::{misc::create_reqwest_client, service::Error};

#[derive(Clone)]
pub struct PukToken(Arc<ArcSwapOption<PKey<Public>>>);

#[derive(Debug, Error)]
enum FetchError {
    #[error("Reqwest Error: {0}")]
    ReqwestError(ReqwestError),

    #[error("Fetch PUK_TOKEN failed: {0}")]
    FetchPukTokenFailed(StatusCode),

    #[error("Open SSL Error: {0}")]
    OpenSslError(OpenSslError),

    #[error("Invalid Format of PUK_TOKEN!")]
    InvalidFormat,
}

impl PukToken {
    pub fn from_url(url: Url) -> Result<Self, Error> {
        match url.scheme() {
            "http" | "https" => Self::load_from_web(url),
            "file" => Self::load_from_file(url),
            s => Err(Error::UnsupportedScheme(s.into())),
        }
    }

    pub fn public_key(&self) -> Option<PKey<Public>> {
        self.0.load_full().map(|key| key.as_ref().clone())
    }

    fn load_from_web(url: Url) -> Result<Self, Error> {
        let ret = Arc::new(ArcSwapOption::from(None));

        let pub_key = ret.clone();

        spawn(async move {
            let client = match create_reqwest_client() {
                Ok(client) => client,
                Err(err) => {
                    error!("Unable to create http client for TSL updates: {}", err);

                    return;
                }
            };

            let mut retry_timeout = 30u64;

            loop {
                match fetch_pub_key(&client, url.clone()).await {
                    Ok(value) => {
                        info!("Successfully fetched PUK_TOKEN from IDP endpoint.");

                        pub_key.store(Some(Arc::new(value)));

                        break;
                    }
                    Err(err) => error!("Unable to fetch PUK_TOKEN: {}", err),
                }

                delay_for(Duration::from_secs(retry_timeout)).await;

                retry_timeout = min(15 * 60, (retry_timeout as f64 * 1.2) as u64);
            }
        });

        Ok(Self(ret))
    }

    fn load_from_file(url: Url) -> Result<Self, Error> {
        let filepath = match url.host() {
            Some(host) => format!("{}{}", host, url.path()),
            None => url.path().into(),
        };

        let ret = read(filepath)?;
        let ret = PKey::public_key_from_pem(&ret)?;
        let ret = Arc::new(ArcSwapOption::from(Some(Arc::new(ret))));

        Ok(Self(ret))
    }
}

impl From<ReqwestError> for FetchError {
    fn from(err: ReqwestError) -> Self {
        Self::ReqwestError(err)
    }
}

impl From<OpenSslError> for FetchError {
    fn from(err: OpenSslError) -> Self {
        Self::OpenSslError(err)
    }
}

async fn fetch_pub_key(client: &Client, url: Url) -> Result<PKey<Public>, FetchError> {
    let res = client.get(url).send().await?;
    if res.status() != 200 {
        return Err(FetchError::FetchPukTokenFailed(res.status()));
    }

    let cert = res.json::<Value>().await?;
    let cert = match cert.pointer("/keys/0/x5c/0") {
        Some(cert) => cert,
        None => return Err(FetchError::InvalidFormat),
    };
    let cert = cert.as_str().ok_or_else(|| FetchError::InvalidFormat)?;
    let cert = format!(
        "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----",
        cert
    );
    let cert = X509::from_pem(cert.as_bytes())?;

    let pub_key = cert.public_key()?;

    Ok(pub_key)
}
