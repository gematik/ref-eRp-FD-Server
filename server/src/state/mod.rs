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

mod e_prescriptions;
mod erx_receipts;
mod patient_receipts;
mod persist;
mod timeouts;

use std::sync::Arc;

use openssl::{
    pkey::{PKey, Private},
    x509::X509,
};
use tokio::{
    sync::{Mutex, MutexGuard},
    time::{delay_for, Duration},
};

use crate::service::{AuditEvents, Communications, MedicationDispenses, Tasks};

pub use e_prescriptions::EPrescriptions;
pub use erx_receipts::ErxReceipts;
pub use patient_receipts::PatientReceipts;
pub use timeouts::{ResourceId, Timeouts};

#[derive(Clone)]
pub struct State {
    inner: Arc<Mutex<Inner>>,
    config: Arc<Config>,
}

pub struct Inner {
    pub(super) max_communications: usize,

    pub(super) tasks: Tasks,
    pub(super) e_prescriptions: EPrescriptions,
    pub(super) patient_receipts: PatientReceipts,
    pub(super) erx_receipts: ErxReceipts,
    pub(super) communications: Communications,
    pub(super) medication_dispenses: MedicationDispenses,
    pub(super) audit_events: AuditEvents,
    pub(super) timeouts: Timeouts,
}

struct Config {
    throttling: usize,
    throttling_header: String,
}

impl State {
    pub fn new(
        sig_key: PKey<Private>,
        sig_cert: X509,
        max_communications: usize,
        throttling: usize,
        throttling_header: String,
    ) -> Self {
        let inner = Inner {
            max_communications,

            tasks: Default::default(),
            e_prescriptions: Default::default(),
            patient_receipts: PatientReceipts::new(sig_key.clone(), sig_cert.clone()),
            erx_receipts: ErxReceipts::new(sig_key, sig_cert),
            communications: Default::default(),
            medication_dispenses: Default::default(),
            audit_events: Default::default(),
            timeouts: Default::default(),
        };
        let inner = Arc::new(Mutex::new(inner));

        let config = Config {
            throttling,
            throttling_header,
        };
        let config = Arc::new(config);

        let ret = Self { inner, config };
        ret.spawn_timeout_task();

        ret
    }

    pub async fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().await
    }

    pub async fn throttle(&self) -> Option<String> {
        if self.config.throttling > 0 {
            delay_for(Duration::from_millis(self.config.throttling as u64)).await;

            Some(self.config.throttling_header.clone())
        } else {
            None
        }
    }
}
