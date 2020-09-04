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

use serde::{
    de::{Deserializer as DeDeserializer, Visitor},
    forward_to_deserialize_any, serde_if_integer128,
};

use super::error::Error;

#[derive(Clone)]
pub struct ValueDeserializer<'a> {
    value: &'a str,
}

impl<'a> ValueDeserializer<'a> {
    pub fn new(value: &'a str) -> Self {
        Self { value }
    }
}

macro_rules! deserialize_unsuported {
    ($deserialize:ident, $msg:tt) => {
        fn $deserialize<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Error> {
            Err(Error::NotSupported($msg.to_owned()))
        }
    };
}

impl<'de, 'a> DeDeserializer<'de> for ValueDeserializer<'a> {
    type Error = Error;

    deserialize_unsuported!(deserialize_bool, "bool");

    deserialize_unsuported!(deserialize_i8, "i8");
    deserialize_unsuported!(deserialize_i16, "i16");
    deserialize_unsuported!(deserialize_i32, "i32");
    deserialize_unsuported!(deserialize_i64, "i64");

    serde_if_integer128! {
        deserialize_unsuported!(deserialize_i128, "i128");
    }

    deserialize_unsuported!(deserialize_u8, "u8");
    deserialize_unsuported!(deserialize_u16, "u16");
    deserialize_unsuported!(deserialize_u32, "u32");
    deserialize_unsuported!(deserialize_u64, "u64");

    serde_if_integer128! {
        deserialize_unsuported!(deserialize_u128, "u128");
    }

    deserialize_unsuported!(deserialize_f32, "f32");
    deserialize_unsuported!(deserialize_f64, "f64");

    deserialize_unsuported!(deserialize_option, "option");
    deserialize_unsuported!(deserialize_unit, "unit");
    deserialize_unsuported!(deserialize_any, "any");

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(Error::NotSupported("tuple struct".to_owned()))
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(Error::NotSupported("enum".to_owned()))
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_str(self.value)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_bytes(self.value.as_bytes())
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_bytes(visitor)
    }

    forward_to_deserialize_any! {
        unit_struct seq tuple map struct ignored_any
    }
}
