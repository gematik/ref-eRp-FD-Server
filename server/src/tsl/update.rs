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
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwapOption;
use log::{error, info, warn};
use reqwest::{Client, Error as ReqwestError};
use thiserror::Error;
use tokio::time::delay_for;
use url::{ParseError, Url};

use crate::misc::create_reqwest_client;

use super::{extract::extract, Tsl};

pub async fn update(url: Url, tsl: Arc<ArcSwapOption<Tsl>>) {
    let client = match create_reqwest_client() {
        Ok(client) => client,
        Err(err) => {
            error!("Unable to create http client for TSL updates: {}", err);

            return;
        }
    };

    loop {
        let mut retry_timeout = 30u64;

        let next = loop {
            let xml = match fetch_data(&client, &url, "TSL.xml").await {
                Ok(xml) => xml,
                Err(err) => {
                    warn!("Unable to fetch TSL.xml: {}", err);

                    delay_for(Duration::from_secs(retry_timeout)).await;
                    retry_timeout = min(15 * 60, (retry_timeout as f64 * 1.2) as u64);

                    continue;
                }
            };

            let sha2 = match fetch_data(&client, &url, "TSL.sha2").await {
                Ok(xml) => xml,
                Err(err) => {
                    warn!("Unable to fetch TSL.sha2: {}", err);

                    delay_for(Duration::from_secs(retry_timeout)).await;
                    retry_timeout = min(15 * 60, (retry_timeout as f64 * 1.2) as u64);

                    continue;
                }
            };

            let certs = match extract(&xml) {
                Ok(certs) => certs,
                Err(err) => {
                    warn!("Unable to extract certificats from TSL: {}", err);

                    delay_for(Duration::from_secs(retry_timeout)).await;
                    retry_timeout = min(15 * 60, (retry_timeout as f64 * 1.2) as u64);

                    continue;
                }
            };

            break Tsl { xml, sha2, certs };
        };

        tsl.store(Some(Arc::new(next)));

        info!("TSL updated");

        delay_for(Duration::from_secs(12 * 60 * 60)).await; // 12 h
    }
}

#[derive(Debug, Error)]
#[allow(clippy::enum_variant_names)]
enum Error {
    #[error("Url Parse Error: {0}")]
    ParseError(ParseError),

    #[error("Reqwest Error: {0}")]
    ReqwestError(ReqwestError),

    #[error("Invalid Response (url={0})")]
    InvalidResponse(String),
}

impl From<ParseError> for Error {
    fn from(err: ParseError) -> Error {
        Error::ParseError(err)
    }
}

impl From<ReqwestError> for Error {
    fn from(err: ReqwestError) -> Error {
        Error::ReqwestError(err)
    }
}

async fn fetch_data(client: &Client, url: &Url, file: &str) -> Result<String, Error> {
    let url = url.join(file)?;
    let res = client.get(url.clone()).send().await?;

    if res.status() != 200 {
        return Err(Error::InvalidResponse(url.to_string()));
    }

    let body = res.text().await?;

    Ok(body)
}
