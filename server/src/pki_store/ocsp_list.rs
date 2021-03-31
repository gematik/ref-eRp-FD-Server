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

use std::collections::hash_map::{Entry, HashMap};

use base64::encode;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use log::{error, info, warn};
use openssl::{hash::MessageDigest, ocsp::OcspResponse, x509::X509Ref};
use tokio::{
    select, spawn,
    sync::{
        watch::{Receiver, Sender},
        RwLock, RwLockReadGuard,
    },
    time::{delay_for, Duration as TokioDuration},
};

use super::{misc::Client, Error, PkiStore, Tsl};

pub struct OcspList {
    items: RwLock<HashMap<String, Item>>,
    data: RwLock<Vec<String>>,
    notify: Sender<()>,
}

pub struct Item {
    used: bool,
    response: OcspResponse,
    timeout: DateTime<Utc>,
}

impl PkiStore {
    pub(super) fn spawn_ocsp_list_task(&self, notify: Receiver<()>) {
        let store = self.clone();

        spawn(update_task(store, notify));
    }
}

impl OcspList {
    pub fn new(notify: Sender<()>) -> Self {
        let data = Default::default();
        let data = RwLock::new(data);

        let items = Default::default();
        let items = RwLock::new(items);

        Self {
            items,
            data,
            notify,
        }
    }

    pub async fn data(&self) -> RwLockReadGuard<'_, Vec<String>> {
        self.data.read().await
    }

    pub fn update(&self) {
        let _ = self.notify.broadcast(());
    }
}

impl Item {
    async fn new(client: &Client, tsl: &Tsl, cert: &X509Ref) -> Result<Self, Error> {
        let response = client.get_ocsp_response(tsl, cert).await?;

        Ok(Self {
            used: true,
            response,
            timeout: Utc::now() + ChronoDuration::seconds(RENEWAL_INTERVAL.as_secs() as i64),
        })
    }

    async fn update(&mut self, client: &Client, tsl: &Tsl, cert: &X509Ref) -> Result<(), Error> {
        let now = Utc::now();

        self.used = true;

        if self.timeout < now {
            self.response = client.get_ocsp_response(tsl, cert).await?;
            self.timeout = now + ChronoDuration::seconds(RENEWAL_INTERVAL.as_secs() as i64);
        }

        Ok(())
    }
}

async fn update_task(store: PkiStore, mut notify: Receiver<()>) {
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
        // Wait for timeout or notification
        select! {
            _ = delay_for(UPDATE_INTERVAL) => (),
            notify = notify.recv() => {
                if notify.is_none() {
                    return;
                }
            }
        }

        // get TSL
        let tsl = store.tsl();
        let tsl = match &*tsl {
            Some(tsl) => tsl,
            None => {
                warn!("Unable to update OCSP list: TSL was not fetched yet!");
                continue;
            }
        };

        // mark all items as unused
        let mut items = store.ocsp_list().items.write().await;
        for item in items.values_mut() {
            item.used = false;
        }

        // create list of certs to get OCSP response for
        let mut certs = Vec::new();
        certs.push(store.enc_cert().to_owned());
        if let Some(cert) = store.puk_token().as_ref().map(|p| &p.cert) {
            certs.push(cert.to_owned());
        }

        // update items
        for cert in certs {
            macro_rules! ok {
                ($e:expr, $msg:tt $(, $args:expr)*) => {
                    match $e {
                        Ok(value) => value,
                        Err(err) => {
                            let key = Tsl::cert_key(cert.subject_name())
                                .unwrap_or_else(|_| "<unknown>".to_owned());

                            warn!($msg, key, $($args,)* err);

                            continue;
                        }
                    }
                };
            }

            let hash = ok!(
                cert.digest(MessageDigest::sha256()),
                "Unable to get hash for certificate {0}: {1}"
            );
            let hash = encode(&hash);

            match items.entry(hash) {
                Entry::Occupied(mut e) => {
                    ok!(
                        e.get_mut().update(&client, &tsl, &cert).await,
                        "Unable to receive OCSP response for {0}: {1}"
                    );
                }
                Entry::Vacant(e) => {
                    let item = ok!(
                        Item::new(&client, &tsl, &cert).await,
                        "Unable to receive OCSP response for {0}: {1}"
                    );

                    e.insert(item);
                }
            }
        }

        // remove unused items
        items.retain(|_, v| v.used);

        // convert OCSP responses to base64 DER
        let mut data = store.ocsp_list().data.write().await;
        data.clear();
        for item in items.values() {
            match item.response.to_der() {
                Ok(res) => data.push(encode(&res)),
                Err(err) => warn!("Unable to convert OCSP response to base64 DER: {0}", err),
            }
        }

        info!("OCSP List updated");
    }
}

const UPDATE_INTERVAL: TokioDuration = TokioDuration::from_secs(30 * 60); // 30min
const RENEWAL_INTERVAL: TokioDuration = TokioDuration::from_secs(6 * 60 * 60); // 6h
