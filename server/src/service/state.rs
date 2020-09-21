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

use std::collections::HashMap;
use std::sync::Arc;

use resources::{
    communication::{Communication, Inner as CommunicationInner},
    misc::{Kvnr, TelematikId},
    primitives::Id,
    KbvBundle, Task,
};
use tokio::sync::{Mutex, MutexGuard};

use super::header::XAccessCode;

#[derive(Default, Clone)]
pub struct State(Arc<Mutex<Inner>>);

#[derive(Default)]
pub struct Inner {
    pub tasks: HashMap<Id, Task>,
    pub e_prescriptions: HashMap<Id, KbvBundle>,
    pub patient_receipts: HashMap<Id, KbvBundle>,
    pub communications: HashMap<Id, Communication>,
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
}

impl Inner {
    pub fn get_task(
        &self,
        id: &Id,
        kvnr: &Option<Kvnr>,
        access_code: &Option<XAccessCode>,
    ) -> Option<Result<&Task, ()>> {
        let task = match self.tasks.get(&id) {
            Some(task) => task,
            None => return None,
        };

        if task_matches(task, kvnr, access_code) {
            Some(Ok(task))
        } else {
            Some(Err(()))
        }
    }

    pub fn iter_tasks(
        &self,
        kvnr: Option<Kvnr>,
        access_code: Option<XAccessCode>,
    ) -> impl Iterator<Item = &Task> {
        self.tasks.iter().filter_map(move |(_, task)| {
            if task_matches(task, &kvnr, &access_code) {
                Some(task)
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
