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

use std::borrow::Cow;
use std::convert::TryInto;

use resources::misc::Address;
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

use super::{
    super::super::super::constants::{
        EXTENSION_URL_ADDRESS_ADDITION, EXTENSION_URL_ADDRESS_NUMBER,
        EXTENSION_URL_ADDRESS_POST_BOX, EXTENSION_URL_ADDRESS_STREET,
    },
    ExtensionDef, ValueDef,
};

pub struct AddressDef;

#[serde(rename = "Address")]
#[derive(Serialize, Deserialize)]
pub struct AddressCow<'a>(#[serde(with = "AddressDef")] pub Cow<'a, Address>);

#[serde(rename = "Address")]
#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize)]
struct AddressHelper {
    #[serde(rename = "type")]
    type_: String,
    line: Vec<String>,
    #[serde(rename = "_line")]
    line_extension: Vec<LineDef>,
    city: Option<String>,
    postal_code: Option<String>,
    country: Option<String>,
}

#[serde(rename = "Line")]
#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize)]
struct LineDef {
    extension: Vec<ExtensionDef>,
}

pub const ADDRESS_TYPE: &str = "both";

impl AddressDef {
    pub fn serialize<S: Serializer>(address: &Address, serializer: S) -> Result<S::Ok, S::Error> {
        let value: AddressHelper = address.into();

        value.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Cow<'static, Address>, D::Error> {
        let value = AddressHelper::deserialize(deserializer)?;

        Ok(Cow::Owned(value.try_into().map_err(D::Error::custom)?))
    }
}

impl Into<AddressHelper> for &Address {
    fn into(self) -> AddressHelper {
        let mut extension = Vec::new();

        if let Some(number) = self.number.as_ref() {
            extension.push(ExtensionDef {
                url: EXTENSION_URL_ADDRESS_NUMBER.into(),
                value: Some(ValueDef::String(number.clone())),
                ..Default::default()
            })
        }

        if let Some(street) = self.street.as_ref() {
            extension.push(ExtensionDef {
                url: EXTENSION_URL_ADDRESS_STREET.into(),
                value: Some(ValueDef::String(street.clone())),
                ..Default::default()
            })
        }

        if let Some(addition) = self.addition.as_ref() {
            extension.push(ExtensionDef {
                url: EXTENSION_URL_ADDRESS_ADDITION.into(),
                value: Some(ValueDef::String(addition.clone())),
                ..Default::default()
            })
        }

        if let Some(post_box) = self.post_box.as_ref() {
            extension.push(ExtensionDef {
                url: EXTENSION_URL_ADDRESS_POST_BOX.into(),
                value: Some(ValueDef::String(post_box.clone())),
                ..Default::default()
            })
        }

        AddressHelper {
            type_: ADDRESS_TYPE.into(),
            line: vec![self.address.clone()],
            line_extension: vec![LineDef { extension }],
            city: self.city.clone(),
            postal_code: self.zip_code.clone(),
            country: self.country.clone(),
        }
    }
}

impl TryInto<Address> for AddressHelper {
    type Error = String;

    fn try_into(self) -> Result<Address, Self::Error> {
        let address = self
            .line
            .into_iter()
            .next()
            .ok_or_else(|| "Address is missing the `line` field!")?;

        let mut street = None;
        let mut number = None;
        let mut addition = None;
        let mut post_box = None;

        for ex in self
            .line_extension
            .into_iter()
            .next()
            .ok_or_else(|| "Patient address is missing the `_line` field!")?
            .extension
        {
            match ex.url.as_str() {
                EXTENSION_URL_ADDRESS_STREET => match ex.value {
                    Some(ValueDef::String(value)) => street = Some(value),
                    _ => {
                        return Err(
                            "Extension address street is missing the `valueString` field!"
                                .to_owned(),
                        )
                    }
                },
                EXTENSION_URL_ADDRESS_NUMBER => match ex.value {
                    Some(ValueDef::String(value)) => number = Some(value),
                    _ => {
                        return Err(
                            "Extension address number is missing the `valueString` field!"
                                .to_owned(),
                        )
                    }
                },
                EXTENSION_URL_ADDRESS_ADDITION => match ex.value {
                    Some(ValueDef::String(value)) => addition = Some(value),
                    _ => {
                        return Err(
                            "Extension address addition is missing the `valueString` field!"
                                .to_owned(),
                        )
                    }
                },
                EXTENSION_URL_ADDRESS_POST_BOX => match ex.value {
                    Some(ValueDef::String(value)) => post_box = Some(value),
                    _ => {
                        return Err(
                            "Extension address post box is missing the `valueString` field!"
                                .to_owned(),
                        )
                    }
                },
                _ => return Err(format!("Unexpected address extension: {}!", ex.url)),
            }
        }

        Ok(Address {
            address,
            street,
            number,
            addition,
            post_box,
            city: self.city,
            zip_code: self.postal_code,
            country: self.country,
        })
    }
}

impl<'a> AddressCow<'a> {
    pub fn borrowed(address: &'a Address) -> Self {
        Self(Cow::Borrowed(address))
    }
}

impl AddressCow<'_> {
    pub fn into_owned(self) -> Address {
        self.0.into_owned()
    }
}
