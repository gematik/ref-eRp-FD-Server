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

use std::io::{Read, Write};
use std::ops::Deref;

use chrono::{serde::ts_nanoseconds_option, DateTime, Utc};
use resources::{
    primitives::Id, AuditEvent, Communication, ErxBundle, KbvBinary, KbvBundle, MedicationDispense,
    Task,
};
use serde::{Deserialize, Serialize};
use serde_json::{from_reader, to_writer};

use crate::{error::Error, fhir::security::Signed};

use super::{History, Inner, TaskMeta, Version};

impl Inner {
    pub fn load<R>(&mut self, reader: R) -> Result<(), Error>
    where
        R: Read,
    {
        let data: Data = from_reader(reader)?;

        for task in data.tasks {
            let task_meta = TaskMeta {
                history: task
                    .history
                    .into_iter()
                    .collect::<Result<History<_>, _>>()?,
                accept_timestamp: task.accept_timestamp,
            };
            let id = task_meta
                .history
                .get_current()
                .resource
                .id
                .as_ref()
                .unwrap()
                .clone();

            self.tasks.insert(id, task_meta);
        }

        for (id, kbv_binary) in data.e_prescriptions {
            self.e_prescriptions.insert(id, kbv_binary);
        }

        for patient_receipt in data.patient_receipts {
            self.patient_receipts
                .insert(patient_receipt.id.clone(), Signed::new(patient_receipt));
        }

        for erx_receipt in data.erx_receipts {
            self.erx_receipts
                .insert(erx_receipt.id.clone(), erx_receipt);
        }

        for communication in data.communications {
            self.communications
                .insert(communication.id().as_ref().unwrap().clone(), communication);
        }

        for medication_dispense in data.medication_dispenses {
            self.medication_dispenses.insert(
                medication_dispense.id.as_ref().unwrap().clone(),
                medication_dispense,
            );
        }

        for audit_event in data.audit_events {
            self.audit_events
                .entry(audit_event.entity.name.clone())
                .or_default()
                .push(audit_event);
        }

        Ok(())
    }

    pub fn save<W>(&self, writer: W) -> Result<(), Error>
    where
        W: Write,
    {
        let data = Data {
            tasks: self.tasks.values().map(From::from).collect(),
            e_prescriptions: self
                .e_prescriptions
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            patient_receipts: self
                .patient_receipts
                .values()
                .map(Deref::deref)
                .cloned()
                .collect(),
            erx_receipts: self.erx_receipts.values().cloned().collect(),
            communications: self.communications.values().cloned().collect(),
            medication_dispenses: self.medication_dispenses.values().cloned().collect(),
            audit_events: self.audit_events.values().flatten().cloned().collect(),
        };

        to_writer(writer, &data)?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct Data {
    tasks: Vec<TaskData>,
    e_prescriptions: Vec<(Id, KbvBinary)>,
    patient_receipts: Vec<KbvBundle>,
    erx_receipts: Vec<ErxBundle>,
    communications: Vec<Communication>,
    medication_dispenses: Vec<MedicationDispense>,
    audit_events: Vec<AuditEvent>,
}

#[derive(Serialize, Deserialize)]
struct TaskData {
    history: Vec<Version<Task>>,

    #[serde(with = "ts_nanoseconds_option")]
    accept_timestamp: Option<DateTime<Utc>>,
}

impl From<&TaskMeta> for TaskData {
    fn from(v: &TaskMeta) -> Self {
        Self {
            history: v.history.iter().cloned().collect(),
            accept_timestamp: v.accept_timestamp,
        }
    }
}

#[cfg(test)]
pub mod tests {
    use std::fs::read_to_string;
    use std::str::from_utf8;

    use openssl::{pkey::PKey, x509::X509};

    use crate::fhir::tests::trim_json_str;

    use super::super::State;

    #[tokio::test]
    pub async fn load_save() {
        let sig_key = PKey::generate_ed448().unwrap();
        let sig_cert = X509::builder().unwrap().build();

        let state = State::new(sig_key, sig_cert);
        let mut state = state.lock().await;

        let expected = read_to_string("./examples/state.json").unwrap();
        let expected = trim_json_str(&expected);
        state.load(expected.as_bytes()).unwrap();

        let mut actual = Vec::new();
        state.save(&mut actual).unwrap();

        let actual = from_utf8(&actual).unwrap();
        assert_eq!(actual, expected);
    }
}
