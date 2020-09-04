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

use quick_xml::DeError as Error;
use serde::{
    de::{
        DeserializeSeed, Deserializer as DeDeserializer, EnumAccess as DeEnumAccess,
        VariantAccess as DeVariantAccess, Visitor,
    },
    forward_to_deserialize_any, serde_if_integer128,
};

#[derive(Clone)]
pub struct ValueDeserializer<'a> {
    value: &'a str,
}

impl<'a> ValueDeserializer<'a> {
    pub fn new(value: &'a str) -> Self {
        Self { value }
    }
}

macro_rules! deserialize_num {
    ($method:ident, $visit:ident) => {
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            let value = self.value.parse()?;

            visitor.$visit(value)
        }
    };
}

impl<'de, 'a> DeDeserializer<'de> for ValueDeserializer<'a> {
    type Error = Error;

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.value {
            "true" | "1" | "True" | "TRUE" | "t" | "Yes" | "YES" | "yes" | "y" => {
                visitor.visit_bool(true)
            }
            "false" | "0" | "False" | "FALSE" | "f" | "No" | "NO" | "no" | "n" => {
                visitor.visit_bool(false)
            }
            _ => Err(Error::InvalidBoolean(self.value.into())),
        }
    }

    deserialize_num!(deserialize_i8, visit_i8);
    deserialize_num!(deserialize_i16, visit_i16);
    deserialize_num!(deserialize_i32, visit_i32);
    deserialize_num!(deserialize_i64, visit_i64);

    deserialize_num!(deserialize_u8, visit_u8);
    deserialize_num!(deserialize_u16, visit_u16);
    deserialize_num!(deserialize_u32, visit_u32);
    deserialize_num!(deserialize_u64, visit_u64);

    deserialize_num!(deserialize_f32, visit_f32);
    deserialize_num!(deserialize_f64, visit_f64);

    serde_if_integer128! {
        deserialize_num!(deserialize_i128, visit_i128);
        deserialize_num!(deserialize_u128, visit_u128);
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

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        if self.value.is_empty() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        if self.value.is_empty() {
            visitor.visit_unit()
        } else {
            Err(Error::InvalidUnit(
                "Expecting unit, got non empty attribute".into(),
            ))
        }
    }

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
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_enum(self)
    }

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.value {
            "true" | "True" | "TRUE" | "Yes" | "YES" | "yes" => visitor.visit_bool(true),
            "false" | "False" | "FALSE" | "No" | "NO" | "no" => visitor.visit_bool(false),
            v => visitor.visit_str(v),
        }
    }

    forward_to_deserialize_any! {
        unit_struct seq tuple map struct identifier ignored_any
    }
}

impl<'de, 'a> DeEnumAccess<'de> for ValueDeserializer<'a> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self), Error> {
        let name = seed.deserialize(self.clone())?;

        Ok((name, self))
    }
}

impl<'de, 'a> DeVariantAccess<'de> for ValueDeserializer<'a> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Error> {
        seed.deserialize(self)
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, _visitor: V) -> Result<V::Value, Error> {
        Err(Error::Custom("Unexpected type: tuple variant".to_owned()))
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Error> {
        Err(Error::Custom("Unexpected type: struct variant".to_owned()))
    }
}
