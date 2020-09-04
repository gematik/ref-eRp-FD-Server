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

use std::str::from_utf8;

use asn1_der::{
    typed::{DerDecodable, DerTypeView, Null},
    DerObject,
};
use serde::{
    de::{Deserialize, Deserializer as DeDeserializer, Visitor},
    serde_if_integer128,
};

use super::{
    super::types::{
        BitString, ObjectIdentifier, OctetString, PrintableString, Sequence, Set, Utf8String,
    },
    enums::EnumAccess,
    sequences::SeqAccess,
    Error,
};

pub fn from_bytes<'de, T: Deserialize<'de>>(bytes: &'de [u8]) -> Result<T, Error> {
    let der = DerObject::decode(bytes)?;
    let mut de = Deserializer::new(Root(bytes), der);

    T::deserialize(&mut de)
}

pub struct Deserializer<'a> {
    root: Root<'a>,
    der: Option<DerObject<'a>>,
    accepted_tags: Vec<u8>,
}

#[derive(Copy, Clone)]
pub struct Root<'a>(&'a [u8]);

#[derive(Default, Debug)]
pub struct Arguments {
    pub oid: Option<ObjectIdentifier>,
    pub name: Option<String>,
    pub tags: Vec<u8>,
}

impl<'a> Deserializer<'a> {
    pub fn new(root: Root<'a>, der: DerObject<'a>) -> Self {
        Self {
            root,
            der: Some(der),
            accepted_tags: Vec::new(),
        }
    }

    pub fn accepted_tags(&mut self, tags: Vec<u8>) {
        self.accepted_tags = tags;
    }

    pub fn extract(self) -> Option<DerObject<'a>> {
        self.der
    }

    fn consume(&mut self) -> Result<DerObject<'a>, Error> {
        Ok(self
            .der
            .take()
            .ok_or_else(|| Error::DerObjectAlreadyConsumed)?)
    }
}

macro_rules! deserialize_unsuported {
    ($deserialize:ident, $msg:tt) => {
        fn $deserialize<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Error> {
            Err(Error::NotSupported($msg.to_owned()))
        }
    };
}

macro_rules! deserialize_unsigned {
    ($deserialize:ident, $visit:ident) => {
        fn $deserialize<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
            let der = self.consume()?;
            let value = DerDecodable::load(der)?;

            visitor.$visit(value)
        }
    };
}

impl<'de, 'a> DeDeserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        unimplemented!()
    }

    fn deserialize_bool<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        unimplemented!()
    }

    deserialize_unsuported!(deserialize_i8, "i8");
    deserialize_unsuported!(deserialize_i16, "i16");
    deserialize_unsuported!(deserialize_i32, "i32");
    deserialize_unsuported!(deserialize_i64, "i64");

    serde_if_integer128! {
        deserialize_unsuported!(deserialize_i128, "i128");
    }

    deserialize_unsigned!(deserialize_u8, visit_u8);
    deserialize_unsigned!(deserialize_u16, visit_u16);
    deserialize_unsigned!(deserialize_u32, visit_u32);
    deserialize_unsigned!(deserialize_u64, visit_u64);

    serde_if_integer128! {
        deserialize_unsigned!(deserialize_u128, visit_u128);
    }

    deserialize_unsuported!(deserialize_f32, "f32");
    deserialize_unsuported!(deserialize_f64, "f64");

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let der = self.consume()?;
        let s = String::load(der)?;
        let mut it = s.chars();

        if let Some(c) = it.next() {
            if it.next().is_some() {
                Err(Error::InvalidChar(self.root.offset_of(&der)))
            } else {
                visitor.visit_char(c)
            }
        } else {
            Err(Error::InvalidChar(self.root.offset_of(&der)))
        }
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let der = self.consume()?;
        let s = self.root.parse_str(der, &self.accepted_tags)?;

        visitor.visit_str(&s)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let der = self.consume()?;
        let s = self.root.parse_str(der, &self.accepted_tags)?;

        visitor.visit_string(s.to_owned())
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let der = self.consume()?;
        let bytes = self.root.parse_bytes(der)?;

        visitor.visit_bytes(&bytes)
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let der = self.consume()?;
        let bytes = self.root.parse_bytes(der)?;

        visitor.visit_byte_buf(bytes)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let der = self.consume()?;
        match der.tag() {
            Null::TAG => visitor.visit_none(),
            tag if self.accepted_tags.contains(&tag) => {
                let der = DerObject::decode(der.value())?;
                let mut de = Deserializer::new(self.root, der);

                visitor.visit_some(&mut de)
            }
            _ if !self.accepted_tags.is_empty() => {
                self.der = Some(der);
                visitor.visit_none()
            }
            _ => {
                let mut de = Deserializer::new(self.root, der);

                visitor.visit_some(&mut de)
            }
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.consume()?;

        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.consume()?;

        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let args = Arguments::from_str(name)?;
        let der = self.consume()?;
        let der = self.root.get_oid_wrapped(der, args.oid)?;

        let mut de = Deserializer::new(self.root, der);
        de.accepted_tags(args.tags);

        visitor.visit_newtype_struct(&mut de)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let der = self.consume()?;

        if self.accepted_tags.contains(&Sequence::TAG) {
            visitor.visit_seq(SeqAccess::new(
                self.root,
                Sequence::load(der)?.into_iter(),
                None,
            ))
        } else {
            visitor.visit_seq(SeqAccess::new(self.root, Set::load(der)?.into_iter(), None))
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        unimplemented!()
    }

    fn deserialize_map<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        unimplemented!()
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let args = Arguments::from_str(name)?;
        let der = self.consume()?;
        let der = self.root.get_oid_wrapped(der, args.oid)?;
        let sequence = Sequence::load(der)?;

        visitor.visit_seq(SeqAccess::new(
            self.root,
            sequence.into_iter(),
            Some(fields),
        ))
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let args = Arguments::from_str(name)?;
        let der = self.consume()?;
        let der = self.root.get_oid_wrapped(der, args.oid)?;
        let mut e = EnumAccess::new(self.root, der, variants)?;

        visitor.visit_enum(&mut e)
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        unimplemented!()
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(
        self,
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        unimplemented!()
    }
}

impl<'a> Root<'a> {
    pub fn get_oid_wrapped(
        &self,
        der: DerObject<'a>,
        oid: Option<ObjectIdentifier>,
    ) -> Result<DerObject<'a>, Error> {
        let der = if let Some(oid) = oid {
            let mut sequence = Sequence::load(der)?.into_iter();

            let ident = sequence
                .next()
                .ok_or_else(|| Error::SequenceToSmall(self.offset_of(&der)))??;
            let ident = ObjectIdentifier::load(ident)?;

            if oid != ident {
                return Err(Error::UnexpectedObjectIdentifier {
                    offset: self.offset_of(&der),
                    actual: ident,
                    expected: oid,
                });
            }

            let der = sequence
                .next()
                .ok_or_else(|| Error::SequenceToSmall(self.offset_of(&der)))??;

            if der.tag() != 0xA0 {
                return Err(Error::UnexpectedTag {
                    offset: self.offset_of(&der),
                    actual: der.tag(),
                    expected: vec![0xA0],
                });
            }

            DerObject::decode(der.value())?
        } else {
            der
        };

        Ok(der)
    }

    pub fn parse_str(&self, der: DerObject<'a>, accepted_tags: &[u8]) -> Result<&'a str, Error> {
        if !accepted_tags.is_empty() {
            if !accepted_tags.contains(&der.tag()) {
                return Err(Error::UnexpectedTag {
                    offset: self.offset_of(&der),
                    expected: accepted_tags.to_owned(),
                    actual: der.tag(),
                });
            }

            let value = from_utf8(der.value())
                .map_err(|_| Error::InvalidUtf8String(self.offset_of(&der)))?;

            return Ok(value);
        }

        if let Ok(s) = Utf8String::load(der) {
            return Ok(s.0);
        }

        if let Ok(s) = PrintableString::load(der) {
            return Ok(s.0);
        }

        Err(Error::InvalidStringObject(self.offset_of(&der)))
    }

    pub fn parse_bytes(&self, der: DerObject<'a>) -> Result<Vec<u8>, Error> {
        if let Ok(buf) = OctetString::load(der) {
            return Ok(buf.0.to_owned());
        }

        if let Ok(buf) = BitString::load(der) {
            return Ok(buf.0);
        }

        Err(Error::InvalidBufferObject(self.offset_of(&der)))
    }

    pub fn offset_of(&self, der: &DerObject<'a>) -> usize {
        let beg = self.0.as_ptr() as usize;
        let end = der.raw().as_ptr() as usize;

        end - beg
    }
}

impl Arguments {
    pub fn from_str(name: &str) -> Result<Self, Error> {
        let mut args = Self::default();

        for part in name.split('&') {
            if part.starts_with("oid=") {
                args.oid = Some(
                    part[4..]
                        .parse()
                        .map_err(|_| Error::InvalidObjectIdentifier(part[4..].to_owned()))?,
                );
            } else if part.starts_with("tag=") {
                args.tags.push(match &part[4..] {
                    "Set" => Set::TAG,
                    "Sequence" => Sequence::TAG,
                    "UTCTime" => 0x17,
                    "GeneralizedTime" => 0x18,
                    tag => {
                        let mut tag = tag.parse().map_err(|_| Error::InvalidTag(tag.to_owned()))?;
                        tag += 0xA0;

                        tag
                    }
                })
            } else if part.starts_with("name=") {
                args.name = Some(part[5..].to_owned());
            } else {
                args.name = Some(part.to_owned());
            }
        }

        Ok(args)
    }
}
