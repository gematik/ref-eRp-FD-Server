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

use crate::{error::Error, fhir::security::Signed, service::TaskMeta};

use super::Inner;

impl Inner {
    pub fn load<R>(&mut self, reader: R) -> Result<(), Error>
    where
        R: Read,
    {
        let version: Version = from_reader(reader)?;

        match version {
            Version::Old(data) => old::load(self, data),
            Version::V3(data) => v3::load(self, data),
        }
    }

    pub fn save<W>(&self, writer: W) -> Result<(), Error>
    where
        W: Write,
    {
        let version = Version::V3(v3::save(self));

        to_writer(writer, &version)?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "version")]
enum Version {
    #[serde(alias = "v3")]
    V3(v3::Data),

    #[serde(alias = "old")]
    Old(old::Data),
}

mod v3 {
    use super::*;

    pub fn load(inner: &mut Inner, data: Data) -> Result<(), Error> {
        for task in data.tasks {
            let task_meta = TaskMeta {
                task: task.task,
                accept_timestamp: task.accept_timestamp,
                communication_count: task.communication_count,
            };

            inner.tasks.insert_task_meta(task_meta);
        }

        for (id, kbv_binary) in data.e_prescriptions {
            inner.e_prescriptions.insert(id, kbv_binary);
        }

        for patient_receipt in data.patient_receipts {
            inner
                .patient_receipts
                .insert_signed(Signed::new(patient_receipt));
        }

        for erx_receipt in data.erx_receipts {
            inner.erx_receipts.insert_signed(Signed::new(erx_receipt));
        }

        for communication in data.communications {
            inner.communications.insert(communication);
        }

        for medication_dispense in data.medication_dispenses {
            inner.medication_dispenses.insert(medication_dispense);
        }

        for audit_event in data.audit_events {
            inner.audit_events.insert(audit_event)
        }

        Ok(())
    }

    pub fn save(inner: &Inner) -> Data {
        let mut data = Data {
            tasks: inner.tasks.iter().map(From::from).collect(),
            e_prescriptions: inner
                .e_prescriptions
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            patient_receipts: inner
                .patient_receipts
                .iter()
                .map(Deref::deref)
                .cloned()
                .collect(),
            erx_receipts: inner
                .erx_receipts
                .iter()
                .map(Deref::deref)
                .cloned()
                .collect(),
            communications: inner.communications.iter().cloned().collect(),
            medication_dispenses: inner.medication_dispenses.iter().cloned().collect(),
            audit_events: inner.audit_events.iter().cloned().collect(),
        };

        data.tasks.sort_by(|a, b| {
            let a = &a.task.id;
            let b = &b.task.id;

            a.cmp(&b)
        });
        data.e_prescriptions.sort_by(|(a, _), (b, _)| a.cmp(&b));
        data.patient_receipts.sort_by(|a, b| a.id.cmp(&b.id));
        data.erx_receipts.sort_by(|a, b| a.id.cmp(&b.id));
        data.communications.sort_by(|a, b| {
            let a = a.id().as_ref().unwrap();
            let b = b.id().as_ref().unwrap();

            a.cmp(&b)
        });
        data.medication_dispenses.sort_by(|a, b| {
            let a = a.id.as_ref().unwrap();
            let b = b.id.as_ref().unwrap();

            a.cmp(&b)
        });
        data.audit_events.sort_by(|a, b| a.id.cmp(&b.id));

        data
    }

    #[derive(Serialize, Deserialize)]
    pub struct Data {
        tasks: Vec<TaskData>,
        e_prescriptions: Vec<(Id, KbvBinary)>,
        patient_receipts: Vec<KbvBundle>,
        erx_receipts: Vec<ErxBundle>,
        communications: Vec<Communication>,
        medication_dispenses: Vec<MedicationDispense>,
        audit_events: Vec<AuditEvent>,
    }

    #[derive(Serialize, Deserialize)]
    pub struct TaskData {
        task: Task,

        #[serde(with = "ts_nanoseconds_option")]
        accept_timestamp: Option<DateTime<Utc>>,

        #[serde(default)]
        communication_count: usize,
    }

    impl From<&TaskMeta> for TaskData {
        fn from(v: &TaskMeta) -> Self {
            Self {
                task: v.task.clone(),
                accept_timestamp: v.accept_timestamp,
                communication_count: v.communication_count,
            }
        }
    }
}

mod old {
    use super::*;

    pub fn load(inner: &mut Inner, data: Data) -> Result<(), Error> {
        for task in data.tasks {
            let task_meta = TaskMeta {
                task: task
                    .history
                    .into_iter()
                    .last()
                    .ok_or_else(|| {
                        Error::Generic("State must contain at least one task version!".into())
                    })?
                    .resource,
                accept_timestamp: task.accept_timestamp,
                communication_count: task.communication_count,
            };

            inner.tasks.insert_task_meta(task_meta);
        }

        for (id, kbv_binary) in data.e_prescriptions {
            inner.e_prescriptions.insert(id, kbv_binary);
        }

        for patient_receipt in data.patient_receipts {
            inner
                .patient_receipts
                .insert_signed(Signed::new(patient_receipt));
        }

        for erx_receipt in data.erx_receipts {
            inner.erx_receipts.insert_signed(Signed::new(erx_receipt));
        }

        for communication in data.communications {
            inner.communications.insert(communication);
        }

        for medication_dispense in data.medication_dispenses {
            inner.medication_dispenses.insert(medication_dispense);
        }

        for audit_event in data.audit_events {
            inner.audit_events.insert(audit_event)
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn save(inner: &Inner) -> Data {
        let mut data = Data {
            tasks: inner.tasks.iter().map(From::from).collect(),
            e_prescriptions: inner
                .e_prescriptions
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            patient_receipts: inner
                .patient_receipts
                .iter()
                .map(Deref::deref)
                .cloned()
                .collect(),
            erx_receipts: inner
                .erx_receipts
                .iter()
                .map(Deref::deref)
                .cloned()
                .collect(),
            communications: inner.communications.iter().cloned().collect(),
            medication_dispenses: inner.medication_dispenses.iter().cloned().collect(),
            audit_events: inner.audit_events.iter().cloned().collect(),
        };

        data.tasks.sort_by(|a, b| {
            let a = &a.history.first().unwrap().resource.id;
            let b = &b.history.first().unwrap().resource.id;

            a.cmp(&b)
        });
        data.e_prescriptions.sort_by(|(a, _), (b, _)| a.cmp(&b));
        data.patient_receipts.sort_by(|a, b| a.id.cmp(&b.id));
        data.erx_receipts.sort_by(|a, b| a.id.cmp(&b.id));
        data.communications.sort_by(|a, b| {
            let a = a.id().as_ref().unwrap();
            let b = b.id().as_ref().unwrap();

            a.cmp(&b)
        });
        data.medication_dispenses.sort_by(|a, b| {
            let a = a.id.as_ref().unwrap();
            let b = b.id.as_ref().unwrap();

            a.cmp(&b)
        });
        data.audit_events.sort_by(|a, b| a.id.cmp(&b.id));

        data
    }

    #[derive(Serialize, Deserialize)]
    pub struct Data {
        tasks: Vec<TaskData>,
        e_prescriptions: Vec<(Id, KbvBinary)>,
        patient_receipts: Vec<KbvBundle>,
        erx_receipts: Vec<ErxBundle>,
        communications: Vec<Communication>,
        medication_dispenses: Vec<MedicationDispense>,
        audit_events: Vec<AuditEvent>,
    }

    #[derive(Serialize, Deserialize)]
    pub struct TaskData {
        history: Vec<TaskVersion>,

        #[serde(with = "ts_nanoseconds_option")]
        accept_timestamp: Option<DateTime<Utc>>,

        #[serde(default)]
        communication_count: usize,
    }

    #[derive(Serialize, Deserialize)]
    pub struct TaskVersion {
        version_id: usize,
        timestamp: usize,
        resource: Task,
    }

    impl From<&TaskMeta> for TaskData {
        fn from(v: &TaskMeta) -> Self {
            Self {
                history: vec![TaskVersion {
                    version_id: 0,
                    timestamp: 0,
                    resource: v.task.clone(),
                }],
                accept_timestamp: v.accept_timestamp,
                communication_count: v.communication_count,
            }
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
    pub async fn load_save_v1() {
        let sig_key = PKey::generate_ed448().unwrap();
        let sig_cert = X509::builder().unwrap().build();

        let state = State::new(sig_key, sig_cert, 10, 500, "999 Throttling active".into());
        let mut state = state.lock().await;

        let content = read_to_string("./examples/state_load_v1.json").unwrap();
        let content = trim_json_str(&content);
        state.load(content.as_bytes()).unwrap();

        let expected = read_to_string("./examples/state_save_v1_to_v3.json").unwrap();
        let expected = trim_json_str(&expected);

        let mut actual = Vec::new();
        state.save(&mut actual).unwrap();

        let actual = from_utf8(&actual).unwrap();
        assert_eq!(actual, expected);
    }

    #[tokio::test]
    pub async fn load_save_v2() {
        let sig_key = PKey::generate_ed448().unwrap();
        let sig_cert = X509::builder().unwrap().build();

        let state = State::new(sig_key, sig_cert, 10, 500, "999 Throttling active".into());
        let mut state = state.lock().await;

        let content = read_to_string("./examples/state_load_v2.json").unwrap();
        let content = trim_json_str(&content);
        state.load(content.as_bytes()).unwrap();

        let expected = read_to_string("./examples/state_save_v2_to_v3.json").unwrap();
        let expected = trim_json_str(&expected);

        let mut actual = Vec::new();
        state.save(&mut actual).unwrap();

        let actual = from_utf8(&actual).unwrap();
        assert_eq!(actual, expected);
    }

    #[tokio::test]
    pub async fn load_save_v3() {
        let sig_key = PKey::generate_ed448().unwrap();
        let sig_cert = X509::builder().unwrap().build();

        let state = State::new(sig_key, sig_cert, 10, 500, "999 Throttling active".into());
        let mut state = state.lock().await;

        let content = read_to_string("./examples/state_load_v3.json").unwrap();
        let content = trim_json_str(&content);
        state.load(content.as_bytes()).unwrap();

        let expected = read_to_string("./examples/state_save_v3_to_v3.json").unwrap();
        let expected = trim_json_str(&expected);

        let mut actual = Vec::new();
        state.save(&mut actual).unwrap();

        let actual = from_utf8(&actual).unwrap();
        assert_eq!(actual, expected);
    }
}
