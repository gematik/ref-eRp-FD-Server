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

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;

use regex::Regex;
use thiserror::Error;

use super::{super::types::FlowType, Decode, Encode};

#[derive(Debug, Clone, PartialEq)]
pub struct PrescriptionId {
    flow_type: FlowType,
    number: usize,
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
    pub fn new(flow_type: FlowType, number: usize) -> Self {
        Self { flow_type, number }
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

        let flow_type =
            FlowType::decode(numbers[0] as usize).map_err(|_| FromStrError::InvalidFlowType)?;
        let number =
            numbers[1] * 1000000000 + numbers[2] * 1000000 + numbers[3] * 1000 + numbers[4];
        let checksum = numbers[5];

        if !verify_iso_7064_checksum(
            100000000000000 * flow_type.encode() as u64 + 100 * number + checksum,
        ) {
            return Err(FromStrError::InvalidChecksum);
        }

        Ok(PrescriptionId::new(flow_type, number as usize))
    }
}

impl Display for PrescriptionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let code = self.flow_type.encode() as u64;
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

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn to_string() {
        let id = PrescriptionId::new(FlowType::PharmaceuticalDrugs, 123456789123);

        let actual = id.to_string();
        let expected = "160.123.456.789.123.58";

        assert_eq!(actual, expected);
    }

    #[test]
    fn from_string() {
        let actual: PrescriptionId = "160.123.456.789.123.58".parse().unwrap();
        let expected = PrescriptionId::new(FlowType::PharmaceuticalDrugs, 123456789123);

        assert_eq!(actual, expected);
    }
}
