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

use std::convert::TryFrom;
use std::fmt::Display;
use std::ops::Deref;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Kvnr(String);

impl Kvnr {
    pub fn new<T: Display>(value: T) -> Result<Self, String> {
        let value = value.to_string();
        if value.len() == KVID_LEN {
            Ok(Self(value))
        } else {
            Err(format!("Invalid KV-Nr.: {}!", value))
        }
    }

    pub fn as_string(&self) -> &String {
        &self.0
    }
}

impl Into<String> for Kvnr {
    fn into(self) -> String {
        self.0
    }
}

impl TryFrom<String> for Kvnr {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() == KVID_LEN {
            Ok(Self(value))
        } else {
            Err(value)
        }
    }
}

impl Deref for Kvnr {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

const KVID_LEN: usize = 10;
