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

use std::collections::HashMap;
use std::convert::TryInto;
use std::str::FromStr;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use resources::{
    communication::{Communication, Inner as CommunicationInner},
    misc::{Kvnr, TelematikId},
    primitives::Id,
    AuditEvent, ErxBundle, KbvBinary, KbvBundle, MedicationDispense, Task,
};
use tokio::sync::{Mutex, MutexGuard};
use url::Url;

use crate::fhir::security::Signed;

use super::{header::XAccessCode, misc::History, routes::task::Error as TaskError, RequestError};

#[derive(Default, Clone)]
pub struct State(Arc<Mutex<Inner>>);

#[derive(Default)]
pub struct Inner {
    pub tasks: HashMap<Id, TaskMeta>,
    pub e_prescriptions: HashMap<Id, (KbvBinary, KbvBundle)>,
    pub patient_receipts: HashMap<Id, Signed<KbvBundle>>,
    pub erx_receipts: HashMap<Id, ErxBundle>,
    pub communications: HashMap<Id, Communication>,
    pub medication_dispense: HashMap<Id, MedicationDispense>,
    pub audit_events: HashMap<Id, AuditEvent>,
}

pub struct TaskMeta {
    pub history: History<Task>,
    pub accept_timestamp: Option<DateTime<Utc>>,
}

pub enum CommunicationMatch<'a> {
    NotFound,
    Unauthorized,
    Sender(&'a mut Communication),
    Recipient(&'a mut Communication),
}

impl State {
    pub async fn lock(&self) -> MutexGuard<'_, Inner> {
        self.0.lock().await
    }

    pub fn parse_task_url(uri: &str) -> Result<(Id, Option<XAccessCode>), RequestError> {
        let url = format!("http://localhost/{}", uri);
        let url = Url::from_str(&url).map_err(|_| TaskError::InvalidUrl(uri.into()))?;

        let mut path = url
            .path_segments()
            .ok_or_else(|| TaskError::InvalidUrl(uri.into()))?;
        if path.next() != Some("Task") {
            return Err(TaskError::InvalidUrl(uri.into()).into());
        }

        let task_id = path
            .next()
            .ok_or_else(|| TaskError::InvalidUrl(uri.into()))?;
        let task_id = task_id
            .try_into()
            .map_err(|_| TaskError::InvalidUrl(uri.into()))?;

        let access_code = url.query_pairs().find_map(|(key, value)| {
            if key == "ac" {
                Some(XAccessCode(value.into_owned()))
            } else {
                None
            }
        });

        Ok((task_id, access_code))
    }
}

impl Inner {
    pub fn get_task(
        &self,
        id: &Id,
        kvnr: &Option<Kvnr>,
        access_code: &Option<XAccessCode>,
    ) -> Option<Result<&TaskMeta, ()>> {
        let task_meta = match self.tasks.get(&id) {
            Some(task) => task,
            None => return None,
        };

        let task = task_meta.history.get();
        if task_matches(task, kvnr, access_code) {
            Some(Ok(task_meta))
        } else {
            Some(Err(()))
        }
    }

    pub fn get_task_mut(
        &mut self,
        id: &Id,
        kvnr: &Option<Kvnr>,
        access_code: &Option<XAccessCode>,
    ) -> Option<Result<&mut TaskMeta, ()>> {
        let task_meta = match self.tasks.get_mut(&id) {
            Some(task_meta) => task_meta,
            None => return None,
        };

        let task = task_meta.history.get();
        if task_matches(task, kvnr, access_code) {
            Some(Ok(task_meta))
        } else {
            Some(Err(()))
        }
    }

    pub fn iter_tasks(
        &self,
        kvnr: Option<Kvnr>,
        access_code: Option<XAccessCode>,
    ) -> impl Iterator<Item = &TaskMeta> {
        self.tasks.iter().filter_map(move |(_, task_meta)| {
            let task = task_meta.history.get();
            if task_matches(task, &kvnr, &access_code) {
                Some(task_meta)
            } else {
                None
            }
        })
    }

    pub fn get_communication(
        &mut self,
        id: &Id,
        kvnr: &Option<Kvnr>,
        telematik_id: &Option<TelematikId>,
    ) -> CommunicationMatch<'_> {
        let communication = match self.communications.get_mut(id) {
            Some(communication) => communication,
            None => return CommunicationMatch::NotFound,
        };

        communication_matches(communication, kvnr, telematik_id)
    }

    pub fn iter_communications(
        &mut self,
        kvnr: Option<Kvnr>,
        telematik_id: Option<TelematikId>,
    ) -> impl Iterator<Item = CommunicationMatch<'_>> {
        self.communications
            .iter_mut()
            .filter_map(move |(_, communication)| {
                let m = communication_matches(communication, &kvnr, &telematik_id);
                match m {
                    CommunicationMatch::NotFound => None,
                    CommunicationMatch::Unauthorized => None,
                    _ => Some(m),
                }
            })
    }

    pub fn remove_communications(&mut self, task_id: &Id) {
        self.communications.retain(|_, c| {
            if let Some(based_on) = c.based_on() {
                let (id, _) = State::parse_task_url(&based_on).unwrap();

                &id != task_id
            } else {
                true
            }
        })
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

fn task_matches(task: &Task, kvnr: &Option<Kvnr>, access_code: &Option<XAccessCode>) -> bool {
    match (task.for_.as_ref(), kvnr) {
        (Some(task_kvnr), Some(kvnr)) if task_kvnr == kvnr => return true,
        _ => (),
    }

    match (task.identifier.access_code.as_ref(), access_code) {
        (Some(task_ac), Some(ac)) if task_ac == ac => return true,
        _ => (),
    }

    false
}

fn communication_matches<'a>(
    communication: &'a mut Communication,
    kvnr: &Option<Kvnr>,
    telematik_id: &Option<TelematikId>,
) -> CommunicationMatch<'a> {
    match communication {
        Communication::InfoReq(CommunicationInner {
            sender, recipient, ..
        })
        | Communication::DispenseReq(CommunicationInner {
            sender, recipient, ..
        }) => {
            match (kvnr, sender) {
                (Some(kvnr), Some(sender)) if kvnr == sender => {
                    return CommunicationMatch::Sender(communication)
                }
                _ => (),
            }

            match (telematik_id, recipient) {
                (Some(telematik_id), recipient) if telematik_id == recipient => {
                    return CommunicationMatch::Recipient(communication)
                }
                _ => (),
            }

            CommunicationMatch::Unauthorized
        }
        Communication::Reply(CommunicationInner {
            sender, recipient, ..
        }) => {
            match (telematik_id, sender) {
                (Some(telematik_id), Some(sender)) if telematik_id == sender => {
                    return CommunicationMatch::Sender(communication)
                }
                _ => (),
            }

            match (kvnr, recipient) {
                (Some(kvnr), recipient) if kvnr == recipient => {
                    return CommunicationMatch::Recipient(communication)
                }
                _ => (),
            }

            CommunicationMatch::Unauthorized
        }
        Communication::Representative(CommunicationInner {
            sender, recipient, ..
        }) => {
            match (kvnr, sender) {
                (Some(kvnr), Some(sender)) if kvnr == sender => {
                    return CommunicationMatch::Sender(communication)
                }
                _ => (),
            }

            match (kvnr, recipient) {
                (Some(kvnr), recipient) if kvnr == recipient => {
                    return CommunicationMatch::Recipient(communication)
                }
                _ => (),
            }

            CommunicationMatch::Unauthorized
        }
    }
}
