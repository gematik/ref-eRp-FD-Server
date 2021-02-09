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
use std::fmt::Display;

use chrono::Utc;
use resources::{
    audit_event::{Action, Agent, AuditEvent, Entity, Outcome, Source, SubType},
    misc::{Kvnr, PrescriptionId},
    primitives::Id,
};

use crate::{service::misc::DEVICE, state::Inner};

use super::Error;

impl Inner {
    pub fn audit_event_get(&self, id: Id, kvnr: &Kvnr) -> Result<&AuditEvent, Error> {
        let events = match self.audit_events.get(&kvnr) {
            Some(events) => events,
            None => return Err(Error::NotFound(id)),
        };

        let event = match events.iter().find(|av| av.id == id) {
            Some(event) => event,
            None => return Err(Error::NotFound(id)),
        };

        Ok(event)
    }

    pub fn audit_event_iter<F>(&self, kvnr: &Kvnr, mut f: F) -> impl Iterator<Item = &AuditEvent>
    where
        F: FnMut(&AuditEvent) -> bool,
    {
        static EMPTY: Vec<AuditEvent> = Vec::new();

        let events = match self.audit_events.get(&kvnr) {
            Some(events) => events,
            None => &EMPTY,
        };

        events.iter().filter(move |v| f(v))
    }

    pub fn logged<F, T, E>(audit_events: &mut HashMap<Kvnr, Vec<AuditEvent>>, f: F) -> Result<T, E>
    where
        F: FnOnce(&mut Builder) -> Result<T, E>,
        E: Display,
    {
        let mut builder = Self::audit_event_builder();
        let ret = f(&mut builder);

        let err = ret.as_ref().err().map(|err| format!("{}", err));

        builder.build(audit_events, err);

        ret
    }

    pub fn audit_event_builder() -> Builder {
        Builder::new()
    }
}

pub struct Builder {
    error_outcome: Option<Outcome>,
    sub_type: Option<SubType>,
    action: Option<Action>,
    agent: Option<Agent>,
    what: Option<String>,
    patient: Option<Kvnr>,
    description: Option<PrescriptionId>,
    text: Option<String>,
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
        audit_events: &mut HashMap<Kvnr, Vec<AuditEvent>>,
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
        let description = self.description?;
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
                observer: format!("{} {}", &DEVICE.device_name.name, &DEVICE.version),
            },
            entity: Entity {
                what,
                name: patient.clone(),
                description,
            },
        };

        let events = audit_events.entry(patient).or_default();
        events.push(event);

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

    pub fn what<T>(&mut self, value: T) -> &mut Self
    where
        T: Into<String>,
    {
        self.what = Some(value.into());

        self
    }

    pub fn patient(&mut self, value: Kvnr) -> &mut Self {
        self.patient = Some(value);

        self
    }

    pub fn description(&mut self, value: PrescriptionId) -> &mut Self {
        self.description = Some(value);

        self
    }

    pub fn text<T>(&mut self, value: T) -> &mut Self
    where
        T: Into<String>,
    {
        self.text = Some(value.into());

        self
    }
}
