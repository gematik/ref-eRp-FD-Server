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

mod history;
mod persist;

use std::collections::hash_map::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use openssl::{
    pkey::{PKey, Private},
    x509::X509,
};
use resources::{
    communication::Communication, misc::Kvnr, primitives::Id, AuditEvent, ErxBundle, KbvBinary,
    KbvBundle, MedicationDispense, Task,
};
use tokio::sync::{Mutex, MutexGuard};

pub use history::{Error as HistoryError, History, Version};

use crate::fhir::security::Signed;

#[derive(Clone)]
pub struct State(Arc<Mutex<Inner>>);

pub struct Inner {
    pub(super) sig_key: PKey<Private>,
    pub(super) sig_cert: X509,

    pub(super) tasks: HashMap<Id, TaskMeta>,
    pub(super) e_prescriptions: HashMap<Id, KbvBinary>,
    pub(super) patient_receipts: HashMap<Id, Signed<KbvBundle>>,
    pub(super) erx_receipts: HashMap<Id, Signed<ErxBundle>>,
    pub(super) communications: HashMap<Id, Communication>,
    pub(super) medication_dispenses: HashMap<Id, MedicationDispense>,
    pub(super) audit_events: HashMap<Kvnr, Vec<AuditEvent>>,
}

pub struct TaskMeta {
    pub history: History<Task>,
    pub accept_timestamp: Option<DateTime<Utc>>,
}

impl State {
    pub fn new(sig_key: PKey<Private>, sig_cert: X509) -> Self {
        let inner = Inner {
            sig_key,
            sig_cert,

            tasks: Default::default(),
            e_prescriptions: Default::default(),
            patient_receipts: Default::default(),
            erx_receipts: Default::default(),
            communications: Default::default(),
            medication_dispenses: Default::default(),
            audit_events: Default::default(),
        };

        Self(Arc::new(Mutex::new(inner)))
    }

    pub async fn lock(&self) -> MutexGuard<'_, Inner> {
        self.0.lock().await
    }
}

impl From<Task> for TaskMeta {
    fn from(task: Task) -> Self {
        Self {
            history: History::new(task),
            accept_timestamp: None,
        }
    }
}
