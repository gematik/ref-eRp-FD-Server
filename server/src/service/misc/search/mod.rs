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

mod audit_event;
mod date_time;
mod option;
mod string;
mod task_status;
mod telematik_id;

use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::str::FromStr;

pub struct Search<T: Parameter> {
    args: Vec<(Comperator, T::Storage)>,
}

pub trait Parameter: Sized {
    type Storage;

    fn parse(s: &str) -> Result<Self::Storage, String>;

    fn compare(&self, comperator: Comperator, param: &Self::Storage) -> bool;
}

#[derive(Debug, Copy, Clone, PartialEq)]
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
        for (comperator, value) in &self.args {
            if other.compare(*comperator, value) {
                return true;
            }
        }

        false
    }
}

impl<T> Debug for Search<T>
where
    T: Parameter,
    T::Storage: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("Search").field("args", &self.args).finish()
    }
}

impl<T: Parameter> FromStr for Search<T> {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let args = s
            .split(',')
            .map(|s: &str| -> Result<(Comperator, T::Storage), Self::Err> {
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

                Ok((comperator, value))
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self { args })
    }
}
