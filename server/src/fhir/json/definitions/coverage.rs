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
    coverage::{Coverage, Extension, Payor},
    primitives::{DateTime, Id},
};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

use super::{
    super::super::constants::{
        CODING_SYSTEM_V2_0203, EXTENSION_URL_ALTERNATIVE_IK, EXTENSION_URL_DMP_MARK,
        EXTENSION_URL_INSURED_TYPE, EXTENSION_URL_SPECIAL_GROUP, EXTENSION_URL_WOP,
        IDENTITY_SYSTEM_IKNR, RESOURCE_PROFILE_COVERAGE, RESOURCE_TYPE_COVERAGE,
    },
    misc::{
        CodableConceptDef, CodingDef, DeserializeRoot, ExtensionDef, IdentifierDef, MetaDef,
        ReferenceDef, ResourceType, SerializeRoot, ValueDef,
    },
    primitives::{DateTimeDef, IdDef},
};

pub struct CoverageDef;

#[serde(rename = "Coverage")]
#[derive(Serialize, Deserialize)]
pub struct CoverageCow<'a>(#[serde(with = "CoverageDef")] pub Cow<'a, Coverage>);

#[serde(rename = "Coverage")]
#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize)]
struct CoverageHelper {
    #[serde(with = "IdDef")]
    id: Id,

    meta: MetaDef,

    extension: ExtensionsDef,

    status: String,

    type_: CodableConceptDef,

    beneficiary: ReferenceDef,

    period: Option<PeriodDef>,

    payor: Vec<ReferenceDef>,
}

#[derive(Serialize, Deserialize)]
struct ExtensionsDef(Vec<ExtensionDef>);

#[serde(rename = "Period")]
#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize)]
struct PeriodDef {
    #[serde(with = "DateTimeDef")]
    end: DateTime,
}

const STATUS: &str = "active";

impl ResourceType for Coverage {
    fn resource_type() -> &'static str {
        RESOURCE_TYPE_COVERAGE
    }
}

impl<'a> SerializeRoot<'a> for CoverageCow<'a> {
    type Inner = Coverage;

    fn from_inner(inner: &'a Self::Inner) -> Self {
        CoverageCow(Cow::Borrowed(inner))
    }
}

impl DeserializeRoot for CoverageCow<'_> {
    type Inner = Coverage;

    fn into_inner(self) -> Self::Inner {
        self.0.into_owned()
    }
}

impl CoverageDef {
    pub fn serialize<S: Serializer>(coverage: &Coverage, serializer: S) -> Result<S::Ok, S::Error> {
        let root: CoverageHelper = coverage.into();

        root.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Cow<'static, Coverage>, D::Error> {
        let root = CoverageHelper::deserialize(deserializer)?;

        Ok(Cow::Owned(root.try_into().map_err(D::Error::custom)?))
    }
}

impl Into<CoverageHelper> for &Coverage {
    fn into(self) -> CoverageHelper {
        CoverageHelper {
            id: self.id.clone(),
            meta: MetaDef {
                profile: vec![RESOURCE_PROFILE_COVERAGE.into()],
                ..Default::default()
            },
            extension: (&self.extension).into(),
            status: STATUS.into(),
            type_: self.type_.clone().into(),
            beneficiary: ReferenceDef {
                reference: Some(self.beneficiary.clone()),
                ..Default::default()
            },
            period: self
                .period_end
                .as_ref()
                .map(|end| PeriodDef { end: end.clone() }),
            payor: vec![(&self.payor).into()],
        }
    }
}

impl Into<ExtensionsDef> for &Extension {
    fn into(self) -> ExtensionsDef {
        let mut ret = Vec::new();

        if let Some(special_group) = &self.special_group {
            ret.push(ExtensionDef {
                url: EXTENSION_URL_SPECIAL_GROUP.into(),
                value: Some(special_group.clone().into()),
                ..Default::default()
            })
        }

        if let Some(dmp_mark) = &self.dmp_mark {
            ret.push(ExtensionDef {
                url: EXTENSION_URL_DMP_MARK.into(),
                value: Some(dmp_mark.clone().into()),
                ..Default::default()
            })
        }

        if let Some(insured_type) = &self.insured_type {
            ret.push(ExtensionDef {
                url: EXTENSION_URL_INSURED_TYPE.into(),
                value: Some(insured_type.clone().into()),
                ..Default::default()
            })
        }

        if let Some(wop) = &self.wop {
            ret.push(ExtensionDef {
                url: EXTENSION_URL_WOP.into(),
                value: Some(wop.clone().into()),
                ..Default::default()
            })
        }

        ExtensionsDef(ret)
    }
}

impl Into<ReferenceDef> for &Payor {
    fn into(self) -> ReferenceDef {
        ReferenceDef {
            identifier: Some(IdentifierDef {
                extension: self
                    .alternative_id
                    .as_ref()
                    .into_iter()
                    .map(|alternative_id| ExtensionDef {
                        url: EXTENSION_URL_ALTERNATIVE_IK.into(),
                        value: Some(ValueDef::Identifier(IdentifierDef {
                            type_: Some(CodableConceptDef {
                                coding: vec![CodingDef {
                                    system: Some(CODING_SYSTEM_V2_0203.into()),
                                    code: Some("XX".into()),
                                    ..Default::default()
                                }],
                                ..Default::default()
                            }),
                            system: Some(IDENTITY_SYSTEM_IKNR.into()),
                            value: Some(alternative_id.clone()),
                            ..Default::default()
                        })),
                        ..Default::default()
                    })
                    .collect(),
                system: Some(IDENTITY_SYSTEM_IKNR.into()),
                value: self.value.as_ref().map(Clone::clone),
                ..Default::default()
            }),
            display: Some(self.display.clone()),
            ..Default::default()
        }
    }
}

impl TryInto<Coverage> for CoverageHelper {
    type Error = String;

    fn try_into(self) -> Result<Coverage, Self::Error> {
        if self.meta.profile != vec![RESOURCE_PROFILE_COVERAGE] {
            return Err("Coverage has an invalid profile".to_owned());
        }

        Ok(Coverage {
            id: self.id,
            extension: self.extension.try_into()?,
            type_: self.type_.try_into()?,
            beneficiary: self
                .beneficiary
                .reference
                .ok_or_else(|| "Coverage beneficiary is missing the `reference` field!")?,
            period_end: self.period.map(|period| period.end),
            payor: self
                .payor
                .into_iter()
                .next()
                .ok_or_else(|| "Coverage is missing the `payor` field!")?
                .try_into()?,
        })
    }
}

impl TryInto<Extension> for ExtensionsDef {
    type Error = String;

    fn try_into(self) -> Result<Extension, Self::Error> {
        let mut special_group = None;
        let mut dmp_mark = None;
        let mut insured_type = None;
        let mut wop = None;

        for ex in self.0 {
            match ex.url.as_str() {
                EXTENSION_URL_SPECIAL_GROUP => {
                    special_group = Some(
                        ex.value
                            .ok_or_else(|| "Extension is missing the `value` field!")?
                            .try_into()?,
                    )
                }
                EXTENSION_URL_DMP_MARK => {
                    dmp_mark = Some(
                        ex.value
                            .ok_or_else(|| "Extension is missing the `value` field!")?
                            .try_into()?,
                    )
                }
                EXTENSION_URL_INSURED_TYPE => {
                    insured_type = Some(
                        ex.value
                            .ok_or_else(|| "Extension is missing the `value` field!")?
                            .try_into()?,
                    )
                }
                EXTENSION_URL_WOP => {
                    wop = Some(
                        ex.value
                            .ok_or_else(|| "Extension is missing the `value` field!")?
                            .try_into()?,
                    )
                }
                url => return Err(format!("Unexpected extension: {}", url)),
            }
        }

        Ok(Extension {
            special_group,
            dmp_mark,
            insured_type,
            wop,
        })
    }
}

impl TryInto<Payor> for ReferenceDef {
    type Error = String;

    fn try_into(self) -> Result<Payor, Self::Error> {
        let mut value = None;
        let mut alternative_id = None;

        if let Some(identifier) = self.identifier {
            for ex in identifier.extension {
                match ex.url.as_ref() {
                    EXTENSION_URL_ALTERNATIVE_IK => {
                        match ex.value {
                            Some(ValueDef::Identifier(identifier)) => {
                                match identifier.system {
                                    Some(s) if s == IDENTITY_SYSTEM_IKNR => (),
                                    Some(_) => return Err("Coverage payor identifier has invalid system!".to_owned()),
                                    None => return Err("Coverage payor identifier value is missing the `system` field!".to_owned()),
                                }

                                alternative_id = Some(identifier.value.ok_or_else(|| {
                                    "Coverage payor identifier value is missing the `value` field!"
                                })?);
                            }
                            _ => return Err(
                                "Coverage payor identifier is missing the `valueIdentifier` field!"
                                    .to_owned(),
                            ),
                        }
                    }
                    url => {
                        return Err(format!(
                            "Coverage payor identifier has unexpected extension: {}!",
                            url
                        ))
                    }
                }
            }

            match identifier.system.as_deref() {
                Some(IDENTITY_SYSTEM_IKNR) => (),
                Some(system) => {
                    return Err(format!(
                        "Coverage payor identifier has invalid system: {}!",
                        system
                    ))
                }
                None => {
                    return Err(
                        "Coverage payor identifier is missing the `system` field!".to_owned()
                    )
                }
            }

            value = Some(identifier.value.ok_or_else(|| {
                "Coverage payor identifier is missing the `value` field!".to_owned()
            })?);
        }

        Ok(Payor {
            display: self
                .display
                .ok_or_else(|| "Coverage payor is missing the `display` field!")?,
            value,
            alternative_id,
        })
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::convert::TryInto;
    use std::fs::read_to_string;

    use resources::misc::Code;

    use crate::fhir::{
        json::{from_str as from_json, to_string as to_json},
        test::trim_json_str,
    };

    use super::super::misc::Root;

    type CoverageRoot<'a> = Root<CoverageCow<'a>>;

    #[test]
    fn convert_to() {
        let coverage = test_coverage();

        let actual = trim_json_str(&to_json(&CoverageRoot::new(&coverage)).unwrap());
        let expected = trim_json_str(&read_to_string("./examples/coverage.json").unwrap());

        assert_eq!(actual, expected);
    }

    #[test]
    fn convert_from() {
        let actual =
            from_json::<CoverageRoot>(&read_to_string("./examples/coverage.json").unwrap())
                .unwrap()
                .into_inner();
        let expected = test_coverage();

        assert_eq!(actual, expected);
    }

    pub fn test_coverage() -> Coverage {
        Coverage {
            id: "1b1ffb6e-eb05-43d7-87eb-e7818fe9661a".try_into().unwrap(),
            extension: Extension {
                special_group: Some(Code {
                    system: "https://fhir.kbv.de/CodeSystem/KBV_CS_SFHIR_KBV_PERSONENGRUPPE".into(),
                    code: "00".into(),
                }),
                dmp_mark: Some(Code {
                    system: "https://fhir.kbv.de/CodeSystem/KBV_CS_SFHIR_KBV_DMP".into(),
                    code: "00".into(),
                }),
                insured_type: Some(Code {
                    system: "https://fhir.kbv.de/CodeSystem/KBV_CS_SFHIR_KBV_VERSICHERTENSTATUS"
                        .into(),
                    code: "1".into(),
                }),
                wop: Some(Code {
                    system: "https://fhir.kbv.de/CodeSystem/KBV_CS_SFHIR_ITA_WOP".into(),
                    code: "03".into(),
                }),
            },
            type_: Code {
                system: "http://fhir.de/CodeSystem/versicherungsart-de-basis".into(),
                code: "GKV".into(),
            },
            beneficiary: "Patient/9774f67f-a238-4daf-b4e6-679deeef3811".into(),
            period_end: None,
            payor: Payor {
                display: "AOK Rheinland/Hamburg".into(),
                value: Some("104212059".into()),
                alternative_id: None,
            },
        }
    }
}
