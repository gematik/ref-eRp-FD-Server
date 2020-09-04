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
    composition::{Author, Extension, LegalBasis, Section},
    misc::{DecodeStr, EncodeStr},
    primitives::{DateTime, Id},
    Composition,
};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

use super::{
    super::super::constants::{
        CODING_SYSTEM_COMPOSITION, CODING_SYSTEM_LEGAL_BASIS, CODING_SYSTEM_SECTION,
        COMPOSITION_ATTESTER_MODE, COMPOSITION_CODE_SECTION_COVERAGE,
        COMPOSITION_CODE_SECTION_PRACTITIONER_ROLE, COMPOSITION_CODE_SECTION_REGULATION,
        COMPOSITION_STATUS, COMPOSITION_TYPE_AUTHOR_DOCTOR, COMPOSITION_TYPE_AUTHOR_PRF,
        COMPOSITION_TYPE_CODE, EXTENSION_URL_LEGAL_BASIS, IDENTIFIER_SYSTEM_PRF,
        RESOURCE_PROFILE_COMPOSITION, XMLNS_COMPOSITION,
    },
    misc::{
        CodableConceptDef, CodingDef, DeserializeRoot, ExtensionDef, IdentifierDef, MetaDef,
        ReferenceDef, SerializeRoot, ValueDef, XmlnsType,
    },
    primitives::{DateTimeDef, IdDef},
};

pub struct CompositionDef;

#[derive(Serialize, Deserialize)]
#[serde(rename = "Composition")]
pub struct CompositionCow<'a>(#[serde(with = "CompositionDef")] Cow<'a, Composition>);

#[serde(rename = "Composition")]
#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize)]
struct CompositionHelper {
    #[serde(with = "IdDef")]
    pub id: Id,

    pub meta: MetaDef,

    pub extension: ExtensionsDef,

    #[serde(alias = "status")]
    #[serde(rename = "value-tag=status")]
    pub status: String,

    pub type_: CodableConceptDef,

    pub subject: Option<ReferenceDef>,

    #[serde(with = "DateTimeDef")]
    pub date: DateTime,

    pub author: AuthorDef,

    #[serde(alias = "title")]
    #[serde(rename = "value-tag=title")]
    pub title: String,

    pub attester: Option<AttesterDef>,

    pub custodian: ReferenceDef,

    pub section: SectionsDef,
}

#[derive(Serialize, Deserialize)]
struct ExtensionsDef(Vec<ExtensionDef>);

#[derive(Serialize, Deserialize)]
struct AuthorDef(Vec<ReferenceDef>);

#[derive(Serialize, Deserialize)]
struct SectionsDef(Vec<SectionDef>);

#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize)]
struct AttesterDef {
    pub mode: String,
    pub party: ReferenceDef,
}

#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize)]
struct SectionDef {
    pub code: CodableConceptDef,
    pub entry: Vec<ReferenceDef>,
}

impl XmlnsType for Composition {
    fn xmlns() -> &'static str {
        XMLNS_COMPOSITION
    }
}

impl<'a> SerializeRoot<'a> for CompositionCow<'a> {
    type Inner = Composition;

    fn from_inner(inner: &'a Self::Inner) -> Self {
        CompositionCow(Cow::Borrowed(inner))
    }
}

impl DeserializeRoot for CompositionCow<'_> {
    type Inner = Composition;

    fn into_inner(self) -> Self::Inner {
        self.0.into_owned()
    }
}

impl CompositionDef {
    pub fn serialize<S: Serializer>(
        composition: &Composition,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let value: CompositionHelper = composition.into();

        value.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Cow<'static, Composition>, D::Error> {
        let value = CompositionHelper::deserialize(deserializer)?;

        Ok(Cow::Owned(value.try_into().map_err(D::Error::custom)?))
    }
}

impl Into<CompositionHelper> for &Composition {
    fn into(self) -> CompositionHelper {
        CompositionHelper {
            id: self.id.clone(),
            meta: MetaDef {
                profile: vec![RESOURCE_PROFILE_COMPOSITION.into()],
                ..Default::default()
            },
            extension: (&self.extension).into(),
            status: COMPOSITION_STATUS.into(),
            type_: CodableConceptDef {
                coding: vec![CodingDef {
                    system: Some(CODING_SYSTEM_COMPOSITION.into()),
                    code: Some(COMPOSITION_TYPE_CODE.into()),
                    ..Default::default()
                }],
                ..Default::default()
            },
            subject: self.subject.clone().map(|subject| ReferenceDef {
                reference: Some(subject),
                ..Default::default()
            }),
            date: self.date.clone(),
            author: (&self.author).into(),
            title: self.title.clone(),
            attester: self.attester.clone().map(|attester| AttesterDef {
                mode: COMPOSITION_ATTESTER_MODE.into(),
                party: ReferenceDef {
                    reference: Some(attester),
                    ..Default::default()
                },
            }),
            custodian: ReferenceDef {
                reference: Some(self.custodian.clone()),
                ..Default::default()
            },
            section: (&self.section).into(),
        }
    }
}

impl Into<ExtensionsDef> for &Extension {
    fn into(self) -> ExtensionsDef {
        let mut ret = Vec::new();

        if let Some(legal_basis) = &self.legal_basis {
            ret.push(ExtensionDef {
                url: EXTENSION_URL_LEGAL_BASIS.into(),
                value: Some(ValueDef::Coding(CodingDef {
                    system: Some(CODING_SYSTEM_LEGAL_BASIS.into()),
                    code: Some(legal_basis.encode_str()),
                    ..Default::default()
                })),
                ..Default::default()
            })
        }

        ExtensionsDef(ret)
    }
}

impl Into<AuthorDef> for &Author {
    fn into(self) -> AuthorDef {
        let mut ret = vec![ReferenceDef {
            type_: Some(COMPOSITION_TYPE_AUTHOR_DOCTOR.into()),
            reference: Some(self.doctor.clone()),
            ..Default::default()
        }];

        if let Some(prf) = &self.prf {
            ret.push(ReferenceDef {
                type_: Some(COMPOSITION_TYPE_AUTHOR_PRF.into()),
                identifier: Some(IdentifierDef {
                    system: Some(IDENTIFIER_SYSTEM_PRF.into()),
                    value: Some(prf.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            })
        }

        AuthorDef(ret)
    }
}

impl Into<SectionsDef> for &Section {
    fn into(self) -> SectionsDef {
        let mut ret = vec![SectionDef {
            code: CodableConceptDef {
                coding: vec![CodingDef {
                    system: Some(CODING_SYSTEM_SECTION.into()),
                    code: Some(COMPOSITION_CODE_SECTION_REGULATION.into()),
                    ..Default::default()
                }],
                ..Default::default()
            },
            entry: vec![ReferenceDef {
                reference: Some(self.regulation.clone()),
                ..Default::default()
            }],
        }];

        if let Some(value) = &self.health_insurance_relationship {
            ret.push(SectionDef {
                code: CodableConceptDef {
                    coding: vec![CodingDef {
                        system: Some(CODING_SYSTEM_SECTION.into()),
                        code: Some(COMPOSITION_CODE_SECTION_COVERAGE.into()),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
                entry: vec![ReferenceDef {
                    reference: Some(value.clone()),
                    ..Default::default()
                }],
            })
        }

        if let Some(value) = &self.asv_performance {
            ret.push(SectionDef {
                code: CodableConceptDef {
                    coding: vec![CodingDef {
                        system: Some(CODING_SYSTEM_SECTION.into()),
                        code: Some(COMPOSITION_CODE_SECTION_PRACTITIONER_ROLE.into()),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
                entry: vec![ReferenceDef {
                    reference: Some(value.clone()),
                    ..Default::default()
                }],
            })
        }

        SectionsDef(ret)
    }
}

impl TryInto<Composition> for CompositionHelper {
    type Error = String;

    fn try_into(self) -> Result<Composition, Self::Error> {
        if self.meta.profile != vec![RESOURCE_PROFILE_COMPOSITION.into()] {
            return Err("Composition has an invalid profile".to_owned());
        }

        Ok(Composition {
            id: self.id,
            extension: self.extension.try_into()?,
            subject: self
                .subject
                .map(|subject| {
                    subject
                        .reference
                        .ok_or_else(|| "Subject is missing the `reference` field!")
                })
                .transpose()?,
            date: self.date,
            author: self.author.try_into()?,
            title: self.title,
            attester: self
                .attester
                .map(|attester| {
                    if attester.mode != COMPOSITION_ATTESTER_MODE {
                        Err("Attester has invalid mode!".to_owned())
                    } else if let Some(value) = attester.party.reference {
                        Ok(value)
                    } else {
                        Err("Attester party is missing the `reference` field!".to_owned())
                    }
                })
                .transpose()?,
            custodian: self
                .custodian
                .reference
                .ok_or_else(|| "Custodian is missing the `reference` field!")?,
            section: self.section.try_into()?,
        })
    }
}

impl TryInto<Extension> for ExtensionsDef {
    type Error = String;

    fn try_into(self) -> Result<Extension, Self::Error> {
        let mut legal_basis = None;

        for ex in self.0 {
            if ex.url == EXTENSION_URL_LEGAL_BASIS {
                let coding = match ex.value {
                    Some(ValueDef::Coding(coding)) => Ok(coding),
                    _ => Err("Extension is missing the `valueCoding` field!"),
                }?;

                match coding.system.as_deref() {
                    Some(CODING_SYSTEM_LEGAL_BASIS) => Ok(()),
                    Some(system) => Err(format!(
                        "Extension coding has an invalid system: {}!",
                        system
                    )),
                    None => Err("Extension coding is missing the `system` field!".to_owned()),
                }?;

                legal_basis = match coding.code.as_deref().map(LegalBasis::decode_str) {
                    Some(Ok(legal_basis)) => Some(legal_basis),
                    Some(Err(err)) => {
                        return Err(format!("Extension coding contains invalid code: {}!", err))
                    }
                    None => return Err("Extension coding is missing the `code` field!".to_owned()),
                }
            }
        }

        Ok(Extension { legal_basis })
    }
}

impl TryInto<Author> for AuthorDef {
    type Error = String;

    fn try_into(self) -> Result<Author, Self::Error> {
        let mut doctor = None;
        let mut prf = None;

        for author in self.0 {
            let type_ = author
                .type_
                .ok_or_else(|| "Author is missing the `type` field!")?;

            if type_ == COMPOSITION_TYPE_AUTHOR_DOCTOR {
                doctor = author.reference;
            } else if type_ == COMPOSITION_TYPE_AUTHOR_PRF {
                let identifier = author
                    .identifier
                    .ok_or_else(|| "Author is missing the `identifier` field!")?;

                match identifier.system.as_deref() {
                    Some(IDENTIFIER_SYSTEM_PRF) => (),
                    Some(system) => {
                        return Err(format!(
                            "Author identifier has unexpected system: {}!",
                            system
                        ))
                    }
                    None => {
                        return Err("Author identifier is missing the `system` field!".to_owned())
                    }
                }

                let value = identifier
                    .value
                    .ok_or_else(|| "Author identifier is missing the `value` field!")?;

                prf = Some(value);
            }
        }

        Ok(Author {
            doctor: doctor.ok_or_else(|| "Composition author is missing the `doctor` field!")?,
            prf,
        })
    }
}

impl TryInto<Section> for SectionsDef {
    type Error = String;

    fn try_into(self) -> Result<Section, Self::Error> {
        let mut regulation = None;
        let mut health_insurance_relationship = None;
        let mut asv_performance = None;

        for section in self.0 {
            let coding = section
                .code
                .coding
                .into_iter()
                .next()
                .ok_or_else(|| "Section code is missing the `coding` field!")?;

            let entry = section
                .entry
                .into_iter()
                .next()
                .ok_or_else(|| "Section is missing the `entry` field!")?;

            match coding.system.as_deref() {
                Some(CODING_SYSTEM_SECTION) => (),
                Some(system) => {
                    return Err(format!("Section coding has unexpected system: {}!", system))
                }
                None => return Err("Section coding is missing the `system` field!".to_owned()),
            }

            let code = coding
                .code
                .ok_or_else(|| "Section coding is missing the `code` field!")?;

            match code.as_str() {
                COMPOSITION_CODE_SECTION_REGULATION => {
                    regulation = Some(
                        entry
                            .reference
                            .ok_or_else(|| "Section entry is missing the `reference` field!")?,
                    )
                }
                COMPOSITION_CODE_SECTION_COVERAGE => {
                    health_insurance_relationship = Some(
                        entry
                            .reference
                            .ok_or_else(|| "Section entry is missing the `reference` field!")?,
                    )
                }
                COMPOSITION_CODE_SECTION_PRACTITIONER_ROLE => {
                    asv_performance = Some(
                        entry
                            .reference
                            .ok_or_else(|| "Section entry is missing the `reference` field!")?,
                    )
                }
                _ => (),
            }
        }

        Ok(Section {
            regulation: regulation
                .ok_or_else(|| "Composition is missing the regulation section!")?,
            health_insurance_relationship,
            asv_performance,
        })
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::convert::TryInto;
    use std::fs::read_to_string;

    use resources::composition::{Author, Extension, LegalBasis, Section};

    use crate::fhir::{
        test::trim_xml_str,
        xml::{from_str as from_xml, to_string as to_xml},
    };

    use super::super::misc::Root;

    type CompositionRoot<'a> = Root<CompositionCow<'a>>;

    #[test]
    fn convert_to() {
        let bundle = test_composition();

        let actual = trim_xml_str(&to_xml(&CompositionRoot::new(&bundle)).unwrap());
        let expected = trim_xml_str(&read_to_string("./examples/composition.xml").unwrap());

        assert_eq!(actual, expected);
    }

    #[test]
    fn convert_from() {
        let xml = read_to_string("./examples/composition.xml").unwrap();
        let actual = from_xml::<CompositionRoot>(&xml).unwrap().into_inner();
        let expected = test_composition();

        assert_eq!(actual, expected);
    }

    pub fn test_composition() -> Composition {
        Composition {
            id: "ed52c1e3-b700-4497-ae19-b23744e29876".try_into().unwrap(),
            extension: Extension {
                legal_basis: Some(LegalBasis::None),
            },
            subject: Some("Patient/9774f67f-a238-4daf-b4e6-679deeef3811".into()),
            date: "2020-05-04T08:00:00+00:00".try_into().unwrap(),
            author: Author {
                doctor: "Practitioner/20597e0e-cb2a-45b3-95f0-dc3dbdb617c3".into(),
                prf: Some("Y/400/1910/36/346".into()),
            },
            title: "elektronische Arzneimittelverordnung".into(),
            attester: None,
            custodian: "Organization/cf042e44-086a-4d51-9c77-172f9a972e3b".into(),
            section: Section {
                regulation: "MedicationRequest/e930cdee-9eb5-4b44-88b5-2a18b69f3b9a".into(),
                health_insurance_relationship: Some(
                    "Coverage/1b1ffb6e-eb05-43d7-87eb-e7818fe9661a".into(),
                ),
                asv_performance: Some(
                    "PractitionerRole/9a4090f8-8c5a-11ea-bc55-0242ac13000".into(),
                ),
            },
        }
    }
}
