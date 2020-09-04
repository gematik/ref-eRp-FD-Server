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

use base64::{decode, encode};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

#[serde(rename = "Binary")]
#[serde(rename_all = "camelCase")]
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct BinaryDef {
    pub content_type: String,

    #[serde(default)]
    #[serde(serialize_with = "to_base64")]
    #[serde(deserialize_with = "from_base64")]
    pub data: Option<Vec<u8>>,
}

fn to_base64<S: Serializer>(value: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error> {
    value.as_ref().map(encode).serialize(serializer)
}

fn from_base64<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error> {
    Option::<String>::deserialize(deserializer)?
        .map(decode)
        .transpose()
        .map_err(|err| D::Error::custom(format!("invalid base64 string: {}", err)))
}
