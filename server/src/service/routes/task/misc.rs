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

use rand::{distributions::Standard, thread_rng, Rng};
use resources::{KbvBinary, KbvBundle, Task};

use crate::fhir::{
    definitions::EncodeBundleResource,
    encode::{DataStorage, Encode, EncodeError, EncodeStream},
};

#[derive(Clone)]
pub enum Resource<'a> {
    Task(&'a Task),
    Binary(&'a KbvBinary),
    Bundle(&'a KbvBundle),
}

impl EncodeBundleResource for Resource<'_> {}

impl Encode for Resource<'_> {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        match self {
            Self::Task(v) => v.encode(stream),
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
