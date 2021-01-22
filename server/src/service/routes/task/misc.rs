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

use rand::{distributions::Standard, thread_rng, Rng};
use resources::{
    primitives::{Id, Instant},
    KbvBinary, KbvBundle, Task,
};

use crate::{
    fhir::{
        definitions::{AsTask, EncodeBundleResource, TaskContainer},
        encode::{DataStorage, Encode, EncodeError, EncodeStream},
    },
    service::misc::Version,
};

#[derive(Clone)]
pub enum Resource<'a> {
    TaskVersion(&'a Version<Task>),
    Binary(&'a KbvBinary),
    Bundle(&'a KbvBundle),
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
            Self::TaskVersion(v) => TaskContainer(v).encode(stream),
            Self::Binary(v) => v.encode(stream),
            Self::Bundle(v) => v.encode(stream),
        }
    }
}

pub fn random_id() -> String {
    thread_rng()
        .sample_iter(&Standard)
        .take(32)
        .map(|x: u8| format!("{:02x}", x))
        .collect::<Vec<_>>()
        .join("")
}
