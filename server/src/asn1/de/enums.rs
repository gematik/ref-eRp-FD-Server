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

use std::collections::hash_map::{Entry, HashMap};

use asn1_der::{
    typed::{DerDecodable, Null, Sequence},
    DerObject,
};
use serde::de::{
    DeserializeSeed, Deserializer as DeDeserializer, EnumAccess as DeEnumAccess,
    VariantAccess as DeVariantAccess, Visitor,
};

use super::{
    super::types::ObjectIdentifier,
    deserializer::{Arguments, Deserializer, Root},
    error::Error,
    values::ValueDeserializer,
};

pub struct EnumAccess<'a> {
    root: Root<'a>,
    der: DerObject<'a>,
    default: Option<String>,
    tags: HashMap<u8, String>,
    oids: HashMap<ObjectIdentifier, String>,
}

pub struct VariantAccess<'a> {
    root: Root<'a>,
    der: DerObject<'a>,
}

impl<'a> EnumAccess<'a> {
    pub fn new(
        root: Root<'a>,
        der: DerObject<'a>,
        variants: &'static [&'static str],
    ) -> Result<Self, Error> {
        let mut default = None;
        let mut oids = HashMap::default();
        let mut tags = HashMap::default();

        for variant in variants {
            let args = Arguments::from_str(variant)?;
            let name = args.name.ok_or_else(|| Error::EnumVariantWithoutName)?;

            if let Some(oid) = args.oid {
                match oids.entry(oid) {
                    Entry::Occupied(_) => return Err(Error::EnumVariantWithOidAlreadyExists),
                    Entry::Vacant(entry) => {
                        entry.insert(name);
                    }
                }
            } else if !args.tags.is_empty() {
                for tag in args.tags {
                    match tags.entry(tag) {
                        Entry::Occupied(_) => return Err(Error::EnumVariantWithTagAlreadyExists),
                        Entry::Vacant(entry) => {
                            entry.insert(name.clone());
                        }
                    }
                }
            } else if default.is_some() {
                return Err(Error::EnumVariantWithoutName);
            } else {
                default = Some(name);
            }
        }

        Ok(EnumAccess {
            root,
            der,
            default,
            oids,
            tags,
        })
    }

    fn find_variant<'b>(&'b self) -> Result<(DerObject<'a>, &'b str), Error> {
        if let Ok(sequence) = Sequence::load(self.der) {
            if let (Ok(oid), Ok(value)) = (sequence.get(0), sequence.get(1)) {
                if let Ok(oid) = ObjectIdentifier::load(oid) {
                    let name = self
                        .oids
                        .get(&oid)
                        .or_else(|| self.default.as_ref())
                        .ok_or_else(|| {
                            Error::UnknownEnumVariantOid(self.root.offset_of(&self.der), oid)
                        })?;

                    return Ok((value, name));
                }
            }
        }

        if let Some(tag) = extract_tag(&self.der) {
            let name = self
                .tags
                .get(&tag)
                .or_else(|| self.default.as_ref())
                .ok_or_else(|| Error::UnknownEnumVariantTag(self.root.offset_of(&self.der), tag))?;
            let der = DerObject::decode(self.der.value())?;

            return Ok((der, name));
        }

        if let Some(default) = &self.default {
            return Ok((self.der, default));
        }

        Err(Error::UnknownEnumVariant(self.root.offset_of(&self.der)))
    }
}

impl<'de> DeEnumAccess<'de> for &mut EnumAccess<'de> {
    type Error = Error;
    type Variant = VariantAccess<'de>;

    fn variant_seed<V: DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, VariantAccess<'de>), Error> {
        let (der, name) = self.find_variant()?;

        let de = ValueDeserializer::new(name);
        let ret = seed.deserialize(de)?;
        let acc = VariantAccess::new(self.root, der);

        Ok((ret, acc))
    }
}

impl<'a> VariantAccess<'a> {
    pub fn new(root: Root<'a>, der: DerObject<'a>) -> Self {
        Self { root, der }
    }
}

impl<'de> DeVariantAccess<'de> for VariantAccess<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        Null::load(self.der)?;

        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Error> {
        let mut de = Deserializer::new(self.root, self.der);

        seed.deserialize(&mut de)
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, _visitor: V) -> Result<V::Value, Error> {
        unimplemented!()
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        let mut de = Deserializer::new(self.root, self.der);

        de.deserialize_struct("EnumVariant", fields, visitor)
    }
}

fn extract_tag(der: &DerObject<'_>) -> Option<u8> {
    let tag = der.tag();

    if tag & 0xF0 == 0xA0 {
        Some(tag)
    } else {
        None
    }
}
