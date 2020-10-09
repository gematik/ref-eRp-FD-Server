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

use std::convert::TryFrom;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::ops::Deref;
use std::time::Instant;

use rand::{thread_rng, Rng};
use regex::Regex;
use serde::{de::Error as DeError, Deserialize, Deserializer};
use uuid::{
    v1::{Context, Timestamp},
    Error, Uuid,
};

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Id(String);

impl Id {
    pub fn generate() -> Result<Self, ()> {
        let uuid = generate_uuid().map_err(|_| ())?;
        let id = Self(uuid.to_string());

        Ok(id)
    }
}

impl Deref for Id {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> TryFrom<&'a str> for Id {
    type Error = &'a str;

    fn try_from(v: &'a str) -> Result<Self, Self::Error> {
        if check_str(v) {
            Ok(Self(v.to_owned()))
        } else {
            Err(v)
        }
    }
}

impl TryFrom<String> for Id {
    type Error = String;

    fn try_from(v: String) -> Result<Self, Self::Error> {
        if check_str(&v) {
            Ok(Self(v))
        } else {
            Err(v)
        }
    }
}

impl From<Id> for String {
    fn from(v: Id) -> Self {
        v.0
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;

        Self::try_from(s).map_err(|err| D::Error::custom(format!("Invalid ID: {}", err)))
    }
}

fn check_str(s: &str) -> bool {
    lazy_static! {
        static ref RX: Regex = Regex::new(r#"^[A-Za-z0-9\-\.\\/]{1,64}$"#).unwrap();
    }

    RX.is_match(s)
}

fn generate_uuid() -> Result<Uuid, Error> {
    lazy_static! {
        static ref CONTEXT: Context = Context::new(42);
        static ref START_TIME: Instant = Instant::now();
        static ref UNIQUE_ID: [u8; 6] = thread_rng().gen();
    }

    let context: &Context = &CONTEXT;

    let ts = START_TIME.elapsed();
    let ts = Timestamp::from_unix(context, ts.as_secs(), ts.subsec_nanos());

    Uuid::new_v1(ts, &UNIQUE_ID[..])
}
