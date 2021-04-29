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

use chrono::{DateTime, Utc};
use resources::primitives::DateTimeParts;

use super::{Comperator, Parameter};

impl Parameter for DateTime<Utc> {
    type Storage = DateTimeParts;

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

    use std::str::FromStr;

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
            DateTimeParts {
                year: 2020,
                ..Default::default()
            }
        );
        test_parse!(
            "eq2020-11",
            Equal,
            DateTimeParts {
                year: 2020,
                month: Some(11),
                ..Default::default()
            }
        );
        test_parse!(
            "eq2020-11-17",
            Equal,
            DateTimeParts {
                year: 2020,
                month: Some(11),
                day: Some(17),
                ..Default::default()
            }
        );
        test_parse!(
            "eq2020-11-17T09",
            Equal,
            DateTimeParts {
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
            DateTimeParts {
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
            DateTimeParts {
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
            DateTimeParts {
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
            DateTimeParts {
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
            DateTimeParts {
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
            DateTimeParts {
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
