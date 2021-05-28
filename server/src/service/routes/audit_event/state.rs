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

use std::cell::RefCell;
use std::collections::{hash_map::Entry, HashMap, HashSet};
use std::fmt::Display;
use std::rc::Rc;

use chrono::Utc;
use resources::{
    audit_event::{
        Action, Agent, AuditEvent, Entity, Outcome, ParticipationRoleType, Source, SubType, Text,
        What,
    },
    misc::{Kvnr, PrescriptionId},
    primitives::Id,
};

use crate::{
    service::misc::DEVICE,
    state::{Inner, Timeouts},
};

use super::Error;

#[derive(Default)]
pub struct AuditEvents {
    by_id: HashMap<Id, AuditEvent>,
    by_kvnr: HashMap<Kvnr, HashSet<Id>>,
    by_task: HashMap<Id, HashSet<Id>>,
}

impl AuditEvents {
    pub fn insert(&mut self, audit_event: AuditEvent) {
        let id = audit_event.id.clone();
        let kvnr = audit_event.entity.name.clone();
        let task_id = match &audit_event.entity.what {
            What::Task(task_id) => Some(task_id.clone()),
            What::MedicationDispense(_) => None,
            _ => None,
        };

        match self.by_id.entry(id.clone()) {
            Entry::Occupied(_) => {
                panic!("Audit event with this ID ({}) already exists!", id);
            }
            Entry::Vacant(entry) => {
                entry.insert(audit_event);
            }
        }

        self.by_kvnr.entry(kvnr).or_default().insert(id.clone());

        if let Some(task_id) = task_id {
            self.by_task.entry(task_id).or_default().insert(id);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &AuditEvent> {
        self.by_id.values()
    }

    pub fn get_by_id(&self, id: &Id) -> Option<&AuditEvent> {
        self.by_id.get(id)
    }
}

impl Inner {
    pub fn audit_event_get(&self, id: Id, kvnr: &Kvnr) -> Result<&AuditEvent, Error> {
        let event = match self.audit_events.by_id.get(&id) {
            Some(events) => events,
            None => return Err(Error::NotFound(id)),
        };

        if &event.entity.name != kvnr {
            return Err(Error::Forbidden(id));
        }

        Ok(event)
    }

    pub fn audit_event_iter<F>(&self, kvnr: &Kvnr, mut f: F) -> impl Iterator<Item = &AuditEvent>
    where
        F: FnMut(&AuditEvent) -> bool,
    {
        let Self {
            ref audit_events, ..
        } = self;

        lazy_static! {
            static ref EMPTY: HashSet<Id> = HashSet::new();
        }

        let events = match audit_events.by_kvnr.get(&kvnr) {
            Some(events) => events,
            None => &EMPTY,
        };

        events.iter().filter_map(move |id| {
            let v = audit_events.by_id.get(&id).unwrap();
            if f(v) {
                Some(v)
            } else {
                None
            }
        })
    }

    pub fn audit_event_iter_by_task(
        &self,
        task_id: &Id,
    ) -> Option<impl Iterator<Item = &AuditEvent>> {
        let AuditEvents {
            ref by_id,
            ref by_task,
            ..
        } = &self.audit_events;

        Some(
            by_task
                .get(task_id)?
                .iter()
                .map(move |id| by_id.get(id).unwrap()),
        )
    }

    pub fn audit_event_delete_by_id(&mut self, id: &Id) {
        let Self {
            ref mut audit_events,
            ..
        } = self;

        let audit_event = audit_events.by_id.get(id).unwrap();
        let kvnr = &audit_event.entity.name;

        if let Some(ids) = audit_events.by_kvnr.get_mut(&kvnr) {
            ids.remove(id);
        }

        let ids = match &audit_event.entity.what {
            What::Task(task_id) => audit_events.by_task.get_mut(&task_id),
            What::MedicationDispense(_) => None,
            _ => None,
        };

        if let Some(ids) = ids {
            ids.remove(id);
        }

        audit_events.by_id.remove(id);
    }

    pub fn agent() -> &'static Agent {
        &AGENT
    }

    pub fn logged<F, T, E>(
        audit_events: &mut AuditEvents,
        timeouts: Rc<RefCell<&mut Timeouts>>,
        f: F,
    ) -> Result<T, E>
    where
        F: FnOnce(&mut Builder) -> Result<T, E>,
        E: Display,
    {
        let mut builder = Self::audit_event_builder();
        let ret = f(&mut builder);

        let err = ret.as_ref().err().map(|err| format!("{}", err));

        let mut timeouts = timeouts.borrow_mut();
        builder.build(audit_events, &mut timeouts, err);

        ret
    }

    pub fn audit_event_builder() -> Builder {
        Builder::new()
    }
}

#[derive(Debug)]
pub struct Builder {
    error_outcome: Option<Outcome>,
    sub_type: Option<SubType>,
    action: Option<Action>,
    agent: Option<Agent>,
    what: Option<What>,
    patient: Option<Kvnr>,
    description: Option<PrescriptionId>,
    text: Option<Text>,
}

#[allow(dead_code)]
impl Builder {
    fn new() -> Self {
        Self {
            sub_type: None,
            action: None,
            patient: None,
            agent: None,
            what: None,
            description: None,
            error_outcome: None,
            text: None,
        }
    }

    pub fn build(
        self,
        audit_events: &mut AuditEvents,
        timeouts: &mut Timeouts,
        error: Option<String>,
    ) -> Option<()> {
        let sub_type = self.sub_type?;
        let action = self.action?;
        let (outcome, outcome_description) = if let Some(error) = error {
            (
                self.error_outcome.unwrap_or(Outcome::MinorFailure),
                Some(error),
            )
        } else {
            (Outcome::Success, None)
        };
        let agent = self.agent?;
        let what = self.what?;
        let patient = self.patient?;
        let description = self.description;
        let text = self.text;

        let event = AuditEvent {
            id: Id::generate().unwrap(),
            text,
            sub_type,
            action,
            recorded: Utc::now().into(),
            outcome,
            outcome_description,
            agent,
            source: Source {
                observer: format!("Device/{}", &DEVICE.id),
            },
            entity: Entity {
                what,
                name: patient,
                description,
            },
        };

        timeouts.insert(&event);

        audit_events.insert(event);

        Some(())
    }

    pub fn error_outcome(&mut self, value: Outcome) -> &mut Self {
        self.error_outcome = Some(value);

        self
    }

    pub fn sub_type(&mut self, value: SubType) -> &mut Self {
        self.sub_type = Some(value);

        self
    }

    pub fn action(&mut self, value: Action) -> &mut Self {
        self.action = Some(value);

        self
    }

    pub fn agent(&mut self, value: Agent) -> &mut Self {
        self.agent = Some(value);

        self
    }

    pub fn what(&mut self, value: What) -> &mut Self {
        self.what = Some(value);

        self
    }

    pub fn patient(&mut self, value: Kvnr) -> &mut Self {
        self.patient = Some(value);

        self
    }

    pub fn patient_opt(&mut self, value: Option<Kvnr>) -> &mut Self {
        if let Some(value) = value {
            self.patient = Some(value);
        }

        self
    }

    pub fn description(&mut self, value: PrescriptionId) -> &mut Self {
        self.description = Some(value);

        self
    }

    pub fn description_opt(&mut self, value: Option<PrescriptionId>) -> &mut Self {
        if let Some(value) = value {
            self.description = Some(value);
        }

        self
    }

    pub fn text(&mut self, value: Text) -> &mut Self {
        self.text = Some(value);

        self
    }
}

lazy_static! {
    static ref AGENT: Agent = Agent {
        type_: ParticipationRoleType::HumanUser,
        who: None,
        name: DEVICE.device_name.name.clone(),
        requestor: false,
    };
}
