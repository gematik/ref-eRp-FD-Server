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

use std::ops::Deref;

use resources::primitives::{Id, Instant};
use serde::{Deserialize, Serialize};

use super::{
    super::primitives::{OptionIdDef, OptionInstantDef},
    CodingDef,
};

#[derive(Default, Serialize, Deserialize)]
pub struct MetaDef {
    #[serde(default)]
    #[serde(with = "OptionIdDef")]
    #[serde(alias = "versionId")]
    #[serde(rename = "value-tag=versionId")]
    pub version_id: Option<Id>,

    #[serde(default)]
    #[serde(with = "OptionInstantDef")]
    #[serde(alias = "lastUpdated")]
    #[serde(rename = "value-tag=lastUpdated")]
    pub last_updated: Option<Instant>,

    #[serde(default)]
    pub source: Vec<UriDef>,

    #[serde(default)]
    pub profile: Vec<UriDef>,

    #[serde(default)]
    pub security: Vec<CodingDef>,

    #[serde(default)]
    pub tag: Vec<CodingDef>,
}

#[derive(Serialize, Deserialize, PartialEq)]
#[serde(rename = "value-tag=Uri")]
pub struct UriDef(String);

impl From<&str> for UriDef {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl From<String> for UriDef {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl Into<String> for UriDef {
    fn into(self) -> String {
        self.0
    }
}

impl Deref for UriDef {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
