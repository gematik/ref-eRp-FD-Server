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
    practitioner::{Identifier, Practitioner, Qualification},
    primitives::Id,
};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

use super::{
    super::super::constants::{
        CODING_SYSTEM_IDENTIFIER_BASE, CODING_SYSTEM_V2_0203, IDENTITY_SYSTEM_ANR,
        IDENTITY_SYSTEM_ZANR, PRACTITIONER_CODE_LANR, PRACTITIONER_CODE_ZANR,
        RESOURCE_PROFILE_PRACTITIONER, RESOURCE_TYPE_PRACTITIONER,
    },
    misc::{
        CodableConceptDef, CodingDef, DeserializeRoot, IdentifierDef, MetaDef, NameCow,
        ResourceType, SerializeRoot,
    },
    primitives::IdDef,
};

pub struct PractitionerDef;

#[serde(rename = "Practitioner")]
#[derive(Serialize, Deserialize)]
pub struct PractitionerCow<'a>(#[serde(with = "PractitionerDef")] pub Cow<'a, Practitioner>);

#[serde(rename = "Practitioner")]
#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize)]
struct PractitionerHelper<'a> {
    #[serde(with = "IdDef")]
    id: Id,
    meta: MetaDef,
    identifier: Vec<IdentifierDef>,
    name: Vec<NameCow<'a>>,
    qualification: QualificationsDef,
}

#[derive(Serialize, Deserialize)]
struct QualificationsDef(Vec<QualificationDef>);

#[serde(rename = "Qualification")]
#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize)]
struct QualificationDef {
    code: CodableConceptDef,
}

impl ResourceType for Practitioner {
    fn resource_type() -> &'static str {
        RESOURCE_TYPE_PRACTITIONER
    }
}

impl<'a> SerializeRoot<'a> for PractitionerCow<'a> {
    type Inner = Practitioner;

    fn from_inner(inner: &'a Self::Inner) -> Self {
        PractitionerCow(Cow::Borrowed(inner))
    }
}

impl DeserializeRoot for PractitionerCow<'_> {
    type Inner = Practitioner;

    fn into_inner(self) -> Self::Inner {
        self.0.into_owned()
    }
}

impl PractitionerDef {
    pub fn serialize<S: Serializer>(
        practitioner: &Practitioner,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let value: PractitionerHelper = practitioner.into();

        value.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Cow<'static, Practitioner>, D::Error> {
        let value = PractitionerHelper::deserialize(deserializer)?;

        Ok(Cow::Owned(value.try_into().map_err(D::Error::custom)?))
    }
}

impl<'a> Into<PractitionerHelper<'a>> for &'a Practitioner {
    fn into(self) -> PractitionerHelper<'a> {
        PractitionerHelper {
            id: self.id.clone(),
            meta: MetaDef {
                profile: vec![RESOURCE_PROFILE_PRACTITIONER.into()],
                ..Default::default()
            },
            identifier: self
                .identifier
                .clone()
                .into_iter()
                .map(Into::into)
                .collect(),
            name: vec![NameCow::borrowed(&self.name)],
            qualification: self.qualification.clone().into(),
        }
    }
}

impl Into<IdentifierDef> for Identifier {
    fn into(self) -> IdentifierDef {
        match self {
            Identifier::ANR(anr) => IdentifierDef {
                type_: Some(CodableConceptDef {
                    coding: vec![CodingDef {
                        system: Some(CODING_SYSTEM_V2_0203.into()),
                        code: Some(PRACTITIONER_CODE_LANR.into()),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                system: Some(IDENTITY_SYSTEM_ANR.into()),
                value: Some(anr),
                ..Default::default()
            },
            Identifier::ZANR(zanr) => IdentifierDef {
                type_: Some(CodableConceptDef {
                    coding: vec![CodingDef {
                        system: Some(CODING_SYSTEM_IDENTIFIER_BASE.into()),
                        code: Some(PRACTITIONER_CODE_ZANR.into()),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                system: Some(IDENTITY_SYSTEM_ZANR.into()),
                value: Some(zanr),
                ..Default::default()
            },
        }
    }
}

impl Into<QualificationsDef> for Qualification {
    fn into(self) -> QualificationsDef {
        QualificationsDef(vec![
            QualificationDef {
                code: CodableConceptDef {
                    coding: vec![CodingDef {
                        system: Some(self.type_system),
                        code: Some(self.type_code),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
            },
            QualificationDef {
                code: CodableConceptDef {
                    text: Some(self.job_title),
                    ..Default::default()
                },
            },
        ])
    }
}

impl TryInto<Practitioner> for PractitionerHelper<'_> {
    type Error = String;

    fn try_into(self) -> Result<Practitioner, Self::Error> {
        Ok(Practitioner {
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
                .ok_or_else(|| "Practitioner is missing the `name` field!")?
                .into_owned(),
            qualification: self.qualification.try_into()?,
        })
    }
}

impl TryInto<Identifier> for IdentifierDef {
    type Error = String;

    fn try_into(self) -> Result<Identifier, Self::Error> {
        let code = self
            .value
            .ok_or_else(|| "Practitioner indentifier is missing the `code` field!")?;

        match self.system.as_deref() {
            Some(IDENTITY_SYSTEM_ANR) => Ok(Identifier::ANR(code)),
            Some(IDENTITY_SYSTEM_ZANR) => Ok(Identifier::ZANR(code)),
            Some(system) => Err(format!(
                "Practitioner indentifier has invalid system: {}!",
                system
            )),
            None => Err("Practitioner indentifier is missing the `system` field!".to_owned()),
        }
    }
}

impl TryInto<Qualification> for QualificationsDef {
    type Error = String;

    fn try_into(self) -> Result<Qualification, Self::Error> {
        let mut type_system = None;
        let mut type_code = None;
        let mut job_title = None;

        for q in self.0 {
            if let Some(text) = q.code.text {
                job_title = Some(text);
            }

            if let Some(coding) = q.code.coding.into_iter().next() {
                type_system = coding.system;
                type_code = coding.code;
            }
        }

        Ok(Qualification {
            type_system: type_system.ok_or_else(|| "Identifier is missing the `Type` qualifier")?,
            type_code: type_code.ok_or_else(|| "Identifier is missing the `Type` qualifier")?,
            job_title: job_title.ok_or_else(|| "Identifier is missing the `JobTitle` qualifier")?,
        })
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::convert::TryInto;
    use std::fs::read_to_string;

    use resources::misc::Name;

    use crate::fhir::{
        json::{from_str as from_json, to_string as to_json},
        test::trim_json_str,
    };

    use super::super::misc::Root;

    type PractitionerRoot<'a> = Root<PractitionerCow<'a>>;

    #[test]
    fn convert_to() {
        let practitioner = test_practitioner();

        let actual = trim_json_str(&to_json(&PractitionerRoot::new(&practitioner)).unwrap());
        let expected = trim_json_str(&read_to_string("./examples/practitioner.json").unwrap());

        assert_eq!(actual, expected);
    }

    #[test]
    fn convert_from() {
        let actual =
            from_json::<PractitionerRoot>(&read_to_string("./examples/practitioner.json").unwrap())
                .unwrap()
                .into_inner();
        let expected = test_practitioner();

        assert_eq!(actual, expected);
    }

    pub fn test_practitioner() -> Practitioner {
        Practitioner {
            id: "20597e0e-cb2a-45b3-95f0-dc3dbdb617c3".try_into().unwrap(),
            identifier: Some(Identifier::ANR("838382202".into())),
            name: Name {
                given: "Hans".into(),
                name: "Topp-Glücklich".into(),
                prefix: Some("Dr. med.".into()),
                prefix_qualifier: true,
                family: Some("Topp-Glücklich".into()),
                family_ext: None,
                family_prefix: None,
            },
            qualification: Qualification {
                type_system: "https://fhir.kbv.de/CodeSystem/KBV_CS_FOR_Qualification_Type".into(),
                type_code: "00".into(),
                job_title: "Hausarzt".into(),
            },
        }
    }
}
