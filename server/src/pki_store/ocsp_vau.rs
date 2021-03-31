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

use log::{error, info, warn};
use openssl::ocsp::OcspResponse;
use tokio::{
    spawn,
    time::{delay_for, Duration as TokioDuration},
};

use super::{misc::Client, PkiStore};

impl PkiStore {
    pub(super) fn spawn_ocsp_vau_task(&self) {
        let store = self.clone();

        spawn(update_task(store));
    }

    fn store_ocsp_vau(&self, value: OcspResponse) {
        self.0.ocsp_vau.store(Some(Arc::new(value)));
    }
}

async fn update_task(store: PkiStore) {
    let client = match Client::new() {
        Ok(client) => client,
        Err(err) => {
            error!(
                "Unable to create http client for OCSP list updates: {}",
                err
            );

            return;
        }
    };

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

        loop {
            // get TSL
            let tsl = store.tsl();
            let tsl = ok!(
                tsl.as_ref().ok_or("TSL was not fetched yet"),
                "Unable to update OCSP for enc_cert: {}!"
            );

            // get ENC Cert
            let enc_cert = store.enc_cert();

            // Get OCSP response
            let res = ok!(
                client.get_ocsp_response(&tsl, &enc_cert).await,
                "Unable to update OCSP for enc_cert: {}!"
            );

            store.store_ocsp_vau(res);

            info!("OCSP for enc_cert updated");

            break;
        }

        delay_for(UPDATE_INTERVAL).await;
    }
}

const UPDATE_INTERVAL: TokioDuration = TokioDuration::from_secs(60 * 60); // 1h
