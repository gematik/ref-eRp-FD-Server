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

use std::cmp::{Ordering, PartialOrd};
use std::convert::TryFrom;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::ops::Deref;
use std::str::FromStr;

use chrono::{
    naive::NaiveDate, DateTime as ChronoDateTime, Datelike, Duration, TimeZone, Timelike, Utc,
};
use serde::{Deserialize, Serialize};

use regex::{Captures, Regex};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DateTime(String);

#[derive(Default, Debug, PartialEq)]
pub struct Parts {
    pub year: i32,
    pub month: Option<u32>,
    pub day: Option<u32>,
    pub hour: Option<u32>,
    pub min: Option<u32>,
    pub sec: Option<u32>,
    pub nano: Option<(u32, u32)>,
}

impl<TZ> From<ChronoDateTime<TZ>> for DateTime
where
    TZ: TimeZone,
    <TZ as TimeZone>::Offset: Display,
{
    fn from(v: ChronoDateTime<TZ>) -> Self {
        Self(v.to_rfc3339())
    }
}

impl From<DateTime> for ChronoDateTime<Utc> {
    fn from(v: DateTime) -> Self {
        v.0.parse::<Parts>().unwrap().into()
    }
}

impl TryFrom<&str> for DateTime {
    type Error = String;

    fn try_from(v: &str) -> Result<Self, Self::Error> {
        from_string(v.to_owned())
    }
}

impl TryFrom<String> for DateTime {
    type Error = String;

    fn try_from(v: String) -> Result<Self, Self::Error> {
        from_string(v)
    }
}

impl Deref for DateTime {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for DateTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        self.0.fmt(f)
    }
}

impl FromStr for Parts {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let captures = RX.captures(&s).ok_or(())?;

        let mut parts = Self {
            year: from_capture(&captures, 1)?.ok_or(())?,
            month: from_capture(&captures, 2)?,
            day: from_capture(&captures, 3)?,
            hour: from_capture(&captures, 4)?,
            min: from_capture(&captures, 5)?,
            sec: from_capture(&captures, 6)?,
            nano: from_capture::<u32>(&captures, 7)?.map(|nsec| {
                let exp = (9 - captures.get(7).unwrap().as_str().len()) as u32;

                (nsec, exp)
            }),
        };

        if let Some(tz) = captures.get(8) {
            let tz = tz.as_str();
            if tz != "Z" {
                let mut split = tz[1..].split(':');
                let hours = split.next().ok_or(())?.parse().map_err(|_| ())?;
                let minutes = split.next().ok_or(())?.parse().map_err(|_| ())?;

                let mut t = NaiveDate::from_ymd(
                    parts.year,
                    parts.month.unwrap_or(1),
                    parts.day.unwrap_or(1),
                )
                .and_hms_nano(
                    parts.hour.unwrap_or_default(),
                    parts.min.unwrap_or_default(),
                    parts.sec.unwrap_or_default(),
                    parts
                        .nano
                        .map(|(x, exp)| x * 10u32.pow(exp))
                        .unwrap_or_default(),
                );

                if tz.starts_with('+') {
                    t -= Duration::hours(hours) + Duration::minutes(minutes);
                } else {
                    t += Duration::hours(hours) + Duration::minutes(minutes);
                }

                parts.year = t.year();
                parts.month = parts.month.map(|_| t.month());
                parts.day = parts.day.map(|_| t.day());
                parts.hour = parts.hour.map(|_| t.hour());
                parts.min = parts.min.map(|_| t.minute());
                parts.sec = parts.sec.map(|_| t.second());
            }
        }

        Ok(parts)
    }
}

impl From<Parts> for ChronoDateTime<Utc> {
    fn from(v: Parts) -> Self {
        let t = NaiveDate::from_ymd(v.year, v.month.unwrap_or(1), v.day.unwrap_or(1)).and_hms_nano(
            v.hour.unwrap_or_default(),
            v.min.unwrap_or_default(),
            v.sec.unwrap_or_default(),
            v.nano
                .map(|(x, exp)| x * 10u32.pow(exp))
                .unwrap_or_default(),
        );

        Self::from_utc(t, Utc)
    }
}

impl PartialEq<ChronoDateTime<Utc>> for Parts {
    fn eq(&self, other: &ChronoDateTime<Utc>) -> bool {
        macro_rules! eq {
            ($a:expr, $b:expr) => {
                $a == $b
            };
        }

        macro_rules! eq_opt {
            ($a:expr, $b:expr) => {
                if let Some(a) = $a {
                    a == $b
                } else {
                    true
                }
            };
        }

        eq!(self.year, other.year())
            && eq_opt!(self.month, other.month())
            && eq_opt!(self.day, other.day())
            && eq_opt!(self.hour, other.hour())
            && eq_opt!(self.min, other.minute())
            && eq_opt!(self.sec, other.second())
            && eq_opt!(
                self.nano.map(|(x, exp)| x * 10u32.pow(exp)),
                other.nanosecond()
            )
    }
}

impl PartialOrd<ChronoDateTime<Utc>> for Parts {
    fn partial_cmp(&self, other: &ChronoDateTime<Utc>) -> Option<Ordering> {
        macro_rules! cmp {
            ($a:expr, $b:expr) => {
                if $a < $b {
                    return Some(Ordering::Greater);
                } else if $a > $b {
                    return Some(Ordering::Less);
                }
            };
        }

        macro_rules! cmp_opt {
            ($a:expr, $b:expr) => {
                if let Some(a) = $a {
                    cmp!(a, $b);
                }
            };
        }

        cmp!(self.year, other.year());
        cmp_opt!(self.month, other.month());
        cmp_opt!(self.day, other.day());
        cmp_opt!(self.hour, other.hour());
        cmp_opt!(self.min, other.minute());
        cmp_opt!(self.sec, other.second());

        if let Some((nsec, exp)) = self.nano {
            cmp!(nsec, other.nanosecond() / 10u32.pow(exp));
        }

        Some(Ordering::Equal)
    }
}

fn from_capture<T: FromStr>(captures: &Captures, index: usize) -> Result<Option<T>, ()> {
    captures
        .get(index)
        .map(|c| c.as_str().parse().map_err(|_| ()))
        .transpose()
}

fn from_string(s: String) -> Result<DateTime, String> {
    if RX.is_match(&s) {
        Ok(DateTime(s))
    } else {
        Err(s)
    }
}

lazy_static! {
    static ref RX: Regex = Regex::new(
        r#"^([0-9](?:[0-9](?:[0-9][1-9]|[1-9]0)|[1-9]00)|[1-9]000)(?:-(0[1-9]|1[0-2])(?:-(0[1-9]|[1-2][0-9]|3[0-1])(?:T([01][0-9]|2[0-3])(?::([0-5][0-9])(?::([0-5][0-9])(?:\.([0-9]{1,9}))?)?)?)?)?)?(Z|(?:\+|-)(?:(?:0[0-9]|1[0-3]):[0-5][0-9]|14:00))?$"#
    )
    .unwrap();
}
