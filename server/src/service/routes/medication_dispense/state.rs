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
use std::rc::Rc;

use resources::{
    audit_event::{Action, Agent, SubType, Text, What},
    misc::{Kvnr, PrescriptionId},
    primitives::Id,
    MedicationDispense,
};

use crate::state::Inner;

use super::Error;

#[derive(Default)]
pub struct MedicationDispenses {
    by_id: HashMap<Id, MedicationDispense>,
    by_kvnr: HashMap<Kvnr, HashSet<Id>>,
    by_prescription_id: HashMap<PrescriptionId, Id>,
}

impl MedicationDispenses {
    pub fn insert(&mut self, medication_dispense: MedicationDispense) {
        let id = medication_dispense.id.as_ref().unwrap().clone();
        let kvnr = medication_dispense.subject.clone();
        let prescription_id = medication_dispense.prescription_id.clone();

        match self.by_id.entry(id.clone()) {
            Entry::Occupied(_) => {
                panic!("Medication dispense with this ID ({}) already exists!", id);
            }
            Entry::Vacant(entry) => {
                entry.insert(medication_dispense);
            }
        }

        self.by_kvnr.entry(kvnr).or_default().insert(id.clone());
        self.by_prescription_id.insert(prescription_id, id);
    }

    pub fn get_by_id(&self, id: &Id) -> Option<&MedicationDispense> {
        self.by_id.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &MedicationDispense> {
        self.by_id.values()
    }

    pub fn remove_by_prescription_id(&mut self, prescription_id: &PrescriptionId) {
        if let Some(id) = self.by_prescription_id.remove(prescription_id) {
            let md = self.by_id.remove(&id).unwrap();

            let id = md.id.unwrap();
            let kvnr = md.subject;
            if let Some(by_kvnr) = self.by_kvnr.get_mut(&kvnr) {
                by_kvnr.remove(&id);
            }
        }
    }
}

impl Inner {
    pub fn medication_dispense_get(
        &mut self,
        id: Id,
        kvnr: &Kvnr,
        agent: Agent,
    ) -> Result<&MedicationDispense, Error> {
        let Self {
            ref medication_dispenses,
            ref mut timeouts,
            ref mut audit_events,
            ..
        } = self;

        let timeouts = Rc::new(RefCell::new(timeouts));
        Self::logged(audit_events, timeouts.clone(), move |event_builder| {
            let md = match medication_dispenses.by_id.get(&id) {
                Some(md) => md,
                None => return Err(Error::NotFound(id)),
            };

            event_builder.agent(agent);
            event_builder.action(Action::Read);
            event_builder.sub_type(SubType::Read);
            event_builder.what(What::MedicationDispense(md.id.clone().unwrap()));
            event_builder.patient(kvnr.clone());
            event_builder.description(md.prescription_id.clone());
            event_builder.text(Text::MedicationDispenseGetOne);

            if &md.subject != kvnr {
                return Err(Error::Forbidden(id));
            }

            Ok(md)
        })
    }

    pub fn medication_dispense_iter<'a, F>(
        &'a mut self,
        kvnr: &'a Kvnr,
        agent: Agent,
        f: F,
    ) -> impl Iterator<Item = &'a MedicationDispense>
    where
        F: Fn(&MedicationDispense) -> bool,
    {
        let Self {
            ref medication_dispenses,
            ref mut timeouts,
            ref mut audit_events,
            ..
        } = self;

        lazy_static! {
            static ref EMPTY: HashSet<Id> = HashSet::new();
        }

        let mut event_builder = Self::audit_event_builder();
        event_builder.agent(agent);
        event_builder.action(Action::Read);
        event_builder.sub_type(SubType::Read);
        event_builder.what(What::MedicationDispenses);
        event_builder.patient(kvnr.clone());
        event_builder.text(Text::MedicationDispenseGetMany);
        event_builder.build(audit_events, timeouts, None);

        let items = match medication_dispenses.by_kvnr.get(&kvnr) {
            Some(items) => items,
            None => &EMPTY,
        };

        items.iter().filter_map(move |id| {
            let v = medication_dispenses.by_id.get(&id).unwrap();

            if f(v) {
                Some(v)
            } else {
                None
            }
        })
    }

    pub fn medication_dispense_delete_by_id(&mut self, id: &Id) {
        let Self {
            ref mut medication_dispenses,
            ..
        } = self;

        let medication_dispense = medication_dispenses.by_id.get(id).unwrap();

        medication_dispenses
            .by_prescription_id
            .remove(&medication_dispense.prescription_id);
        if let Some(ids) = medication_dispenses
            .by_kvnr
            .get_mut(&medication_dispense.subject)
        {
            ids.remove(id);
        }

        medication_dispenses.by_id.remove(id);
    }
}
