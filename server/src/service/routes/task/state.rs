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
use std::ops::Add;
use std::rc::Rc;

use bdays::easter::easter_naive_date;
use chrono::{Date, DateTime, Datelike, Duration, Utc, Weekday};
use rand::{distributions::Standard, rngs::OsRng, Rng};
use resources::{
    audit_event::{Action, Agent, SubType, Text, What},
    composition::LegalBasis,
    erx_bundle::{Entry as ErxEntry, ErxBundle},
    misc::{Kvnr, PrescriptionId, TelematikId},
    primitives::Id,
    task::{Extension, Identifier, Status, Task, TaskCreateParameters},
    types::{FlowType, PerformerType},
    ErxComposition, KbvBinary, KbvBundle, MedicationDispense,
};

use crate::{
    service::{
        header::XAccessCode, misc::DEVICE, AuditEventBuilder, Loggable, LoggedIter, LoggedRef,
    },
    state::{History, Inner, Version},
};

use super::Error;

#[derive(Default)]
pub struct Tasks {
    by_id: HashMap<Id, TaskMeta>,
}

impl Tasks {
    pub fn insert_task(&mut self, task: Task) {
        let task_meta = task.into();
        self.insert_task_meta(task_meta);
    }

    pub fn insert_task_meta(&mut self, task_meta: TaskMeta) {
        let id = task_meta.history.get_current().resource.id.clone();

        match self.by_id.entry(id) {
            Entry::Occupied(e) => {
                panic!("Task with this ID ({}) does already exist!", e.key());
            }
            Entry::Vacant(entry) => {
                entry.insert(task_meta);
            }
        }
    }

    pub fn get_by_id(&self, id: &Id) -> Option<&TaskMeta> {
        self.by_id.get(id)
    }

    pub fn get_mut_by_id(&mut self, id: &Id) -> Option<&mut TaskMeta> {
        self.by_id.get_mut(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &TaskMeta> {
        self.by_id.values()
    }
}

pub struct TaskMeta {
    pub history: History<Task>,
    pub accept_timestamp: Option<DateTime<Utc>>,
    pub communication_count: usize,
}

impl From<Task> for TaskMeta {
    fn from(task: Task) -> Self {
        Self {
            history: History::new(task),
            accept_timestamp: None,
            communication_count: 0,
        }
    }
}

impl Inner {
    pub fn task_create(&mut self, args: TaskCreateParameters) -> Result<&Version<Task>, Error> {
        let id = Id::generate().unwrap();
        let flow_type = args.flow_type;
        let access_code = random_id();
        let prescription_id =
            PrescriptionId::generate(flow_type).map_err(|()| Error::GeneratePrescriptionId)?;

        let task = Task {
            id: id.clone(),
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

        let task_meta = match self.tasks.by_id.entry(id) {
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
            ref mut tasks,
            ref mut audit_events,
            ref mut e_prescriptions,
            ref mut patient_receipts,
            ref mut timeouts,
            ..
        } = self;

        let timeouts = Rc::new(RefCell::new(timeouts));
        Self::logged(audit_events, timeouts.clone(), move |event_builder| {
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

            let task_meta = match tasks.get_mut_by_id(&id) {
                Some(task_meta) => task_meta,
                None => return Err(Error::NotFound(id)),
            };

            let task = task_meta.history.get();
            event_builder.agent(agent);
            event_builder.action(Action::Create);
            event_builder.sub_type(SubType::Create);
            event_builder.patient(kvnr.clone());
            event_builder.what(What::Task(id.clone()));
            event_builder.description_opt(task.identifier.prescription_id.clone());
            event_builder.text(Text::TaskActivate);

            /* validate request */

            match &task.identifier.access_code {
                Some(s) if *s == access_code => (),
                Some(_) | None => return Err(Error::Forbidden(id)),
            }

            if Status::Draft != task.status {
                return Err(Error::InvalidStatus);
            }

            if e_prescriptions.contains(&kbv_bundle.id) {
                return Err(Error::EPrescriptionAlreadyRegistered(kbv_bundle.id));
            }

            /* create / update resources */

            let mut patient_receipt = kbv_bundle.clone();
            let patient_receipt_id = Id::generate().unwrap();
            patient_receipt.id = patient_receipt_id.clone();
            patient_receipts.insert_kbv_bundle(patient_receipt)?;

            let e_prescription_id = kbv_bundle.id.clone();
            e_prescriptions.insert(e_prescription_id.clone(), kbv_binary);

            let mut task = task_meta.history.get_mut();
            task.for_ = Some(kvnr);
            task.status = Status::Ready;
            task.input.e_prescription = Some(e_prescription_id);
            task.input.patient_receipt = Some(patient_receipt_id);

            let (accept_duration, expiry_duration) = match task.extension.flow_type {
                FlowType::ApothekenpflichtigeArzneimittel => {
                    (Duration::days(30), Duration::days(92))
                }
                _ => unimplemented!(),
            };

            task.extension.accept_date = Some(signing_time.add(accept_duration).date().into());
            task.extension.expiry_date = Some(signing_time.add(expiry_duration).date().into());

            if let Some(LegalBasis::DischargeManagement) = kbv_bundle
                .entry
                .composition
                .as_ref()
                .and_then(|(_, c)| c.extension.legal_basis.as_ref())
            {
                let mut date = signing_time.date();

                for _ in 0..3 {
                    date = date.add(Duration::days(1));
                    while is_holiday(&date) {
                        date = date.add(Duration::days(1));
                    }
                }

                task.extension.accept_date = Some(date.into());
            }

            let task = task_meta.history.get_current();
            timeouts.borrow_mut().insert(&**task);

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
            ref mut timeouts,
            ..
        } = self;

        let timeouts = Rc::new(RefCell::new(timeouts));
        Self::logged(audit_events, timeouts.clone(), move |event_builder| {
            let mut task_meta = match tasks.by_id.get_mut(&id) {
                Some(task_meta) => task_meta,
                None => return Err(Error::NotFound(id)),
            };

            let task = task_meta.history.get();
            event_builder.agent(agent);
            event_builder.action(Action::Update);
            event_builder.sub_type(SubType::Update);
            event_builder.what(What::Task(id.clone()));
            event_builder.patient_opt(task.for_.clone());
            event_builder.description_opt(task.identifier.prescription_id.clone());
            event_builder.text(Text::TaskAccept);

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
                .get_by_id(&e_prescription)
                .ok_or(Error::EPrescriptionNotFound(e_prescription))?;

            timeouts.borrow_mut().insert(&*task);

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
            ref mut timeouts,
            ..
        } = self;

        let timeouts = Rc::new(RefCell::new(timeouts));
        Self::logged(audit_events, timeouts.clone(), move |event_builder| {
            let mut task_meta = match tasks.by_id.get_mut(&id) {
                Some(task_meta) => task_meta,
                None => return Err(Error::NotFound(id)),
            };

            let task = task_meta.history.get();
            event_builder.agent(agent);
            event_builder.action(Action::Update);
            event_builder.sub_type(SubType::Update);
            event_builder.what(What::Task(id.clone()));
            event_builder.patient_opt(task.for_.clone());
            event_builder.description_opt(task.identifier.prescription_id.clone());
            event_builder.text(Text::TaskReject);

            if task.status != Status::InProgress || task.identifier.secret != secret {
                return Err(Error::Forbidden(id));
            }

            let mut task = task_meta.history.get_mut();
            task.status = Status::Ready;
            task.identifier.secret = None;

            timeouts.borrow_mut().insert(&*task);

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
            ref mut tasks,
            ref mut erx_receipts,
            ref mut communications,
            ref mut audit_events,
            ref mut medication_dispenses,
            ref mut timeouts,
            ..
        } = self;

        let timeouts = Rc::new(RefCell::new(timeouts));
        Self::logged(audit_events, timeouts.clone(), move |event_builder| {
            let task_meta = match tasks.by_id.get_mut(&id) {
                Some(task_meta) => task_meta,
                None => return Err(Error::NotFound(id)),
            };

            let task = task_meta.history.get();
            event_builder.agent(agent);
            event_builder.action(Action::Update);
            event_builder.sub_type(SubType::Update);
            event_builder.what(What::Task(id.clone()));
            event_builder.patient_opt(task.for_.clone());
            event_builder.description_opt(task.identifier.prescription_id.clone());
            event_builder.text(Text::TaskClose);

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
                        id: Id::generate().unwrap(),
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

            timeouts.borrow_mut().insert(&medication_dispense);

            medication_dispenses.insert(medication_dispense);
            let erx_bundle = erx_receipts.insert_erx_bundle(erx_bundle)?;

            /* update task */

            let task = task_meta.history.get_mut();
            task.status = Status::Completed;
            task.output.receipt = Some(erx_bundle.id.clone());

            timeouts.borrow_mut().insert(&*task);

            /* remove communications associated to this task */
            communications.remove_by_task_id(&id);

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
            ref mut timeouts,
            ..
        } = self;

        let timeouts = Rc::new(RefCell::new(timeouts));
        Self::logged(audit_events, timeouts.clone(), move |event_builder| {
            let task_meta = match tasks.by_id.get_mut(&id) {
                Some(task_meta) => task_meta,
                None => return Err(Error::NotFound(id)),
            };

            let task = task_meta.history.get_current();
            event_builder.agent(agent);
            event_builder.action(Action::Delete);
            event_builder.sub_type(SubType::Delete);
            event_builder.what(What::Task(id.clone()));
            event_builder.patient_opt(task.for_.clone());
            event_builder.description_opt(task.identifier.prescription_id.clone());
            event_builder.text(if is_pharmacy {
                Text::TaskAbortPharmacy
            } else if kvnr.is_none() {
                Text::TaskAbortDoctor
            } else if task.for_ != kvnr {
                Text::TaskAbortRepresentative
            } else {
                Text::TaskAbortPatient
            });

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

            medication_dispenses.remove_by_prescription_id(prescription_id);

            if let Some(e_prescription) = task.input.e_prescription.take() {
                e_prescriptions.remove_by_id(&e_prescription);
            }

            if let Some(patient_receipt) = task.input.patient_receipt.take() {
                patient_receipts.remove_by_id(&patient_receipt);
            }

            if let Some(receipt) = task.output.receipt.take() {
                erx_receipts.remove_by_id(&receipt);
            }

            timeouts.borrow_mut().insert(&*task);

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
            ref mut timeouts,
            ref mut audit_events,
            ..
        } = self;

        let timeouts = Rc::new(RefCell::new(timeouts));
        let ret = Self::logged(audit_events, timeouts.clone(), move |event_builder| {
            let task_meta = match tasks.by_id.get(&id) {
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
            event_builder.what(What::Task(id.clone()));
            event_builder.patient_opt(task.for_.clone());
            event_builder.description_opt(task.identifier.prescription_id.clone());
            event_builder.text(if secret.is_some() {
                Text::TaskGetPharmacy
            } else if task.for_ != kvnr {
                Text::TaskGetRepresentative
            } else {
                Text::TaskGetPatient
            });

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
    ) -> impl Iterator<Item = LoggedRef<(&Version<Task>, Text)>>
    where
        F: FnMut(&Task) -> bool,
    {
        let Self {
            ref tasks,
            ref mut timeouts,
            ref mut audit_events,
            ..
        } = self;

        let iter = tasks.by_id.iter().filter_map(move |(_, task_meta)| {
            let task = task_meta.history.get_current();

            if !Self::task_matches(&task, &kvnr, &access_code, &None) {
                return None;
            }

            if !f(&task) {
                return None;
            }

            let text = if task.for_ == kvnr {
                Text::TaskGetPatient
            } else {
                Text::TaskGetRepresentative
            };

            Some((task, text))
        });

        LoggedIter::new(audit_events, timeouts, agent, iter)
    }

    pub fn task_delete_by_id(&mut self, id: &Id) {
        let Self {
            ref mut tasks,
            ref mut timeouts,
            ref mut audit_events,
            ref mut erx_receipts,
            ref mut e_prescriptions,
            ref mut patient_receipts,
            ref mut medication_dispenses,
            ..
        } = self;

        let timeouts = Rc::new(RefCell::new(timeouts));
        Self::logged::<_, (), String>(audit_events, timeouts.clone(), move |event_builder| {
            let task = tasks.by_id.get(id).unwrap();
            let task = task.history.get_current();

            event_builder.agent(Self::agent().clone());
            event_builder.action(Action::Delete);
            event_builder.sub_type(SubType::Delete);
            event_builder.what(What::Task(id.clone()));
            event_builder.patient_opt(task.for_.clone());
            event_builder.description_opt(task.identifier.prescription_id.clone());
            event_builder.text(Text::TaskDelete);

            if let Some(prescription_id) = &task.identifier.prescription_id {
                medication_dispenses.remove_by_prescription_id(prescription_id);
            }

            if let Some(e_prescription) = &task.input.e_prescription {
                e_prescriptions.remove_by_id(e_prescription);
            }

            if let Some(patient_receipt) = &task.input.patient_receipt {
                patient_receipts.remove_by_id(&patient_receipt);
            }

            if let Some(receipt) = &task.output.receipt {
                erx_receipts.remove_by_id(&receipt);
            }

            tasks.by_id.remove(id);

            Ok(())
        })
        .unwrap();
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

impl<'a> Loggable for (&'a Version<Task>, Text) {
    type Item = &'a Version<Task>;

    fn unlogged(&self) -> &Self::Item {
        &self.0
    }

    fn logged(&self, builder: &mut AuditEventBuilder) -> &Self::Item {
        builder.what(What::Task(self.0.resource.id.clone()));
        builder.patient_opt(self.0.for_.clone());
        builder.description_opt(self.0.identifier.prescription_id.clone());
        builder.text(self.1.clone());

        &self.0
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

fn is_holiday(date: &Date<Utc>) -> bool {
    let easter_sunday = easter_naive_date(date.year()).unwrap();
    let easter_sunday = Date::<Utc>::from_utc(easter_sunday, Utc);
    let easter_monday = easter_sunday + Duration::days(1);
    let easter_friday = easter_sunday - Duration::days(2);
    let whitsun_monday = easter_sunday + Duration::days(50);
    let ascension = easter_sunday + Duration::days(39);

    date.weekday() == Weekday::Sun
    || (date.day() == 1 && date.month() == 1) // 01.01.
    || (date.day() == 1 && date.month() == 5) // 01.05
    || (date.day() == 3 && date.month() == 10) // 03.10
    || (date.day() == 25 && date.month() == 12) // 25.12
    || (date.day() == 26 && date.month() == 12) // 26.12
    || date == &easter_friday
    || date == &easter_monday
    || date == &whitsun_monday
    || date == &ascension
}
