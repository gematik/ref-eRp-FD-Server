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

use std::convert::TryInto;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, NaiveDateTime, Utc};
use regex::Regex;
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use super::super::types::FlowType;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct PrescriptionId {
    flow_type: FlowType,
    number: u64,
}

#[derive(Debug, Error)]
pub enum FromStrError {
    #[error("Invalid Format")]
    InvalidFormat,

    #[error("Invalid Flow Type")]
    InvalidFlowType,

    #[error("Invalid Checksum")]
    InvalidChecksum,

    #[error("Parse Error")]
    ParseError,
}

impl PrescriptionId {
    pub fn new(flow_type: FlowType, number: u64) -> Self {
        Self { flow_type, number }
    }

    #[allow(clippy::result_unit_err)]
    pub fn generate(flow_type: FlowType) -> Result<Self, ()> {
        const MAX_COUNTER: u64 = 100;

        static LAST_TIMESTAMP: AtomicU64 = AtomicU64::new(0);
        static LAST_COUNTER: AtomicU64 = AtomicU64::new(0);

        let timestamp = timestamp();
        let last_timestamp = LAST_TIMESTAMP
            .fetch_update(Ordering::Acquire, Ordering::Acquire, |_| Some(timestamp))
            .unwrap();

        let counter = if timestamp != last_timestamp {
            LAST_COUNTER.store(1, Ordering::Release);

            0
        } else {
            let counter = LAST_COUNTER
                .fetch_update(Ordering::Acquire, Ordering::Acquire, |c| c.checked_add(1))
                .map_err(|_| ())?;

            if counter >= MAX_COUNTER {
                return Err(());
            }

            counter
        };

        let number = timestamp * MAX_COUNTER + counter;

        Ok(Self::new(flow_type, number))
    }
}

impl FromStr for PrescriptionId {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RX: Regex = Regex::new(
                r#"^([0-9]{3})\.([0-9]{3})\.([0-9]{3})\.([0-9]{3})\.([0-9]{3})\.([0-9]{2})$"#
            )
            .unwrap();
        }

        let caps = match RX.captures(s) {
            Some(caps) => caps,
            None => return Err(FromStrError::InvalidFormat),
        };

        let numbers = caps
            .iter()
            .skip(1)
            .map(|c| c.unwrap().as_str().parse())
            .collect::<Result<Vec<u64>, _>>()
            .map_err(|_| FromStrError::ParseError)?;

        let flow_type: FlowType = numbers[0]
            .try_into()
            .map_err(|_| FromStrError::InvalidFlowType)?;
        let number =
            numbers[1] * 1000000000 + numbers[2] * 1000000 + numbers[3] * 1000 + numbers[4];
        let checksum = numbers[5];

        let tmp: u64 = flow_type.into();
        if !verify_iso_7064_checksum(100000000000000u64 * tmp + 100u64 * number + checksum) {
            return Err(FromStrError::InvalidChecksum);
        }

        Ok(PrescriptionId::new(flow_type, number))
    }
}

impl Display for PrescriptionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let code: u64 = self.flow_type.into();
        let number = self.number as u64;
        let checksum = calc_iso_7064_checksum(1000000000000 * code + number).unwrap();

        write!(
            f,
            "{:0>3}.{:03}.{:03}.{:03}.{:03}.{:02}",
            code,
            number / 1000000000 % 1000,
            number / 1000000 % 1000,
            number / 1000 % 1000,
            number % 1000,
            checksum
        )
    }
}
impl Serialize for PrescriptionId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", self);

        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for PrescriptionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        Self::from_str(&s)
            .map_err(|err| D::Error::custom(format!("Invalid Prescription ID: {}", err)))
    }
}

fn calc_iso_7064_checksum(value: u64) -> Option<u64> {
    const MODULO: u64 = 97;

    let value = value.checked_mul(100)?;
    let rest = value % MODULO;
    let checksum = (MODULO + 1) - rest;

    Some(checksum)
}

fn verify_iso_7064_checksum(value: u64) -> bool {
    const MODULO: u64 = 97;

    (value % MODULO) == 1
}

fn timestamp() -> u64 {
    const MAX_TIMESTAMP: u64 = 10_000_000_000;

    // 01.01.2021 00:00:00
    let start_timestamp = NaiveDateTime::from_timestamp(1_609_459_200, 0);
    let start_timestamp = DateTime::<Utc>::from_utc(start_timestamp, Utc);

    let timestamp = Utc::now() - start_timestamp;

    timestamp.num_seconds() as u64 % MAX_TIMESTAMP
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn to_string() {
        let id = PrescriptionId::new(FlowType::ApothekenpflichtigeArzneimittel, 123456789123);

        let actual = id.to_string();
        let expected = "160.123.456.789.123.58";

        assert_eq!(actual, expected);
    }

    #[test]
    fn from_string() {
        let actual: PrescriptionId = "160.123.456.789.123.58".parse().unwrap();
        let expected = PrescriptionId::new(FlowType::ApothekenpflichtigeArzneimittel, 123456789123);

        assert_eq!(actual, expected);
    }
}
