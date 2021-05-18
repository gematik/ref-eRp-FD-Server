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

use serde::{Deserialize, Serialize};

use super::{
    misc::{Address, Kvnr, Name},
    primitives::{Date, Id},
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Patient {
    pub id: Id,
    pub identifier: Option<Identifier>,
    pub name: Name,
    pub birth_date: Date,
    pub address: Address,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Identifier {
    GKV {
        value: Kvnr,
    },
    PKV {
        system: Option<String>,
        value: String,
    },
    KVK {
        value: String,
    },
}

impl TryInto<Kvnr> for Identifier {
    type Error = ();

    fn try_into(self) -> Result<Kvnr, Self::Error> {
        match self {
            Self::GKV { value } => Ok(value),
            _ => Err(()),
        }
    }
}
