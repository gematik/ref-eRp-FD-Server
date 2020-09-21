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

use std::fmt::Display;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use regex::Regex;
use serde::de::{Deserialize, Deserializer, Error};

pub struct Search<T: Parameter> {
    value: T::Storage,
    comperator: Comperator,
}

pub trait Parameter: Sized {
    type Storage;
    type Error: Display;

    fn parse(s: &str) -> Result<Self::Storage, Self::Error>;

    fn compare(&self, comperator: Comperator, param: &Self::Storage) -> bool;
}

#[derive(Debug, Copy, Clone)]
pub enum Comperator {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterEqual,
    LessEqual,
    StartsAfter,
    EndsBefore,
    Approximately,
}

impl<T: Parameter> Search<T> {
    pub fn matches(&self, other: &T) -> bool {
        other.compare(self.comperator, &self.value)
    }
}

impl<T: Parameter> FromStr for Search<T> {
    type Err = T::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (comperator, s) = match s {
            s if s.starts_with("eq") => (Comperator::Equal, &s[2..]),
            s if s.starts_with("ne") => (Comperator::NotEqual, &s[2..]),
            s if s.starts_with("gt") => (Comperator::GreaterThan, &s[2..]),
            s if s.starts_with("lt") => (Comperator::LessThan, &s[2..]),
            s if s.starts_with("ge") => (Comperator::GreaterEqual, &s[2..]),
            s if s.starts_with("le") => (Comperator::LessEqual, &s[2..]),
            s if s.starts_with("sa") => (Comperator::StartsAfter, &s[2..]),
            s if s.starts_with("eb") => (Comperator::EndsBefore, &s[2..]),
            s if s.starts_with("ap") => (Comperator::Approximately, &s[2..]),
            s => (Comperator::Equal, s),
        };

        let value = T::parse(s)?;

        Ok(Self { value, comperator })
    }
}

impl<'de, T: Parameter> Deserialize<'de> for Search<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let ret = s.parse().map_err(|err| {
            D::Error::custom(format!("Unable to parse search parameter: {}", err))
        })?;

        Ok(ret)
    }
}

impl Parameter for String {
    type Storage = String;
    type Error = String;

    fn parse(s: &str) -> Result<Self, Self::Error> {
        Ok(s.to_owned())
    }

    fn compare(&self, comperator: Comperator, param: &Self::Storage) -> bool {
        match comperator {
            Comperator::Equal => self == param,
            Comperator::NotEqual => self != param,
            Comperator::GreaterThan => self > param,
            Comperator::LessThan => self < param,
            Comperator::GreaterEqual => self >= param,
            Comperator::LessEqual => self <= param,
            _ => false,
        }
    }
}

impl Parameter for DateTime<Utc> {
    type Storage = (DateTime<Utc>, &'static str);
    type Error = String;

    fn parse(s: &str) -> Result<Self::Storage, Self::Error> {
        lazy_static! {
            static ref RX: Regex = Regex::new(
                r#"^([0-9]{4})(-(0[1-9]|1[0-2])(-(0[1-9]|[1-2][0-9]|3[0-1])(T([01][0-9]|2[0-3])(:[0-5][0-9](:[0-5][0-9])?)?)?)?)?$"#
            )
            .unwrap();
        }

        if !RX.is_match(&s) {
            return Err("Invalid seach parameter: date time format!".into());
        }

        let (date, fmt) = match s {
            s if s.len() == 4 => (format!("{}-01-01T00:00:00Z", s), "%Y"),
            s if s.len() == 7 => (format!("{}-01T00:00:00Z", s), "%Y-%m"),
            s if s.len() == 10 => (format!("{}T00:00:00Z", s), "%Y-%m-%d"),
            s if s.len() == 13 => (format!("{}:00:00Z", s), "%Y-%m-%dT%H"),
            s if s.len() == 16 => (format!("{}:00Z", s), "%Y-%m-%dT%H:%M"),
            s if s.len() == 19 => (format!("{}Z", s), "%Y-%m-%dT%H:%M:%S"),
            _ => unreachable!(),
        };
        let date = date
            .parse()
            .map_err(|err| format!("Invalid seach parameter: {}", err))?;

        Ok((date, fmt))
    }

    fn compare(&self, comperator: Comperator, param: &Self::Storage) -> bool {
        let (param, fmt) = param;
        match comperator {
            Comperator::Equal => self.format(fmt).to_string() == param.format(fmt).to_string(),
            Comperator::NotEqual => self.format(fmt).to_string() != param.format(fmt).to_string(),
            Comperator::GreaterThan | Comperator::StartsAfter => {
                self > param && self.format(fmt).to_string() != param.format(fmt).to_string()
            }
            Comperator::LessThan | Comperator::EndsBefore => {
                self < param && self.format(fmt).to_string() != param.format(fmt).to_string()
            }
            Comperator::GreaterEqual => self >= param,
            Comperator::LessEqual => self <= param,
            Comperator::Approximately => false,
        }
    }
}
