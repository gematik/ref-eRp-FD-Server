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
use std::str::FromStr;

use chrono::{naive::NaiveDate, DateTime, Datelike, Duration, Timelike, Utc};
use regex::{Captures, Regex};

use super::{Comperator, Parameter};

#[derive(Default, Debug, PartialEq)]
pub struct DateTimeStorage {
    year: i32,
    month: Option<u32>,
    day: Option<u32>,
    hour: Option<u32>,
    min: Option<u32>,
    sec: Option<u32>,
    nano: Option<(u32, u32)>,
}

impl FromStr for DateTimeStorage {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RX: Regex = Regex::new(
                r#"^([0-9](?:[0-9](?:[0-9][1-9]|[1-9]0)|[1-9]00)|[1-9]000)(?:-(0[1-9]|1[0-2])(?:-(0[1-9]|[1-2][0-9]|3[0-1])(?:T([01][0-9]|2[0-3])(?::([0-5][0-9])(?::([0-5][0-9])(?:\.([0-9]{1,9}))?)?)?)?)?)?(Z|(?:\+|-)(?:(?:0[0-9]|1[0-3]):[0-5][0-9]|14:00))?$"#
            )
            .unwrap();
        }

        let captures = RX.captures(&s).ok_or(())?;

        let mut storage = Self {
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
                let mut parts = tz[1..].split(':');
                let hours = parts.next().ok_or(())?.parse().map_err(|_| ())?;
                let minutes = parts.next().ok_or(())?.parse().map_err(|_| ())?;

                let mut t = NaiveDate::from_ymd(
                    storage.year,
                    storage.month.unwrap_or(1),
                    storage.day.unwrap_or(1),
                )
                .and_hms_nano(
                    storage.hour.unwrap_or_default(),
                    storage.min.unwrap_or_default(),
                    storage.sec.unwrap_or_default(),
                    storage
                        .nano
                        .map(|(x, exp)| x * 10u32.pow(exp))
                        .unwrap_or_default(),
                );

                if tz.starts_with('+') {
                    t -= Duration::hours(hours) + Duration::minutes(minutes);
                } else {
                    t += Duration::hours(hours) + Duration::minutes(minutes);
                }

                storage.year = t.year();
                storage.month = storage.month.map(|_| t.month());
                storage.day = storage.day.map(|_| t.day());
                storage.hour = storage.hour.map(|_| t.hour());
                storage.min = storage.min.map(|_| t.minute());
                storage.sec = storage.sec.map(|_| t.second());
            }
        }

        Ok(storage)
    }
}

impl PartialEq<DateTime<Utc>> for DateTimeStorage {
    fn eq(&self, other: &DateTime<Utc>) -> bool {
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

impl PartialOrd<DateTime<Utc>> for DateTimeStorage {
    fn partial_cmp(&self, other: &DateTime<Utc>) -> Option<Ordering> {
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

impl Parameter for DateTime<Utc> {
    type Storage = DateTimeStorage;

    fn parse(s: &str) -> Result<Self::Storage, String> {
        Ok(s.parse()
            .map_err(|()| "Invalid search parameter: date time format!")?)
    }

    fn compare(&self, comperator: Comperator, param: &Self::Storage) -> bool {
        let ord = match param.partial_cmp(&self) {
            Some(ord) => ord,
            None => return false,
        };

        match comperator {
            Comperator::Equal => ord == Ordering::Equal,
            Comperator::NotEqual => ord != Ordering::Equal,
            Comperator::GreaterThan | Comperator::StartsAfter => ord == Ordering::Greater,
            Comperator::LessThan | Comperator::EndsBefore => ord == Ordering::Less,
            Comperator::GreaterEqual => ord != Ordering::Less,
            Comperator::LessEqual => ord != Ordering::Greater,
            Comperator::Approximately => false,
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use super::super::Search;

    macro_rules! test_parse {
        ($s:expr, $cmp:ident, $date:expr) => {
            let x = Search::<DateTime<Utc>>::from_str($s).unwrap();
            let x = &x.args[0];

            assert_eq!(x.0, Comperator::$cmp);
            assert_eq!(x.1, $date);
        };
    }

    macro_rules! test_match_inner {
        (does_match => $actual:expr) => {
            let x = Search::<DateTime<Utc>>::from_str($actual).unwrap();

            assert!(
                x.matches(&date_time("2020-11-17T09:52:11.987654321Z")),
                "Search parameter does not match the date '2020-11-17T09:52:11.987654321Z': {:?}",
                x
            );
        };
        (does_not_match => $actual:expr) => {
            let x = Search::<DateTime<Utc>>::from_str($actual).unwrap();

            assert!(
                !x.matches(&date_time("2020-11-17T09:52:11.987654321Z")),
                "Search parameter does match the date '2020-11-17T09:52:11.987654321Z': {:?}",
                x
            );
        };
    }

    macro_rules! test_match {
        ($op:expr => $m:ident => greater) => {
            test_match_inner!($m => format!("{}{}", $op, "2021").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-12").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-18").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T10").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:53").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:12").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:12.0").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11.99").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11.988").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11.9877").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11.98766").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11.987655").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11.9876544").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11.98765433").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11.987654322").as_str());
        };
        ($op:expr => $m:ident => equal) => {
            test_match_inner!($m => format!("{}{}", $op, "2020").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11.9").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11.98").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11.987").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11.9876").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11.98765").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11.987654").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11.9876543").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11.98765432").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11.987654321").as_str());
        };
        ($op:expr => $m:ident => less) => {
            test_match_inner!($m => format!("{}{}", $op, "2019").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-10").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-16").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T08").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:51").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:10").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11.8").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11.97").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11.986").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11.9875").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11.98764").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11.987653").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11.9876542").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11.98765431").as_str());
            test_match_inner!($m => format!("{}{}", $op, "2020-11-17T09:52:11.987654320").as_str());
        };
    }

    #[test]
    fn parse_date_time_search_parameter() {
        test_parse!(
            "eq2020",
            Equal,
            DateTimeStorage {
                year: 2020,
                ..Default::default()
            }
        );
        test_parse!(
            "eq2020-11",
            Equal,
            DateTimeStorage {
                year: 2020,
                month: Some(11),
                ..Default::default()
            }
        );
        test_parse!(
            "eq2020-11-17",
            Equal,
            DateTimeStorage {
                year: 2020,
                month: Some(11),
                day: Some(17),
                ..Default::default()
            }
        );
        test_parse!(
            "eq2020-11-17T09",
            Equal,
            DateTimeStorage {
                year: 2020,
                month: Some(11),
                day: Some(17),
                hour: Some(9),
                ..Default::default()
            }
        );
        test_parse!(
            "eq2020-11-17T09:41",
            Equal,
            DateTimeStorage {
                year: 2020,
                month: Some(11),
                day: Some(17),
                hour: Some(9),
                min: Some(41),
                ..Default::default()
            }
        );
        test_parse!(
            "eq2020-11-17T09:41:11",
            Equal,
            DateTimeStorage {
                year: 2020,
                month: Some(11),
                day: Some(17),
                hour: Some(9),
                min: Some(41),
                sec: Some(11),
                ..Default::default()
            }
        );
        test_parse!(
            "eq2020-11-17T09:41:11+01:00",
            Equal,
            DateTimeStorage {
                year: 2020,
                month: Some(11),
                day: Some(17),
                hour: Some(8),
                min: Some(41),
                sec: Some(11),
                ..Default::default()
            }
        );
        test_parse!(
            "eq2020-11-17T09:41:11.123+01:00",
            Equal,
            DateTimeStorage {
                year: 2020,
                month: Some(11),
                day: Some(17),
                hour: Some(8),
                min: Some(41),
                sec: Some(11),
                nano: Some((123, 6)),
            }
        );
        test_parse!(
            "eq2020-11-17T09:41:11.123456+01:00",
            Equal,
            DateTimeStorage {
                year: 2020,
                month: Some(11),
                day: Some(17),
                hour: Some(8),
                min: Some(41),
                sec: Some(11),
                nano: Some((123456, 3)),
            }
        );
        test_parse!(
            "eq2020-11-17T09:41:11.123456789+01:00",
            Equal,
            DateTimeStorage {
                year: 2020,
                month: Some(11),
                day: Some(17),
                hour: Some(8),
                min: Some(41),
                sec: Some(11),
                nano: Some((123456789, 0)),
            }
        );
    }

    #[test]
    fn match_date_time_search_parameter() {
        test_match!("eq" => does_not_match => greater);
        test_match!("eq" => does_match => equal);
        test_match!("eq" => does_not_match => less);

        test_match!("ne" => does_match => greater);
        test_match!("ne" => does_not_match => equal);
        test_match!("ne" => does_match => less);

        test_match!("gt" => does_not_match => greater);
        test_match!("gt" => does_not_match => equal);
        test_match!("gt" => does_match => less);

        test_match!("lt" => does_match => greater);
        test_match!("lt" => does_not_match => equal);
        test_match!("lt" => does_not_match => less);

        test_match!("ge" => does_not_match => greater);
        test_match!("ge" => does_match => equal);
        test_match!("ge" => does_match => less);

        test_match!("le" => does_match => greater);
        test_match!("le" => does_match => equal);
        test_match!("le" => does_not_match => less);

        test_match!("sa" => does_not_match => greater);
        test_match!("sa" => does_not_match => equal);
        test_match!("sa" => does_match => less);

        test_match!("eb" => does_match => greater);
        test_match!("eb" => does_not_match => equal);
        test_match!("eb" => does_not_match => less);
    }

    fn date_time(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().into()
    }
}
