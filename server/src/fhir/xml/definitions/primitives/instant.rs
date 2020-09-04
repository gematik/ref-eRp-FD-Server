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

use chrono::DateTime;
use resources::primitives::Instant;
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize)]
#[serde(remote = "Instant")]
#[serde(rename = "value-tag=Instant")]
pub struct InstantDef(#[serde(getter = "get_string")] String);

impl<'de> InstantDef {
    pub fn deserialize<D>(deserializer: D) -> Result<Instant, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename = "value-tag=Instant")]
        struct Helper(String);

        let s = Helper::deserialize(deserializer)?.0;

        match DateTime::parse_from_rfc3339(&s) {
            Ok(date_time) => Ok(date_time.into()),
            Err(err) => Err(Error::custom(format!(
                "Invalid RCF 3339 / Iso 8601 format: {}",
                err
            ))),
        }
    }
}

fn get_string(v: &Instant) -> String {
    v.to_rfc3339()
}

make_option_def!(Instant, "InstantDef", OptionInstantDef, "value-tag=Instant");
