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
    organization::{Identifier, Organization, Telecom},
    primitives::Id,
};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

use super::{
    super::super::constants::{
        CODING_SYSTEM_IDENTIFIER_BASE, CODING_SYSTEM_V2_0203, CONTACT_POINT_SYSTEM_EMAIL,
        CONTACT_POINT_SYSTEM_FAX, CONTACT_POINT_SYSTEM_PHONE, IDENTITY_SYSTEM_BSNR,
        IDENTITY_SYSTEM_IKNR, IDENTITY_SYSTEM_ZANR, ORGANIZATION_IDENTIFIER_CODE_BSNR,
        ORGANIZATION_IDENTIFIER_CODE_IKNR, ORGANIZATION_IDENTIFIER_CODE_ZANR,
        RESOURCE_PROFILE_ORGANIZATION, XMLNS_ORGANIZATION,
    },
    misc::{
        AddressCow, CodableConceptDef, CodingDef, ContactPointDef, DeserializeRoot, IdentifierDef,
        MetaDef, SerializeRoot, XmlnsType,
    },
    primitives::IdDef,
};

pub struct OrganizationDef;

#[serde(rename = "Organization")]
#[derive(Serialize, Deserialize)]
pub struct OrganizationCow<'a>(#[serde(with = "OrganizationDef")] pub Cow<'a, Organization>);

#[serde(rename = "Organization")]
#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize)]
struct OrganizationHelper<'a> {
    #[serde(with = "IdDef")]
    id: Id,

    meta: MetaDef,

    identifier: Vec<IdentifierDef>,

    #[serde(alias = "name")]
    #[serde(rename = "value-tag=name")]
    name: Option<String>,

    telecom: TelecomDef,

    address: Vec<AddressCow<'a>>,
}

#[derive(Serialize, Deserialize)]
struct TelecomDef(Vec<ContactPointDef>);

impl XmlnsType for Organization {
    fn xmlns() -> &'static str {
        XMLNS_ORGANIZATION
    }
}

impl<'a> SerializeRoot<'a> for OrganizationCow<'a> {
    type Inner = Organization;

    fn from_inner(inner: &'a Self::Inner) -> Self {
        OrganizationCow(Cow::Borrowed(inner))
    }
}

impl DeserializeRoot for OrganizationCow<'_> {
    type Inner = Organization;

    fn into_inner(self) -> Self::Inner {
        self.0.into_owned()
    }
}

impl OrganizationDef {
    pub fn serialize<S: Serializer>(
        organization: &Organization,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let value: OrganizationHelper = organization.into();

        value.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Cow<'static, Organization>, D::Error> {
        let value = OrganizationHelper::deserialize(deserializer)?;

        Ok(Cow::Owned(value.try_into().map_err(D::Error::custom)?))
    }
}

impl<'a> Into<OrganizationHelper<'a>> for &'a Organization {
    fn into(self) -> OrganizationHelper<'a> {
        OrganizationHelper {
            id: self.id.clone(),
            meta: MetaDef {
                profile: vec![RESOURCE_PROFILE_ORGANIZATION.into()],
                ..Default::default()
            },
            identifier: self
                .identifier
                .clone()
                .into_iter()
                .map(Into::into)
                .collect(),
            name: self.name.as_ref().map(Clone::clone),
            telecom: (&self.telecom).into(),
            address: self
                .address
                .as_ref()
                .into_iter()
                .map(AddressCow::borrowed)
                .collect(),
        }
    }
}

impl Into<IdentifierDef> for Identifier {
    fn into(self) -> IdentifierDef {
        match self {
            Identifier::IK(value) => IdentifierDef {
                type_: Some(CodableConceptDef {
                    coding: vec![CodingDef {
                        system: Some(CODING_SYSTEM_V2_0203.into()),
                        code: Some(ORGANIZATION_IDENTIFIER_CODE_IKNR.into()),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                system: Some(IDENTITY_SYSTEM_IKNR.into()),
                value: Some(value),
                ..Default::default()
            },
            Identifier::BS(value) => IdentifierDef {
                type_: Some(CodableConceptDef {
                    coding: vec![CodingDef {
                        system: Some(CODING_SYSTEM_V2_0203.into()),
                        code: Some(ORGANIZATION_IDENTIFIER_CODE_BSNR.into()),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                system: Some(IDENTITY_SYSTEM_BSNR.into()),
                value: Some(value),
                ..Default::default()
            },
            Identifier::KZV(value) => IdentifierDef {
                type_: Some(CodableConceptDef {
                    coding: vec![CodingDef {
                        system: Some(CODING_SYSTEM_IDENTIFIER_BASE.into()),
                        code: Some(ORGANIZATION_IDENTIFIER_CODE_ZANR.into()),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                system: Some(IDENTITY_SYSTEM_ZANR.into()),
                value: Some(value),
                ..Default::default()
            },
        }
    }
}

impl Into<TelecomDef> for &Telecom {
    fn into(self) -> TelecomDef {
        let mut ret = vec![ContactPointDef {
            system: Some(CONTACT_POINT_SYSTEM_PHONE.into()),
            value: Some(self.phone.clone()),
            ..Default::default()
        }];

        if let Some(fax) = &self.fax {
            ret.push(ContactPointDef {
                system: Some(CONTACT_POINT_SYSTEM_FAX.into()),
                value: Some(fax.clone()),
                ..Default::default()
            })
        }

        if let Some(mail) = &self.mail {
            ret.push(ContactPointDef {
                system: Some(CONTACT_POINT_SYSTEM_EMAIL.into()),
                value: Some(mail.clone()),
                ..Default::default()
            })
        }

        TelecomDef(ret)
    }
}

impl TryInto<Organization> for OrganizationHelper<'_> {
    type Error = String;

    fn try_into(self) -> Result<Organization, Self::Error> {
        Ok(Organization {
            id: self.id,
            name: self.name,
            identifier: self
                .identifier
                .into_iter()
                .next()
                .map(TryInto::try_into)
                .transpose()?,
            telecom: self.telecom.try_into()?,
            address: self.address.into_iter().next().map(AddressCow::into_owned),
        })
    }
}

impl TryInto<Identifier> for IdentifierDef {
    type Error = String;

    fn try_into(self) -> Result<Identifier, Self::Error> {
        match self.system.as_deref() {
            Some(IDENTITY_SYSTEM_IKNR) => {
                Ok(Identifier::IK(self.value.ok_or_else(|| {
                    "Organization identifier is missig the `value` field!"
                })?))
            }
            Some(IDENTITY_SYSTEM_BSNR) => {
                Ok(Identifier::BS(self.value.ok_or_else(|| {
                    "Organization identifier is missig the `value` field!"
                })?))
            }
            Some(IDENTITY_SYSTEM_ZANR) => {
                Ok(Identifier::KZV(self.value.ok_or_else(|| {
                    "Organization identifier is missig the `value` field!"
                })?))
            }
            Some(system) => Err(format!(
                "Organization identifier has invalid system: {}!",
                system
            )),
            None => Err("Organization identifier is missig the `system` field!".to_owned()),
        }
    }
}

impl TryInto<Telecom> for TelecomDef {
    type Error = String;

    fn try_into(self) -> Result<Telecom, Self::Error> {
        let mut phone = None;
        let mut fax = None;
        let mut mail = None;

        for telecom in self.0 {
            match telecom.system.as_deref() {
                Some(CONTACT_POINT_SYSTEM_PHONE) => {
                    phone = Some(
                        telecom
                            .value
                            .ok_or_else(|| "Organization telecom is missing the `value` field!")?,
                    )
                }
                Some(CONTACT_POINT_SYSTEM_FAX) => {
                    fax = Some(
                        telecom
                            .value
                            .ok_or_else(|| "Organization telecom is missing the `value` field!")?,
                    )
                }
                Some(CONTACT_POINT_SYSTEM_EMAIL) => {
                    mail = Some(
                        telecom
                            .value
                            .ok_or_else(|| "Organization telecom is missing the `value` field!")?,
                    )
                }
                Some(system) => {
                    return Err(format!(
                        "Organization telecom has invalid system: {}!",
                        system
                    ))
                }
                None => {
                    return Err("Organization telecom is missing the `system` field!".to_owned())
                }
            }
        }

        Ok(Telecom {
            phone: phone.ok_or_else(|| "Orgranization telecom is missing the `phone` field!")?,
            fax,
            mail,
        })
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::convert::TryInto;
    use std::fs::read_to_string;

    use resources::misc::Address;

    use crate::fhir::{
        test::trim_xml_str,
        xml::{from_str as from_xml, to_string as to_xml},
    };

    use super::super::misc::Root;

    type OrganizationRoot<'a> = Root<OrganizationCow<'a>>;

    #[test]
    fn convert_to() {
        let organization = test_organization();

        let actual = trim_xml_str(&to_xml(&OrganizationRoot::new(&organization)).unwrap());
        let expected = trim_xml_str(&read_to_string("./examples/organization.xml").unwrap());

        assert_eq!(actual, expected);
    }

    #[test]
    fn convert_from() {
        let actual =
            from_xml::<OrganizationRoot>(&read_to_string("./examples/organization.xml").unwrap())
                .unwrap()
                .into_inner();
        let expected = test_organization();

        assert_eq!(actual, expected);
    }

    pub fn test_organization() -> Organization {
        Organization {
            id: "cf042e44-086a-4d51-9c77-172f9a972e3b".try_into().unwrap(),
            name: Some("Hausarztpraxis Dr. Topp-Glücklich".into()),
            identifier: Some(Identifier::BS("031234567".into())),
            telecom: Telecom {
                phone: "0301234567".into(),
                fax: None,
                mail: None,
            },
            address: Some(Address {
                address: "Musterstr. 2".into(),
                street: Some("Musterstr.".into()),
                number: Some("2".into()),
                addition: None,
                post_box: None,
                city: Some("Berlin".into()),
                zip_code: Some("10623".into()),
                country: None,
            }),
        }
    }
}
