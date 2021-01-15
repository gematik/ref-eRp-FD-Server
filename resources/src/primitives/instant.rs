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
use std::ops::{Deref, DerefMut};

use chrono::{DateTime, FixedOffset, Utc};

#[derive(Clone, Debug, PartialEq)]
pub struct Instant(DateTime<Utc>);

impl From<DateTime<Utc>> for Instant {
    fn from(v: DateTime<Utc>) -> Self {
        Self(v)
    }
}

impl From<DateTime<FixedOffset>> for Instant {
    fn from(v: DateTime<FixedOffset>) -> Self {
        Self(v.into())
    }
}

impl TryFrom<&str> for Instant {
    type Error = ();

    fn try_from(v: &str) -> Result<Self, ()> {
        match DateTime::parse_from_rfc3339(v) {
            Ok(value) => Ok(Self(value.into())),
            Err(_) => Err(()),
        }
    }
}

impl Deref for Instant {
    type Target = DateTime<Utc>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Instant {
    fn deref_mut(&mut self) -> &mut DateTime<Utc> {
        &mut self.0
    }
}

impl Display for Instant {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0.to_rfc3339())
    }
}
