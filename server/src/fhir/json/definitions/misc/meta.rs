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

use resources::primitives::{Id, Instant};
use serde::{Deserialize, Serialize};

use super::{
    super::primitives::{OptionIdDef, OptionInstantDef},
    CodingDef,
};

#[serde(rename_all = "camelCase")]
#[derive(Default, Serialize, Deserialize)]
pub struct MetaDef {
    #[serde(default)]
    #[serde(with = "OptionIdDef")]
    pub version_id: Option<Id>,

    #[serde(default)]
    #[serde(with = "OptionInstantDef")]
    pub last_updated: Option<Instant>,

    #[serde(default)]
    pub source: Vec<String>,

    #[serde(default)]
    pub profile: Vec<String>,

    #[serde(default)]
    pub security: Vec<CodingDef>,

    #[serde(default)]
    pub tag: Vec<CodingDef>,
}
