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

#[derive(Clone, PartialEq, Debug)]
pub struct Code {
    pub system: String,
    pub code: String,
}

pub trait Decode: Sized {
    type Code;
    type Auto;

    fn decode(code: Self::Code) -> Result<Self, Self::Code>;
}

pub trait Encode: Sized {
    type Code;
    type Auto;

    fn encode(&self) -> Self::Code;
}

pub trait DecodeStr: Sized {
    fn decode_str(s: &str) -> Result<Self, &str>;
}

pub trait EncodeStr: Sized {
    fn encode_str(&self) -> String;
}

impl<T> DecodeStr for T
where
    T: Decode<Auto = ()>,
    <T as Decode>::Code: FromStr,
{
    fn decode_str(s: &str) -> Result<Self, &str> {
        match s.parse() {
            Ok(code) => Self::decode(code).map_err(|_| s),
            Err(_) => Err(s),
        }
    }
}

impl<T> EncodeStr for T
where
    T: Encode<Auto = ()>,
    <T as Encode>::Code: Display,
{
    fn encode_str(&self) -> String {
        self.encode().to_string()
    }
}
