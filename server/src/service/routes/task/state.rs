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
use std::collections::hash_map::{Entry, HashMap};
use std::convert::TryInto;
use std::ops::{Add, Deref};
use std::rc::Rc;

use chrono::{DateTime, Duration, Utc};
use rand::{distributions::Standard, rngs::OsRng, Rng};
use resources::{
    audit_event::{Action, Agent, SubType},
    erx_bundle::{Entry as ErxEntry, ErxBundle},
    misc::{Kvnr, PrescriptionId, TelematikId},
    primitives::Id,
    task::{Extension, Identifier, Status, Task, TaskCreateParameters},
    types::{FlowType, PerformerType},
    AuditEvent, ErxComposition, KbvBinary, KbvBundle, MedicationDispense, SignatureType,
};

use crate::{
    fhir::security::Signed,
    service::{header::XAccessCode, misc::DEVICE},
    state::{Inner, Version},
};

use super::Error;

impl Inner {
    pub fn task_create(&mut self, args: TaskCreateParameters) -> Result<&Version<Task>, Error> {
        let id = Id::generate().unwrap();
        let flow_type = args.flow_type;
        let access_code = random_id();
        let prescription_id =
            PrescriptionId::generate(flow_type).map_err(|()| Error::GeneratePrescriptionId)?;

        let task = Task {
            id: Some(id.clone()),
            extension: Extension {
                accept_date: None,
                expiry_date: None,
                flow_type,
            },
            identifier: Identifier {
                access_code: Some(access_code),
                prescription_id: Some(prescription_id),
                ..Default::default()
            },
            status: Status::Draft,
            for_: None,
            authored_on: Some(Utc::now().to_rfc3339().try_into().unwrap()),
            last_modified: Some(Utc::now().to_rfc3339().try_into().unwrap()),
            performer_type: vec![PerformerType::PublicPharmacy],
            input: Default::default(),
            output: Default::default(),
        };

        let task_meta = match self.tasks.entry(id) {
            Entry::Occupied(e) => panic!("Task does already exists: {}", e.key()),
            Entry::Vacant(e) => e.insert(task.into()),
        };

        let task = task_meta.history.get_current();

        Ok(task)
    }

    pub fn task_activate(
        &mut self,
        id: Id,
        access_code: XAccessCode,
        signing_time: DateTime<Utc>,
        kbv_binary: KbvBinary,
        kbv_bundle: KbvBundle,
        agent: Agent,
    ) -> Result<&Version<Task>, Error> {
        let Self {
            ref sig_key,
            ref sig_cert,
            ref mut tasks,
            ref mut audit_events,
            ref mut e_prescriptions,
            ref mut patient_receipts,
            ..
        } = self;

        Self::logged(audit_events, move |event_builder| {
            let kvnr: Kvnr = match kbv_bundle
                .entry
                .patient
                .as_ref()
                .and_then(|(_url, patient)| patient.identifier.as_ref())
                .map(Clone::clone)
                .map(TryInto::try_into)
            {
                Some(Ok(kvnr)) => kvnr,
                Some(Err(())) => return Err(Error::KvnrInvalid),
                None => return Err(Error::KvnrMissing),
            };

            let task_meta = match tasks.get_mut(&id) {
                Some(task_meta) => task_meta,
                None => return Err(Error::NotFound(id)),
            };

            let task = task_meta.history.get();
            event_builder.agent(agent);
            event_builder.action(Action::Create);
            event_builder.sub_type(SubType::Create);
            event_builder.patient(kvnr.clone());
            event_builder.what(format!("Task/{}", &id));
            event_builder.description_opt(task.identifier.prescription_id.clone());
            event_builder.text("/Task/$activate Operation");

            /* validate request */

            match &task.identifier.access_code {
                Some(s) if *s == access_code => (),
                Some(_) | None => return Err(Error::Forbidden(id)),
            }

            if Status::Draft != task.status {
                return Err(Error::InvalidStatus);
            }

            if e_prescriptions.contains_key(&kbv_bundle.id) {
                return Err(Error::EPrescriptionAlreadyRegistered(kbv_bundle.id));
            }

            /* create / update resources */

            let mut patient_receipt = kbv_bundle.clone();
            patient_receipt.id = Id::generate().unwrap();

            let patient_receipt = match patient_receipts.entry(patient_receipt.id.clone()) {
                Entry::Occupied(_) => {
                    panic!(
                        "Patient receipt with this ID ({}) already exists!",
                        patient_receipt.id
                    );
                }
                Entry::Vacant(entry) => {
                    let mut patient_receipt = Signed::new(patient_receipt);
                    patient_receipt.sign_json(
                        SignatureType::AuthorsSignature,
                        "Device/software".into(),
                        &sig_key,
                        &sig_cert,
                    )?;

                    entry.insert(patient_receipt).id.clone()
                }
            };

            let e_prescription = match e_prescriptions.entry(kbv_bundle.id.clone()) {
                Entry::Occupied(_) => {
                    panic!(
                        "ePrescription with this ID ({}) does already exist!",
                        kbv_bundle.id
                    );
                }
                Entry::Vacant(entry) => {
                    let id = entry.key().clone();
                    entry.insert(kbv_binary);

                    id
                }
            };

            let task_meta = match tasks.get_mut(&id) {
                Some(task_meta) => task_meta,
                None => return Err(Error::NotFound(id)),
            };

            let mut task = task_meta.history.get_mut();
            task.for_ = Some(kvnr);
            task.status = Status::Ready;
            task.input.e_prescription = Some(e_prescription);
            task.input.patient_receipt = Some(patient_receipt);

            let (accept_duration, expiry_duration) = match task.extension.flow_type {
                FlowType::ApothekenpflichtigeArzneimittel => {
                    (Duration::days(30), Duration::days(92))
                }
                _ => unimplemented!(),
            };
            task.extension.accept_date = Some(signing_time.add(accept_duration).date().into());
            task.extension.expiry_date = Some(signing_time.add(expiry_duration).date().into());

            let task = task_meta.history.get_current();

            Ok(task)
        })
    }

    pub fn task_accept(
        &mut self,
        id: Id,
        access_code: XAccessCode,
        agent: Agent,
    ) -> Result<(&Version<Task>, &KbvBinary), Error> {
        let Self {
            ref mut tasks,
            ref mut audit_events,
            ref mut e_prescriptions,
            ..
        } = self;

        Self::logged(audit_events, move |event_builder| {
            let mut task_meta = match tasks.get_mut(&id) {
                Some(task_meta) => task_meta,
                None => return Err(Error::NotFound(id)),
            };

            let task = task_meta.history.get();
            event_builder.agent(agent);
            event_builder.action(Action::Update);
            event_builder.sub_type(SubType::Update);
            event_builder.what(format!("Task/{}", &id));
            event_builder.patient_opt(task.for_.clone());
            event_builder.description_opt(task.identifier.prescription_id.clone());
            event_builder.text("/Task/$accept Operation");

            match &task.identifier.access_code {
                Some(ac) if ac == &access_code.0 => (),
                Some(_) => return Err(Error::Forbidden(id)),
                None => return Err(Error::Gone(id)),
            }

            match task.status {
                Status::Completed | Status::InProgress | Status::Draft => {
                    return Err(Error::Conflict(id))
                }
                Status::Cancelled => return Err(Error::Gone(id)),
                _ => (),
            }

            task_meta.accept_timestamp = Some(Utc::now());

            let mut task = task_meta.history.get_mut();
            task.status = Status::InProgress;
            task.identifier.secret = Some(random_id());

            let e_prescription = task
                .input
                .e_prescription
                .as_ref()
                .ok_or(Error::EPrescriptionMissing)?
                .clone();
            let e_prescription = e_prescriptions
                .get(&e_prescription)
                .ok_or(Error::EPrescriptionNotFound(e_prescription))?;

            let task = task_meta.history.get_current();

            Ok((task, e_prescription))
        })
    }

    pub fn task_reject(
        &mut self,
        id: Id,
        secret: Option<String>,
        agent: Agent,
    ) -> Result<(), Error> {
        let Self {
            ref mut tasks,
            ref mut audit_events,
            ..
        } = self;

        Self::logged(audit_events, move |event_builder| {
            let mut task_meta = match tasks.get_mut(&id) {
                Some(task_meta) => task_meta,
                None => return Err(Error::NotFound(id)),
            };

            let task = task_meta.history.get();
            event_builder.agent(agent);
            event_builder.action(Action::Update);
            event_builder.sub_type(SubType::Update);
            event_builder.what(format!("Task/{}", &id));
            event_builder.patient_opt(task.for_.clone());
            event_builder.description_opt(task.identifier.prescription_id.clone());
            event_builder.text("/Task/$reject Operation");

            if task.status != Status::InProgress || task.identifier.secret != secret {
                return Err(Error::Forbidden(id));
            }

            let mut task = task_meta.history.get_mut();
            task.status = Status::Ready;
            task.identifier.secret = None;

            task_meta.accept_timestamp = None;

            Ok(())
        })
    }

    pub fn task_close(
        &mut self,
        id: Id,
        secret: Option<String>,
        performer: TelematikId,
        mut medication_dispense: MedicationDispense,
        agent: Agent,
    ) -> Result<&ErxBundle, Error> {
        let Self {
            ref sig_key,
            ref sig_cert,

            ref mut tasks,
            ref mut erx_receipts,
            ref mut communications,
            ref mut audit_events,
            ref mut medication_dispenses,
            ..
        } = self;

        Self::logged(audit_events, move |event_builder| {
            let task_meta = match tasks.get_mut(&id) {
                Some(task_meta) => task_meta,
                None => return Err(Error::NotFound(id)),
            };

            let task = task_meta.history.get();
            event_builder.agent(agent);
            event_builder.action(Action::Update);
            event_builder.sub_type(SubType::Update);
            event_builder.what(format!("Task/{}", &id));
            event_builder.patient_opt(task.for_.clone());
            event_builder.description_opt(task.identifier.prescription_id.clone());
            event_builder.text("/Task/$close Operation");

            /* check the preconditions */

            if task.status != Status::InProgress || task.identifier.secret != secret {
                return Err(Error::Forbidden(id));
            }

            let prescription_id = task
                .identifier
                .prescription_id
                .as_ref()
                .ok_or(Error::EPrescriptionMissing)?;
            if &medication_dispense.prescription_id != prescription_id {
                return Err(Error::EPrescriptionMismatch);
            }

            let subject = task.for_.as_ref().ok_or(Error::SubjectMissing)?;
            if &medication_dispense.subject != subject {
                return Err(Error::SubjectMismatch);
            }

            if medication_dispense.performer != performer {
                return Err(Error::PerformerMismatch);
            }

            /* create erx bundle */

            let now = Utc::now();
            let erx_bundle = ErxBundle {
                id: Id::generate().unwrap(),
                identifier: prescription_id.clone(),
                timestamp: Utc::now().into(),
                entry: ErxEntry {
                    composition: Some(ErxComposition {
                        beneficiary: performer,
                        date: now.clone().into(),
                        author: DEVICE.id.clone().into(),
                        event_start: task_meta
                            .accept_timestamp
                            .ok_or(Error::AcceptTimestampMissing)?
                            .into(),
                        event_end: now.into(),
                    }),
                    device: Some(DEVICE.clone()),
                },
                signature: vec![],
            };

            medication_dispense.id = Some(Id::generate().unwrap());
            medication_dispense.supporting_information = vec![format!("/Task/{}", id)];

            /* add new resources to state */

            let med_dis_id = medication_dispense.id.as_ref().unwrap().clone();
            match medication_dispenses.entry(med_dis_id) {
                Entry::Occupied(_) => {
                    panic!(
                        "Medication dispense with this ID ({}) already exists!",
                        medication_dispense.id.unwrap()
                    );
                }
                Entry::Vacant(entry) => {
                    entry.insert(medication_dispense);
                }
            };

            let mut erx_bundle = Signed::new(erx_bundle);
            erx_bundle.sign_cades(
                SignatureType::AuthorsSignature,
                "Device/software".into(),
                &sig_key,
                &sig_cert,
            )?;

            let erx_bundle = match erx_receipts.entry(erx_bundle.id.clone()) {
                Entry::Occupied(_) => {
                    panic!("ErxBundle with this ID ({}) already exists!", erx_bundle.id);
                }
                Entry::Vacant(entry) => entry.insert(erx_bundle),
            };

            /* update task */

            let task = task_meta.history.get_mut();
            task.status = Status::Completed;
            task.output.receipt = Some(erx_bundle.id.clone());

            /* remove communications associated to this task */

            let task_id = &id;
            communications.retain(|_, c| {
                let based_on = c.based_on();
                let (id, _) = Self::parse_task_url(&based_on).unwrap();

                &id != task_id
            });

            Ok(&**erx_bundle)
        })
    }

    pub fn task_abort(
        &mut self,
        id: Id,
        kvnr: Option<Kvnr>,
        access_code: Option<XAccessCode>,
        is_pharmacy: bool,
        secret: Option<String>,
        agent: Agent,
    ) -> Result<(), Error> {
        let Self {
            ref mut tasks,
            ref mut erx_receipts,
            ref mut e_prescriptions,
            ref mut patient_receipts,
            ref mut audit_events,
            ref mut medication_dispenses,
            ..
        } = self;

        Self::logged(audit_events, move |event_builder| {
            let task_meta = match tasks.get_mut(&id) {
                Some(task_meta) => task_meta,
                None => return Err(Error::NotFound(id)),
            };

            let task = task_meta.history.get_current();
            event_builder.agent(agent);
            event_builder.action(Action::Delete);
            event_builder.sub_type(SubType::Delete);
            event_builder.what(format!("Task/{}", &id));
            event_builder.patient_opt(task.for_.clone());
            event_builder.description_opt(task.identifier.prescription_id.clone());
            event_builder.text("/Task/$abort Operation");

            let is_secret_ok = secret.is_some() && task.identifier.secret == secret;
            let is_access_ok = Self::task_matches(&task, &kvnr, &access_code, &None);
            let is_in_progress = task.status == Status::InProgress;

            if (is_pharmacy && !is_secret_ok) || (!is_pharmacy && !is_access_ok) {
                return Err(Error::Forbidden(id));
            }

            if is_pharmacy != is_in_progress {
                return Err(Error::Forbidden(id));
            }

            let mut task = task_meta.history.get_mut();
            task.for_ = None;
            task.status = Status::Cancelled;
            task.identifier.secret = None;
            task.identifier.access_code = None;
            task.last_modified = Some(Utc::now().into());

            let prescription_id = task
                .identifier
                .prescription_id
                .as_ref()
                .ok_or(Error::EPrescriptionMissing)?;

            medication_dispenses.retain(|_, md| &md.prescription_id != prescription_id);

            if let Some(e_prescription) = task.input.e_prescription.take() {
                e_prescriptions
                    .remove(&e_prescription)
                    .expect("ePrescription not found!");
            }

            if let Some(patient_receipt) = task.input.patient_receipt.take() {
                patient_receipts
                    .remove(&patient_receipt)
                    .expect("Patient Receipt not found!");
            }

            if let Some(receipt) = task.output.receipt.take() {
                erx_receipts
                    .remove(&receipt)
                    .expect("ErxReceipt not found!");
            }

            task_meta.history.clear();

            Ok(())
        })
    }

    pub fn task_get(
        &mut self,
        id: Id,
        version_id: Option<usize>,
        kvnr: Option<Kvnr>,
        access_code: Option<XAccessCode>,
        secret: Option<String>,
        agent: Agent,
    ) -> Result<(&Self, &Version<Task>), Error> {
        let Self {
            ref tasks,
            ref mut audit_events,
            ..
        } = self;

        let ret = Self::logged(audit_events, move |event_builder| {
            let task_meta = match tasks.get(&id) {
                Some(task_meta) => task_meta,
                None => return Err(Error::NotFound(id)),
            };

            let task = task_meta.history.get_current();
            event_builder.agent(agent);
            event_builder.action(Action::Read);
            event_builder.sub_type(if version_id.is_some() {
                SubType::VRead
            } else {
                SubType::Read
            });
            event_builder.what(format!("Task/{}", &id));
            event_builder.patient_opt(task.for_.clone());
            event_builder.description_opt(task.identifier.prescription_id.clone());

            if !Self::task_matches(&task, &kvnr, &access_code, &secret) {
                return Err(Error::Forbidden(id));
            }

            match version_id {
                Some(version_id) => match task_meta.history.get_version(version_id) {
                    Some(task) => Ok(task),
                    None => Err(Error::Gone(id)),
                },
                None => Ok(task),
            }
        });

        match ret {
            Ok(ref task) => Ok((&*self, task)),
            Err(err) => Err(err),
        }
    }

    pub fn task_iter<F>(
        &mut self,
        kvnr: Option<Kvnr>,
        access_code: Option<XAccessCode>,
        agent: Agent,
        mut f: F,
    ) -> impl Iterator<Item = TaskRef>
    where
        F: FnMut(&Task) -> bool,
    {
        let Self {
            ref tasks,
            ref mut audit_events,
            ..
        } = self;

        let iter = tasks.iter().filter_map(move |(_, task_meta)| {
            let task = task_meta.history.get_current();

            if !Self::task_matches(&task, &kvnr, &access_code, &None) {
                return None;
            }

            if !f(&task) {
                return None;
            }

            Some(task)
        });

        TaskIter {
            iter,
            audit_events: AuditEventsRef::new(agent, audit_events),
        }
    }

    pub fn task_matches(
        task: &Task,
        kvnr: &Option<Kvnr>,
        access_code: &Option<XAccessCode>,
        secret: &Option<String>,
    ) -> bool {
        match (task.for_.as_ref(), kvnr) {
            (Some(task_kvnr), Some(kvnr)) if task_kvnr == kvnr => return true,
            _ => (),
        }

        match (task.identifier.access_code.as_ref(), access_code) {
            (Some(task_ac), Some(ac)) if task_ac == ac => return true,
            _ => (),
        }

        match (task.identifier.secret.as_ref(), secret) {
            (Some(task_secret), Some(secret)) if task_secret == secret => return true,
            _ => (),
        }

        false
    }
}

#[derive(Clone)]
struct AuditEventsRef<'a> {
    agent: Agent,
    audit_events: Rc<RefCell<&'a mut HashMap<Kvnr, Vec<AuditEvent>>>>,
}

impl<'a> AuditEventsRef<'a> {
    fn new(agent: Agent, audit_events: &'a mut HashMap<Kvnr, Vec<AuditEvent>>) -> Self {
        Self {
            agent,
            audit_events: Rc::new(RefCell::new(audit_events)),
        }
    }
}

struct TaskIter<'a, T> {
    iter: T,
    audit_events: AuditEventsRef<'a>,
}

impl<'a, T> Iterator for TaskIter<'a, T>
where
    T: Iterator<Item = &'a Version<Task>>,
{
    type Item = TaskRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let task = self.iter.next()?;

        Some(TaskRef {
            task,
            audit_events: self.audit_events.clone(),
        })
    }
}

pub struct TaskRef<'a> {
    task: &'a Version<Task>,
    audit_events: AuditEventsRef<'a>,
}

impl<'a> TaskRef<'a> {
    pub fn unlogged(&self) -> &'a Version<Task> {
        self.task
    }
}

impl<'a> Deref for TaskRef<'a> {
    type Target = Version<Task>;

    fn deref(&self) -> &Self::Target {
        let task = self.task;

        let agent = self.audit_events.agent.clone();
        let mut audit_events = self.audit_events.audit_events.borrow_mut();

        let mut builder = Inner::audit_event_builder();
        builder.agent(agent);
        builder.action(Action::Read);
        builder.sub_type(SubType::Read);
        builder.what(format!("Task/{}", &task.id));
        builder.patient_opt(task.for_.clone());
        builder.description_opt(task.identifier.prescription_id.clone());
        builder.build(&mut audit_events, None);

        task
    }
}

fn random_id() -> String {
    OsRng
        .sample_iter(&Standard)
        .take(32)
        .map(|x: u8| format!("{:02x}", x))
        .collect::<Vec<_>>()
        .join("")
}
