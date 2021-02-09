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
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::ops::Deref;

use chrono::{Date as ChronoDate, TimeZone};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Date(String);

impl<TZ> From<ChronoDate<TZ>> for Date
where
    TZ: TimeZone,
    <TZ as TimeZone>::Offset: Display,
{
    fn from(v: ChronoDate<TZ>) -> Self {
        Self(format!("{}", v.format("%Y-%m-%d")))
    }
}

impl TryFrom<&str> for Date {
    type Error = String;

    fn try_from(v: &str) -> Result<Self, Self::Error> {
        from_string(v.to_owned())
    }
}

impl TryFrom<String> for Date {
    type Error = String;

    fn try_from(v: String) -> Result<Self, Self::Error> {
        from_string(v)
    }
}

impl Deref for Date {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Date {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        self.0.fmt(f)
    }
}

fn from_string(s: String) -> Result<Date, String> {
    lazy_static! {
        static ref RX: Regex = Regex::new(
            r#"^([0-9]([0-9]([0-9][1-9]|[1-9]0)|[1-9]00)|[1-9]000)(-(0[1-9]|1[0-2])(-(0[1-9]|[1-2][0-9]|3[0-1]))?)?$"#
        )
        .unwrap();
    }

    if RX.is_match(&s) {
        Ok(Date(s))
    } else {
        Err(s)
    }
}
