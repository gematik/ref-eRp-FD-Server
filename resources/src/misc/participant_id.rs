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

use serde::{Deserialize, Serialize};

use super::{Kvnr, TelematikId};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ParticipantId {
    Kvnr(Kvnr),
    TelematikId(TelematikId),
}

impl ParticipantId {
    pub fn kvnr(&self) -> Option<&Kvnr> {
        match self {
            Self::Kvnr(v) => Some(v),
            _ => None,
        }
    }

    pub fn telematik_id(&self) -> Option<&TelematikId> {
        match self {
            Self::TelematikId(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_string(&self) -> &String {
        match self {
            Self::Kvnr(v) => v.as_string(),
            Self::TelematikId(v) => v.as_string(),
        }
    }
}
