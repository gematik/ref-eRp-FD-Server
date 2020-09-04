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

mod deserializer;
mod enums;
mod error;
mod sequences;
mod values;

pub use deserializer::{from_bytes, Deserializer};
pub use error::Error;

#[cfg(test)]
pub mod tests {
    use super::*;

    use serde::Deserialize;

    #[test]
    fn deserialize_u8() {
        let data = [0x02, 0x01, 0x01];

        let actual: u8 = from_bytes(&data).unwrap();
        let expected = 1u8;

        assert_eq!(expected, actual);
    }

    #[test]
    fn deserialize_u16() {
        let data = [0x02, 0x01, 0x01];

        let actual: u16 = from_bytes(&data).unwrap();
        let expected = 1u16;

        assert_eq!(expected, actual);
    }

    #[test]
    fn deserialize_u32() {
        let data = [0x02, 0x01, 0x01];

        let actual: u32 = from_bytes(&data).unwrap();
        let expected = 1u32;

        assert_eq!(expected, actual);
    }

    #[test]
    fn deserialize_u64() {
        let data = [0x02, 0x01, 0x01];

        let actual: u64 = from_bytes(&data).unwrap();
        let expected = 1u64;

        assert_eq!(expected, actual);
    }

    #[test]
    fn deserialize_u128() {
        let data = [0x02, 0x07, 0x03, 0x71, 0x76, 0xA8, 0x8F, 0x8A, 0xC0];

        let actual: u128 = from_bytes(&data).unwrap();
        let expected = 969179378191040u128;

        assert_eq!(expected, actual);
    }

    #[test]
    fn deserialize_string() {
        let data = [
            0x0C, 0x16, 0x67, 0x65, 0x6D, 0x61, 0x74, 0x69, 0x6B, 0x20, 0x47, 0x6D, 0x62, 0x48,
            0x20, 0x4E, 0x4F, 0x54, 0x2D, 0x56, 0x41, 0x4C, 0x49, 0x44,
        ];

        let actual: String = from_bytes(&data).unwrap();
        let expected = "gematik GmbH NOT-VALID".to_owned();

        assert_eq!(expected, actual);
    }

    #[test]
    fn deserialize_option_some() {
        let data = [0x02, 0x01, 0x01];

        let actual: Option<u8> = from_bytes(&data).unwrap();
        let expected = Some(1u8);

        assert_eq!(expected, actual);
    }

    #[test]
    fn deserialize_option_none() {
        let data = [0x05, 0x00];

        let actual: Option<u8> = from_bytes(&data).unwrap();
        let expected = None;

        assert_eq!(expected, actual);
    }

    #[test]
    fn deserialize_option_field() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Test {
            #[serde(rename = "tag=0")]
            opt: Option<usize>,

            int: usize,
        };

        let data_some = [
            0x30, 0x0E, 0xA0, 0x03, 0x02, 0x01, 0x02, 0x02, 0x07, 0x03, 0x71, 0x76, 0xA8, 0x8F,
            0x8A, 0xC0,
        ];
        let data_none = [
            0x30, 0x09, 0x02, 0x07, 0x03, 0x71, 0x76, 0xA8, 0x8F, 0x8A, 0xC0,
        ];

        let actual = from_bytes::<Test>(&data_some).unwrap();
        let expected = Test {
            opt: Some(2),
            int: 969179378191040,
        };

        assert_eq!(expected, actual);

        let actual = from_bytes::<Test>(&data_none).unwrap();
        let expected = Test {
            opt: None,
            int: 969179378191040,
        };

        assert_eq!(expected, actual);
    }

    #[test]
    fn deserialize_enum_oid() {
        #[derive(Deserialize, Debug, PartialEq)]
        enum Test {
            #[serde(alias = "Sha256WithRSAEncryption")]
            #[serde(rename = "name=Sha256WithRSAEncryption&oid=1.2.840.113549.1.1.11")]
            Sha256WithRSAEncryption,

            #[serde(alias = "Sha512WithRSAEncryption")]
            #[serde(rename = "name=Sha512WithRSAEncryption&oid=1.2.840.113549.1.1.13")]
            Sha512WithRSAEncryption,
        };

        let data = [
            0x30, 0x0D, 0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0B, 0x05,
            0x00,
        ];

        let actual: Test = from_bytes(&data).unwrap();
        let expceted = Test::Sha256WithRSAEncryption;

        assert_eq!(actual, expceted);
    }

    #[test]
    fn deserialize_enum_tag() {
        #[derive(Deserialize, Debug, PartialEq)]
        enum Test {
            #[serde(alias = "Fuu")]
            #[serde(rename = "name=Fuu")]
            Fuu(String),

            #[serde(alias = "Bar")]
            #[serde(rename = "name=Bar&tag=0")]
            Bar(String),

            #[serde(alias = "Baz")]
            #[serde(rename = "name=Baz&tag=1")]
            Baz(String),
        };

        let data_fuu = [0x0C, 0x03, 0x31, 0x32, 0x33];
        let data_bar = [0xA0, 0x05, 0x0C, 0x03, 0x31, 0x32, 0x33];
        let data_baz = [0xA1, 0x05, 0x0C, 0x03, 0x31, 0x32, 0x33];

        let actual = from_bytes::<Test>(&data_fuu).unwrap();
        let expceted = Test::Fuu("123".into());
        assert_eq!(actual, expceted);

        let actual = from_bytes::<Test>(&data_bar).unwrap();
        let expceted = Test::Bar("123".into());
        assert_eq!(actual, expceted);

        let actual = from_bytes::<Test>(&data_baz).unwrap();
        let expceted = Test::Baz("123".into());
        assert_eq!(actual, expceted);
    }
}
