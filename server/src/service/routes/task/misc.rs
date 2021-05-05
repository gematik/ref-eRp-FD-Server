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

use std::convert::TryInto;

use resources::{
    audit_event::Language,
    primitives::{Id, Instant},
    AuditEvent, ErxBundle, KbvBinary, KbvBundle, Task,
};

use crate::{
    fhir::{
        definitions::{AsTask, AuditEventContainer, EncodeBundleResource, TaskContainer},
        encode::{DataStorage, Encode, EncodeError, EncodeStream},
    },
    state::Version,
};

#[derive(Clone)]
pub enum Resource<'a> {
    TaskForSupplier(&'a Version<Task>),
    TaskForPatient(&'a Version<Task>),
    KbvBinary(&'a KbvBinary),
    KbvBundle(&'a KbvBundle),
    ErxBundle(&'a ErxBundle),
    AuditEvent(&'a AuditEvent, Language),
}

impl AsTask for Version<Task> {
    fn task(&self) -> &Task {
        &self.resource
    }

    fn version_id(&self) -> Option<Id> {
        Some(self.id.to_string().try_into().unwrap())
    }

    fn last_updated(&self) -> Option<Instant> {
        Some(self.timestamp.into())
    }
}

impl EncodeBundleResource for Resource<'_> {}

impl Encode for Resource<'_> {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        match self {
            Self::TaskForSupplier(v) => TaskContainer::for_supplier(v).encode(stream),
            Self::TaskForPatient(v) => TaskContainer::for_patient(v).encode(stream),
            Self::KbvBinary(v) => v.encode(stream),
            Self::KbvBundle(v) => v.encode(stream),
            Self::ErxBundle(v) => v.encode(stream),
            Self::AuditEvent(audit_event, lang) => {
                AuditEventContainer { audit_event, lang }.encode(stream)
            }
        }
    }
}
