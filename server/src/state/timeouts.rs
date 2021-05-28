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

use std::mem::swap;

use chrono::{naive::NaiveDate, DateTime, Duration, Utc};
use tokio::{
    spawn,
    time::{delay_for, Duration as TokioDuration},
};

use resources::{primitives::Id, task::Status, AuditEvent, MedicationDispense, Task};

use super::{Inner, State};

#[derive(Debug)]
pub enum ResourceId {
    Task(Id),
    AuditEvent(Id),
    MedicationDispense(Id),
}

#[derive(Default)]
pub struct Timeouts {
    items: Vec<Item>,
}

pub trait TimeoutResource {
    fn id(&self) -> ResourceId;
    fn timeout(&self) -> DateTime<Utc>;
}

struct Item {
    id: ResourceId,
    timeout: DateTime<Utc>,
}

impl Timeouts {
    pub fn insert<R>(&mut self, resource: &R)
    where
        R: TimeoutResource,
    {
        let item = Item {
            id: resource.id(),
            timeout: resource.timeout(),
        };

        let index = match self
            .items
            .binary_search_by(|i| i.timeout.cmp(&item.timeout))
        {
            Ok(index) => index,
            Err(index) => index,
        };

        self.items.insert(index, item);
    }

    fn split_of_timeouts(&mut self, now: &DateTime<Utc>) -> Vec<Item> {
        let mut index = match self.items.binary_search_by_key(now, |i| i.timeout) {
            Ok(index) => index,
            Err(index) => index,
        };

        while index < self.items.len() && now <= &self.items[index].timeout {
            index += 1;
        }

        let mut tail = self.items.split_off(index);

        swap(&mut tail, &mut self.items);

        tail
    }
}

impl State {
    pub(super) fn spawn_timeout_task(&self) {
        let store = self.clone();

        spawn(timeout_task(store));
    }
}

impl Inner {
    fn progress_timeouts(&mut self) {
        let now = Utc::now();
        let items = self.timeouts.split_of_timeouts(&now);
        let ids = items.into_iter().map(|i| i.id);

        for id in ids {
            match id {
                ResourceId::Task(id) => {
                    let task_meta = match self.tasks.get_by_id(&id) {
                        Some(task) => task,
                        None => continue,
                    };

                    if task_meta.task.timeout() > now {
                        continue;
                    }

                    self.task_delete_by_id(&id);
                }
                ResourceId::AuditEvent(id) => {
                    let audit_event = match self.audit_events.get_by_id(&id) {
                        Some(audit_event) => audit_event,
                        None => continue,
                    };

                    if audit_event.timeout() > now {
                        continue;
                    }

                    self.audit_event_delete_by_id(&id);
                }
                ResourceId::MedicationDispense(id) => {
                    let md = match self.medication_dispenses.get_by_id(&id) {
                        Some(md) => md,
                        None => continue,
                    };

                    if md.timeout() > now {
                        continue;
                    }

                    self.medication_dispense_delete_by_id(&id);
                }
            }
        }
    }
}

impl TimeoutResource for MedicationDispense {
    fn id(&self) -> ResourceId {
        let id = self
            .id
            .as_ref()
            .expect("MedicationDispense without Id!")
            .clone();

        ResourceId::MedicationDispense(id)
    }

    fn timeout(&self) -> DateTime<Utc> {
        let date: DateTime<Utc> = self.when_handed_over.clone().into();

        date + Duration::days(100)
    }
}

impl TimeoutResource for AuditEvent {
    fn id(&self) -> ResourceId {
        let id = self.id.clone();

        ResourceId::AuditEvent(id)
    }

    fn timeout(&self) -> DateTime<Utc> {
        *self.recorded + Duration::days(3 * 365)
    }
}

impl TimeoutResource for Task {
    fn id(&self) -> ResourceId {
        let id = self.id.clone();

        ResourceId::Task(id)
    }

    fn timeout(&self) -> DateTime<Utc> {
        match self.status {
            Status::Draft => Utc::now() + Duration::days(1),
            Status::Ready => {
                let date = self.extension.expiry_date.as_ref().unwrap();
                let date = NaiveDate::parse_from_str(&date, "%Y-%m-%d").unwrap();
                let date = date.and_hms(0, 0, 0) + Duration::days(10);

                DateTime::from_utc(date, Utc)
            }
            Status::Cancelled => Utc::now() + Duration::days(10),
            Status::InProgress => Utc::now() + Duration::days(100),
            Status::Completed => Utc::now() + Duration::days(100),
            _ => unreachable!("Invalid Task Status"),
        }
    }
}

async fn timeout_task(state: State) {
    loop {
        state.lock().await.progress_timeouts();

        delay_for(TokioDuration::from_secs(60)).await;
    }
}
