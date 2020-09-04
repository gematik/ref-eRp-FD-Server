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

use resources::{
    patient::{Identifier, Patient},
    primitives::{Date, Id},
};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

use super::{
    super::super::constants::{
        CODING_SYSTEM_IDENTIFIER_BASE, IDENTITY_SYSTEM_KVID, IDENTITY_SYSTEM_KVK,
        PATIENT_IDENTIFIER_GKV, PATIENT_IDENTIFIER_KVK, PATIENT_IDENTIFIER_PKV,
        RESOURCE_PROFILE_PATIENT, RESOURCE_TYPE_PATIENT,
    },
    misc::{
        AddressCow, CodableConceptDef, CodingDef, DeserializeRoot, IdentifierDef, MetaDef, NameCow,
        ResourceType, SerializeRoot,
    },
    primitives::{DateDef, IdDef},
};

pub struct PatientDef;

#[serde(rename = "Patient")]
#[derive(Serialize, Deserialize)]
pub struct PatientCow<'a>(#[serde(with = "PatientDef")] pub Cow<'a, Patient>);

#[serde(rename = "Patient")]
#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize)]
struct PatientHelper<'a> {
    #[serde(with = "IdDef")]
    id: Id,
    meta: MetaDef,
    identifier: Vec<IdentifierDef>,
    name: Vec<NameCow<'a>>,
    #[serde(with = "DateDef")]
    birth_date: Date,
    address: Vec<AddressCow<'a>>,
}

impl ResourceType for Patient {
    fn resource_type() -> &'static str {
        RESOURCE_TYPE_PATIENT
    }
}

impl<'a> SerializeRoot<'a> for PatientCow<'a> {
    type Inner = Patient;

    fn from_inner(inner: &'a Self::Inner) -> Self {
        PatientCow(Cow::Borrowed(inner))
    }
}

impl DeserializeRoot for PatientCow<'_> {
    type Inner = Patient;

    fn into_inner(self) -> Self::Inner {
        self.0.into_owned()
    }
}

impl PatientDef {
    pub fn serialize<S: Serializer>(patient: &Patient, serializer: S) -> Result<S::Ok, S::Error> {
        let value: PatientHelper = patient.into();

        value.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Cow<'static, Patient>, D::Error> {
        let value = PatientHelper::deserialize(deserializer)?;

        Ok(Cow::Owned(value.try_into().map_err(D::Error::custom)?))
    }
}

impl<'a> Into<PatientHelper<'a>> for &'a Patient {
    fn into(self) -> PatientHelper<'a> {
        PatientHelper {
            id: self.id.clone(),
            meta: MetaDef {
                profile: vec![RESOURCE_PROFILE_PATIENT.into()],
                ..Default::default()
            },
            identifier: self
                .identifier
                .clone()
                .into_iter()
                .map(Into::into)
                .collect(),
            name: vec![NameCow::borrowed(&self.name)],
            birth_date: self.birth_date.clone(),
            address: vec![AddressCow::borrowed(&self.address)],
        }
    }
}

impl Into<IdentifierDef> for Identifier {
    fn into(self) -> IdentifierDef {
        match self {
            Identifier::GKV { value } => IdentifierDef {
                type_: Some(CodableConceptDef {
                    coding: vec![CodingDef {
                        system: Some(CODING_SYSTEM_IDENTIFIER_BASE.into()),
                        code: Some(PATIENT_IDENTIFIER_GKV.into()),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                system: Some(IDENTITY_SYSTEM_KVID.into()),
                value: Some(value),
                ..Default::default()
            },
            Identifier::PKV { system, value } => IdentifierDef {
                type_: Some(CodableConceptDef {
                    coding: vec![CodingDef {
                        system: Some(CODING_SYSTEM_IDENTIFIER_BASE.into()),
                        code: Some(PATIENT_IDENTIFIER_GKV.into()),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                system,
                value: Some(value),
                ..Default::default()
            },
            Identifier::KVK { value } => IdentifierDef {
                type_: Some(CodableConceptDef {
                    coding: vec![CodingDef {
                        system: Some(CODING_SYSTEM_IDENTIFIER_BASE.into()),
                        code: Some(PATIENT_IDENTIFIER_KVK.into()),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                system: Some(IDENTITY_SYSTEM_KVK.into()),
                value: Some(value),
                ..Default::default()
            },
        }
    }
}

impl TryInto<Patient> for PatientHelper<'_> {
    type Error = String;

    fn try_into(self) -> Result<Patient, Self::Error> {
        Ok(Patient {
            id: self.id,
            identifier: self
                .identifier
                .into_iter()
                .next()
                .map(TryInto::try_into)
                .transpose()?,
            name: self
                .name
                .into_iter()
                .next()
                .ok_or_else(|| "Patient is missing the `name` field!")?
                .into_owned(),
            birth_date: self.birth_date,
            address: self
                .address
                .into_iter()
                .next()
                .ok_or_else(|| "Patient is missing the `address` field!")?
                .into_owned(),
        })
    }
}

impl TryInto<Identifier> for IdentifierDef {
    type Error = String;

    fn try_into(self) -> Result<Identifier, Self::Error> {
        let type_ = match self.type_ {
            Some(type_) => type_,
            None => return Err("Patient identifier is missing the `type` field!".to_owned()),
        };

        let coding = type_
            .coding
            .into_iter()
            .next()
            .ok_or_else(|| "Patient identifier is missing the `coding` field!")?;

        match coding.system.as_deref() {
            Some(CODING_SYSTEM_IDENTIFIER_BASE) => (),
            Some(system) => {
                return Err(format!(
                    "Patient identifier type coding has invalid system: {}!",
                    system
                ))
            }
            None => {
                return Err(
                    "Patient identifier type coding is missing the `system` field!".to_owned(),
                )
            }
        }

        let code = coding
            .code
            .as_deref()
            .ok_or_else(|| "Patient identifier type is missing the `code` field!")?;

        match code {
            PATIENT_IDENTIFIER_GKV => Ok(Identifier::GKV {
                value: self
                    .value
                    .ok_or_else(|| "Patient identifier is missing the `value` field!")?,
            }),
            PATIENT_IDENTIFIER_PKV => Ok(Identifier::PKV {
                value: self
                    .value
                    .ok_or_else(|| "Patient identifier is missing the `value` field!")?,
                system: self.system,
            }),
            PATIENT_IDENTIFIER_KVK => Ok(Identifier::KVK {
                value: self
                    .value
                    .ok_or_else(|| "Patient identifier is missing the `value` field!")?,
            }),
            _ => Err("Patient identifier type has unknown system!".to_owned()),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::convert::TryInto;
    use std::fs::read_to_string;

    use resources::misc::{Address, Name};

    use crate::fhir::{
        json::{from_str as from_json, to_string as to_json},
        test::trim_json_str,
    };

    use super::super::misc::Root;

    type PatientRoot<'a> = Root<PatientCow<'a>>;

    #[test]
    fn convert_to() {
        let patient = test_patient();

        let actual = trim_json_str(&to_json(&PatientRoot::new(&patient)).unwrap());
        let expected = trim_json_str(&read_to_string("./examples/patient.json").unwrap());

        assert_eq!(actual, expected);
    }

    #[test]
    fn convert_from() {
        let actual = from_json::<PatientRoot>(&read_to_string("./examples/patient.json").unwrap())
            .unwrap()
            .into_inner();
        let expected = test_patient();

        assert_eq!(actual, expected);
    }

    pub fn test_patient() -> Patient {
        Patient {
            id: "9774f67f-a238-4daf-b4e6-679deeef3811".try_into().unwrap(),
            identifier: Some(Identifier::GKV {
                value: "X234567890".into(),
            }),
            name: Name {
                prefix: None,
                prefix_qualifier: false,
                given: "Ludger".into(),
                name: "Ludger Königsstein".into(),
                family: Some("Königsstein".into()),
                family_ext: None,
                family_prefix: None,
            },
            birth_date: "1935-06-22".try_into().unwrap(),
            address: Address {
                address: "Musterstr. 1".into(),
                street: Some("Musterstr.".into()),
                number: Some("1".into()),
                addition: None,
                post_box: None,
                city: Some("Berlin".into()),
                zip_code: Some("10623".into()),
                country: None,
            },
        }
    }
}
