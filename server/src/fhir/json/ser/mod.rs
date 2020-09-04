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

mod formatter;

use std::io::Write;

use serde::{ser::Error as SerError, Serialize};
use serde_json::{
    ser::{CompactFormatter, Serializer},
    Error,
};

pub use formatter::Formatter;

pub fn to_writer<W: Write, T: Serialize>(writer: W, value: &T) -> Result<(), Error> {
    let formatter = Formatter::new(CompactFormatter);
    let mut ser = Serializer::with_formatter(writer, formatter);

    value.serialize(&mut ser)?;

    Ok(())
}

pub fn to_string<T: Serialize>(value: &T) -> Result<String, Error> {
    let mut buf = Vec::with_capacity(128);

    to_writer(&mut buf, value)?;

    Ok(String::from_utf8(buf)
        .map_err(|err| Error::custom(format!("Invalid UTF-8 string: {}", err)))?)
}

#[cfg(test)]
pub mod tests {
    use super::super::super::test::trim_json_str;
    use super::*;

    #[test]
    fn skip_null_values_in_objects() {
        #[derive(Serialize)]
        struct Test {
            test: Option<usize>,
            fuu: usize,
        }

        let test = Test {
            test: None,
            fuu: 123,
        };

        let actual = trim_json_str(&to_string(&test).unwrap());
        let expected = trim_json_str(
            r##"
                {
                    "fuu":123
                }
            "##,
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn skip_null_values_in_arrays() {
        #[derive(Serialize)]
        struct Test {
            test: Vec<Option<usize>>,
            fuu: usize,
        }

        let test = Test {
            test: vec![None, Some(123)],
            fuu: 123,
        };

        let actual = trim_json_str(&to_string(&test).unwrap());
        let expected = trim_json_str(
            r##"
                {
                    "test":[123],
                    "fuu":123
                }
            "##,
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn skip_empty_arrays() {
        #[derive(Serialize)]
        struct Test {
            test: Vec<usize>,
            fuu: usize,
        }

        let test = Test {
            test: Vec::new(),
            fuu: 123,
        };

        let actual = trim_json_str(&to_string(&test).unwrap());
        let expected = trim_json_str(
            r##"
                {
                    "fuu":123
                }
            "##,
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn skip_empty_object() {
        #[derive(Serialize)]
        struct Inner {
            fuu: Option<usize>,
            bar: Option<usize>,
        }

        #[derive(Serialize)]
        struct Test {
            inner: Inner,
            fuu: usize,
        }

        let test = Test {
            inner: Inner {
                fuu: None,
                bar: None,
            },
            fuu: 123,
        };

        let actual = trim_json_str(&to_string(&test).unwrap());
        let expected = trim_json_str(
            r##"
                {
                    "fuu":123
                }
            "##,
        );

        assert_eq!(actual, expected);
    }
}
