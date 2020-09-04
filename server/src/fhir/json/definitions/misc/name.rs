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

use resources::misc::Name;
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

use super::{
    super::super::super::constants::{
        EXTENSION_URL_HUMAN_NAME_EXTENSION, EXTENSION_URL_HUMAN_NAME_OWN_NAME,
        EXTENSION_URL_HUMAN_NAME_PREFIX, EXTENSION_URL_ISO21090_EN,
    },
    ExtensionDef, ValueDef,
};

pub struct NameDef;

#[serde(rename = "Name")]
#[derive(Serialize, Deserialize)]
pub struct NameCow<'a>(#[serde(with = "NameDef")] pub Cow<'a, Name>);

#[serde(rename = "Name")]
#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize)]
struct NameHelper {
    #[serde(rename = "use")]
    use_: String,

    family: String,

    #[serde(rename = "_family")]
    family_extension: FamilyDef,

    #[serde(default)]
    given: Vec<String>,

    #[serde(default)]
    prefix: Vec<String>,

    #[serde(default)]
    #[serde(rename = "_prefix")]
    prefix_extension: Vec<PrefixDef>,
}

#[serde(rename = "Family")]
#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize)]
struct FamilyDef {
    extension: Vec<ExtensionDef>,
}

#[serde(rename = "Prefix")]
#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize)]
struct PrefixDef {
    extension: Vec<ExtensionDef>,
}

const NAME_USE: &str = "official";

impl NameDef {
    pub fn serialize<S: Serializer>(name: &Name, serializer: S) -> Result<S::Ok, S::Error> {
        let value: NameHelper = name.clone().into();

        value.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Cow<'static, Name>, D::Error> {
        let value = NameHelper::deserialize(deserializer)?;

        Ok(Cow::Owned(value.try_into().map_err(D::Error::custom)?))
    }
}

impl Into<NameHelper> for Name {
    fn into(self) -> NameHelper {
        let mut extension = Vec::new();

        if let Some(family) = self.family {
            extension.push(ExtensionDef {
                url: EXTENSION_URL_HUMAN_NAME_OWN_NAME.into(),
                value: Some(ValueDef::String(family)),
                ..Default::default()
            })
        }

        if let Some(family_ext) = self.family_ext {
            extension.push(ExtensionDef {
                url: EXTENSION_URL_HUMAN_NAME_EXTENSION.into(),
                value: Some(ValueDef::String(family_ext)),
                ..Default::default()
            })
        }

        if let Some(family_prefix) = self.family_prefix {
            extension.push(ExtensionDef {
                url: EXTENSION_URL_HUMAN_NAME_PREFIX.into(),
                value: Some(ValueDef::String(family_prefix)),
                ..Default::default()
            })
        }

        NameHelper {
            use_: NAME_USE.into(),
            family: self.name,
            family_extension: FamilyDef { extension },
            given: vec![self.given],
            prefix: self.prefix.into_iter().collect(),
            prefix_extension: if self.prefix_qualifier {
                vec![PrefixDef {
                    extension: vec![ExtensionDef {
                        url: EXTENSION_URL_ISO21090_EN.into(),
                        value: Some(ValueDef::Code("AC".into())),
                        ..Default::default()
                    }],
                }]
            } else {
                Vec::new()
            },
        }
    }
}

impl TryInto<Name> for NameHelper {
    type Error = String;

    fn try_into(self) -> Result<Name, Self::Error> {
        let given = self
            .given
            .into_iter()
            .next()
            .ok_or_else(|| "Name name is missing the `given` field!")?;

        let name = self.family;
        let mut family = None;
        let mut family_ext = None;
        let mut family_prefix = None;

        for ex in self.family_extension.extension {
            match ex.url.as_str() {
                EXTENSION_URL_HUMAN_NAME_OWN_NAME => match ex.value {
                    Some(ValueDef::String(value)) => family = Some(value),
                    _ => {
                        return Err(
                            "Extension name familiy is missing the `valueString` field!".to_owned()
                        )
                    }
                },
                EXTENSION_URL_HUMAN_NAME_EXTENSION => match ex.value {
                    Some(ValueDef::String(value)) => family_ext = Some(value),
                    _ => {
                        return Err(
                            "Extension name extension is missing the `valueString` field!"
                                .to_owned(),
                        )
                    }
                },
                EXTENSION_URL_HUMAN_NAME_PREFIX => match ex.value {
                    Some(ValueDef::String(value)) => family_prefix = Some(value),
                    _ => {
                        return Err(
                            "Extension name prefix is missing the `valueString` field!".to_owned()
                        )
                    }
                },
                _ => return Err(format!("Unexpected name extension: {}!", ex.url)),
            }
        }

        let mut prefix_qualifier = false;
        if let Some(prefix) = self.prefix_extension.into_iter().next() {
            for ex in prefix.extension {
                match ex.url.as_str() {
                    EXTENSION_URL_ISO21090_EN => {
                        match &ex.value {
                            Some(ValueDef::Code(code)) if code.as_str() == "AC" => (),
                            Some(_) => {
                                return Err("Name prefix extension has invalid value!".to_owned())
                            }
                            None => {
                                return Err("Name prefix extension is missing the `value` field!"
                                    .to_owned())
                            }
                        }

                        prefix_qualifier = true;
                    }
                    url => return Err(format!("Unexpected extension: {}!", url)),
                }
            }
        }

        Ok(Name {
            given,
            name,
            prefix: self.prefix.into_iter().next(),
            prefix_qualifier,
            family,
            family_ext,
            family_prefix,
        })
    }
}

impl<'a> NameCow<'a> {
    pub fn borrowed(name: &'a Name) -> Self {
        Self(Cow::Borrowed(name))
    }
}

impl NameCow<'_> {
    pub fn into_owned(self) -> Name {
        self.0.into_owned()
    }
}
