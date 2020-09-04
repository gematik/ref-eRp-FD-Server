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

use resources::{primitives::Id, PractitionerRole};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

use super::{
    super::super::constants::{
        IDENTITY_SYSTEM_TEAM_NUMBER, RESOURCE_PROFILE_PRACTITIONER_ROLE, XMLNS_PRACTITIONER_ROLE,
    },
    misc::{DeserializeRoot, IdentifierDef, MetaDef, ReferenceDef, SerializeRoot, XmlnsType},
    primitives::IdDef,
};

pub struct PractitionerRoleDef;

#[derive(Serialize, Deserialize)]
#[serde(rename = "PractitionerRole")]
pub struct PractitionerRoleCow<'a>(
    #[serde(with = "PractitionerRoleDef")] Cow<'a, PractitionerRole>,
);

#[serde(rename = "PractitionerRole")]
#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize)]
struct PractitionerRoleHelper {
    #[serde(with = "IdDef")]
    id: Id,
    meta: MetaDef,
    practitioner: ReferenceDef,
    organization: ReferenceDef,
}

impl XmlnsType for PractitionerRole {
    fn xmlns() -> &'static str {
        XMLNS_PRACTITIONER_ROLE
    }
}

impl<'a> SerializeRoot<'a> for PractitionerRoleCow<'a> {
    type Inner = PractitionerRole;

    fn from_inner(inner: &'a Self::Inner) -> Self {
        PractitionerRoleCow(Cow::Borrowed(inner))
    }
}

impl DeserializeRoot for PractitionerRoleCow<'_> {
    type Inner = PractitionerRole;

    fn into_inner(self) -> Self::Inner {
        self.0.into_owned()
    }
}

impl PractitionerRoleDef {
    pub fn serialize<S: Serializer>(
        practitioner_role: &PractitionerRole,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let value: PractitionerRoleHelper = practitioner_role.into();

        value.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Cow<'static, PractitionerRole>, D::Error> {
        let value = PractitionerRoleHelper::deserialize(deserializer)?;

        Ok(Cow::Owned(value.try_into().map_err(D::Error::custom)?))
    }
}

impl Into<PractitionerRoleHelper> for &PractitionerRole {
    fn into(self) -> PractitionerRoleHelper {
        PractitionerRoleHelper {
            id: self.id.clone(),
            meta: MetaDef {
                profile: vec![RESOURCE_PROFILE_PRACTITIONER_ROLE.into()],
                ..Default::default()
            },
            practitioner: ReferenceDef {
                reference: Some(self.practitioner.clone()),
                ..Default::default()
            },
            organization: ReferenceDef {
                identifier: Some(IdentifierDef {
                    system: Some(IDENTITY_SYSTEM_TEAM_NUMBER.into()),
                    value: Some(self.organization.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            },
        }
    }
}

impl TryInto<PractitionerRole> for PractitionerRoleHelper {
    type Error = String;

    fn try_into(self) -> Result<PractitionerRole, Self::Error> {
        let ident = self
            .organization
            .identifier
            .ok_or_else(|| "Practitioner role organization is missing the `identifier` field!")?;

        match ident.system.as_deref() {
            Some(IDENTITY_SYSTEM_TEAM_NUMBER) => (),
            Some(system) => {
                return Err(format!(
                    "Practitioner role organization identifier has invalid system: {}!",
                    system
                ))
            }
            None => {
                return Err(
                    "Practitioner role organization identifier is missing the `system` field!"
                        .to_owned(),
                )
            }
        }

        Ok(PractitionerRole {
            id: self.id,
            practitioner: self.practitioner.reference.ok_or_else(|| {
                "Practitioner role practitioner is missing the `reference` field!"
            })?,
            organization: ident
                .value
                .ok_or_else(|| "Practitioner role organization is missing the `value` field!")?,
        })
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::convert::TryInto;
    use std::fs::read_to_string;

    use crate::fhir::{
        test::trim_xml_str,
        xml::{from_str as from_xml, to_string as to_xml},
    };

    use super::super::misc::Root;

    type PractitionerRoleRoot<'a> = Root<PractitionerRoleCow<'a>>;

    #[test]
    fn convert_to() {
        let bundle = test_practitioner_role();

        let actual = trim_xml_str(&to_xml(&PractitionerRoleRoot::new(&bundle)).unwrap());
        let expected = trim_xml_str(&read_to_string("./examples/practitioner_role.xml").unwrap());

        assert_eq!(actual, expected);
    }

    #[test]
    fn convert_from() {
        let xml = read_to_string("./examples/practitioner_role.xml").unwrap();
        let actual = from_xml::<PractitionerRoleRoot>(&xml).unwrap().into_inner();
        let expected = test_practitioner_role();

        assert_eq!(actual, expected);
    }

    pub fn test_practitioner_role() -> PractitionerRole {
        PractitionerRole {
            id: "9a4090f8-8c5a-11ea-bc55-0242ac13000".try_into().unwrap(),
            practitioner: "Practitioner/20597e0e-cb2a-45b3-95f0-dc3dbdb617c3".into(),
            organization: "003456789".into(),
        }
    }
}
