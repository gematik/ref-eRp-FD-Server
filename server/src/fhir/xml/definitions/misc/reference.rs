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

use serde::{Deserialize, Serialize};

use super::IdentifierDef;

#[serde(rename_all = "camelCase")]
#[derive(Default, Serialize, Deserialize)]
pub struct ReferenceDef {
    #[serde(default)]
    #[serde(alias = "reference")]
    #[serde(rename = "value-tag=reference")]
    pub reference: Option<String>,

    #[serde(default)]
    #[serde(alias = "type")]
    #[serde(rename = "value-tag=type")]
    pub type_: Option<String>,

    #[serde(default)]
    pub identifier: Option<IdentifierDef>,

    #[serde(default)]
    #[serde(alias = "display")]
    #[serde(rename = "value-tag=display")]
    pub display: Option<String>,
}
