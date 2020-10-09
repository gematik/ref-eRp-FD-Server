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
    bundle::{
        Bundle, Entry as BundleEntry, Identifier as BundleIdentifier, Meta as BundleMeta,
        Type as BundleType,
    },
    kbv_bundle::{Entry, Meta},
    Composition, Coverage, KbvBundle, Medication, MedicationRequest, Organization, Patient,
    Practitioner, PractitionerRole,
};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

use super::{
    super::super::constants::{
        IDENTIFIER_SYSTEM_PRESCRIPTION_ID, RESOURCE_PROFILE_KBV_BUNDLE, XMLNS_KBV_BUNDLE,
    },
    misc::{DeserializeRoot, Root, SerializeRoot, XmlnsType},
    BundleCow, CompositionDef, CoverageDef, MedicationDef, MedicationRequestDef, OrganizationDef,
    PatientDef, PractitionerDef, PractitionerRoleDef,
};

pub struct KbvBundleDef;
pub type KbvBundleRoot<'a> = Root<KbvBundleCow<'a>>;

#[serde(rename = "Bundle")]
#[derive(Clone, Serialize, Deserialize)]
pub struct KbvBundleCow<'a>(#[serde(with = "KbvBundleDef")] pub Cow<'a, KbvBundle>);

struct BundleDef<'a>(Bundle<ResourceDef<'a>>);

#[derive(Clone, Serialize, Deserialize)]
enum ResourceDef<'a> {
    Composition(#[serde(with = "CompositionDef")] Cow<'a, Composition>),
    MedicationRequest(#[serde(with = "MedicationRequestDef")] Cow<'a, MedicationRequest>),
    Medication(#[serde(with = "MedicationDef")] Cow<'a, Medication>),
    Patient(#[serde(with = "PatientDef")] Cow<'a, Patient>),
    Practitioner(#[serde(with = "PractitionerDef")] Cow<'a, Practitioner>),
    Organization(#[serde(with = "OrganizationDef")] Cow<'a, Organization>),
    Coverage(#[serde(with = "CoverageDef")] Cow<'a, Coverage>),
    PractitionerRole(#[serde(with = "PractitionerRoleDef")] Cow<'a, PractitionerRole>),
}

impl XmlnsType for KbvBundle {
    fn xmlns() -> &'static str {
        XMLNS_KBV_BUNDLE
    }
}

impl<'a> SerializeRoot<'a> for KbvBundleCow<'a> {
    type Inner = KbvBundle;

    fn from_inner(inner: &'a Self::Inner) -> Self {
        KbvBundleCow(Cow::Borrowed(inner))
    }
}

impl DeserializeRoot for KbvBundleCow<'_> {
    type Inner = KbvBundle;

    fn into_inner(self) -> Self::Inner {
        self.0.into_owned()
    }
}

impl KbvBundleDef {
    pub fn serialize<'a, S: Serializer>(
        kbv_bundle: &'a KbvBundle,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let value: BundleDef<'a> = kbv_bundle.into();
        let value = BundleCow(Cow::Owned(value.0));

        value.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Cow<'static, KbvBundle>, D::Error> {
        let value = BundleCow::<'static, ResourceDef>::deserialize(deserializer)?
            .0
            .into_owned();
        let value = BundleDef(value);

        Ok(Cow::Owned(value.try_into().map_err(D::Error::custom)?))
    }
}

impl<'a> Into<BundleDef<'a>> for &'a KbvBundle {
    fn into(self) -> BundleDef<'a> {
        let mut entries = Vec::new();

        create(
            &mut entries,
            &self.entry.composition,
            ResourceDef::Composition,
        );
        create(
            &mut entries,
            &self.entry.medication_request,
            ResourceDef::MedicationRequest,
        );
        create(
            &mut entries,
            &self.entry.medication,
            ResourceDef::Medication,
        );
        create(&mut entries, &self.entry.patient, ResourceDef::Patient);
        create(
            &mut entries,
            &self.entry.practitioner,
            ResourceDef::Practitioner,
        );
        create(
            &mut entries,
            &self.entry.organization,
            ResourceDef::Organization,
        );
        create(&mut entries, &self.entry.coverage, ResourceDef::Coverage);
        create(
            &mut entries,
            &self.entry.practitioner_role,
            ResourceDef::PractitionerRole,
        );

        BundleDef(Bundle {
            id: Some(self.id.clone()),
            meta: Some(BundleMeta {
                last_updated: self.meta.last_updated.clone(),
                profile: vec![RESOURCE_PROFILE_KBV_BUNDLE.into()],
            }),
            identifier: Some(BundleIdentifier {
                system: Some(IDENTIFIER_SYSTEM_PRESCRIPTION_ID.into()),
                value: Some(self.identifier.to_string()),
            }),
            type_: BundleType::Document,
            timestamp: Some(self.timestamp.clone()),
            entries,
        })
    }
}

impl TryInto<KbvBundle> for BundleDef<'static> {
    type Error = String;

    fn try_into(self) -> Result<KbvBundle, Self::Error> {
        let meta = self
            .0
            .meta
            .ok_or_else(|| "KBV bundle is missing the `meta` field!")?;

        if meta.profile != vec![RESOURCE_PROFILE_KBV_BUNDLE] {
            return Err("KBV bundle has an invalid profile!".to_owned());
        }

        let identifier = self
            .0
            .identifier
            .ok_or_else(|| "KBV bundle is missing the `identifier` field!")?;

        match identifier.system.as_deref() {
            Some(IDENTIFIER_SYSTEM_PRESCRIPTION_ID) => (),
            Some(system) => {
                return Err(format!(
                    "KBV bundle identifier has an unexpected system: {}!",
                    system
                ))
            }
            None => return Err("KBV bundle identifier is missing the `system` field!".to_owned()),
        }

        let identifier = identifier
            .value
            .ok_or_else(|| "KBV bundle identifier is missing the `value` field!")?
            .parse()
            .map_err(|err| format!("KBV bundle contains invalid identifier: {}!", err))?;

        let mut entry = Entry::default();
        for e in self.0.entries {
            match e.resource {
                ResourceDef::Composition(v) => update(&mut entry.composition, e.url, v)?,
                ResourceDef::MedicationRequest(v) => {
                    update(&mut entry.medication_request, e.url, v)?
                }
                ResourceDef::Medication(v) => update(&mut entry.medication, e.url, v)?,
                ResourceDef::Patient(v) => update(&mut entry.patient, e.url, v)?,
                ResourceDef::Practitioner(v) => update(&mut entry.practitioner, e.url, v)?,
                ResourceDef::Organization(v) => update(&mut entry.organization, e.url, v)?,
                ResourceDef::Coverage(v) => update(&mut entry.coverage, e.url, v)?,
                ResourceDef::PractitionerRole(v) => update(&mut entry.practitioner_role, e.url, v)?,
            }
        }

        Ok(KbvBundle {
            id: self
                .0
                .id
                .ok_or_else(|| "KBV bundle is missing the `id` field!")?,
            meta: Meta {
                last_updated: meta.last_updated,
            },
            identifier,
            timestamp: self
                .0
                .timestamp
                .ok_or_else(|| "KBV bundle is missing the `timestampt` field!")?,
            entry,
        })
    }
}

fn create<'a, T: Clone, F: FnOnce(Cow<'a, T>) -> ResourceDef>(
    entries: &mut Vec<BundleEntry<ResourceDef<'a>>>,
    value: &'a Option<(String, T)>,
    f: F,
) {
    if let Some((url, res)) = value {
        let mut entry = BundleEntry::new(f(Cow::Borrowed(res)));
        entry.url = Some(url.clone());

        entries.push(entry);
    }
}

fn update<'a, T: Clone>(
    storage: &mut Option<(String, T)>,
    url: Option<String>,
    res: Cow<'a, T>,
) -> Result<(), String> {
    if storage.is_some() {
        return Err("KBV bundle has duplicate resource!".to_owned());
    }

    let url = url.ok_or_else(|| "KBV bundle entry is missing the `fullUrl` field!")?;

    *storage = Some((url, res.into_owned()));

    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::convert::TryInto;
    use std::fs::read_to_string;

    use chrono::DateTime;
    use resources::{
        kbv_bundle::{Entry, Meta},
        misc::PrescriptionId,
        types::FlowType,
    };

    use crate::fhir::{
        test::trim_xml_str,
        xml::{from_str as from_xml, to_string as to_xml},
    };

    use super::super::{
        composition::tests::test_composition, coverage::tests::test_coverage,
        medication::tests::test_medication_pzn, medication_request::tests::test_medication_request,
        organization::tests::test_organization, patient::tests::test_patient,
        practitioner::tests::test_practitioner, practitioner_role::tests::test_practitioner_role,
    };

    #[test]
    fn convert_to() {
        let bundle = test_kbv_bundle();

        let actual = trim_xml_str(&to_xml(&KbvBundleRoot::new(&bundle)).unwrap());
        let expected = trim_xml_str(&read_to_string("./examples/kbv_bundle.xml").unwrap());

        assert_eq!(actual, expected);
    }

    #[test]
    fn convert_from() {
        let actual =
            from_xml::<KbvBundleRoot>(&read_to_string("./examples/kbv_bundle.xml").unwrap())
                .unwrap()
                .into_inner();
        let expected = test_kbv_bundle();

        assert_eq!(actual, expected);
    }

    pub fn test_kbv_bundle() -> KbvBundle {
        let composition_url = "http://pvs.praxis-topp-gluecklich.local/fhir/Composition/ed52c1e3-b700-4497-ae19-b23744e29876".into();
        let medication_request_url = "http://pvs.praxis-topp-gluecklich.local/fhir/MedicationRequest/e930cdee-9eb5-4b44-88b5-2a18b69f3b9a".into();
        let medication_url = "http://pvs.praxis-topp-gluecklich.local/fhir/Medication/5fe6e06c-8725-46d5-aecd-e65e041ca3de".into();
        let patient_url = "http://pvs.praxis-topp-gluecklich.local/fhir/Patient/9774f67f-a238-4daf-b4e6-679deeef3811".into();
        let practitioner_url = "http://pvs.praxis-topp-gluecklich.local/fhir/Practitioner/20597e0e-cb2a-45b3-95f0-dc3dbdb617c3".into();
        let organization_url = "http://pvs.praxis-topp-gluecklich.local/fhir/Organization/cf042e44-086a-4d51-9c77-172f9a972e3b".into();
        let coverage_url = "http://pvs.praxis-topp-gluecklich.local/fhir/Coverage/1b1ffb6e-eb05-43d7-87eb-e7818fe9661a".into();
        let practitioner_role_url = "http://pvs.praxis-topp-gluecklich.local/fhir/PractitionerRole/9a4090f8-8c5a-11ea-bc55-0242ac13000".into();

        KbvBundle {
            id: "281a985c-f25b-4aae-91a6-41ad744080b0".try_into().unwrap(),
            meta: Meta {
                last_updated: Some(
                    DateTime::parse_from_rfc3339("2020-05-04T08:30:00Z")
                        .unwrap()
                        .into(),
                ),
            },
            identifier: PrescriptionId::new(FlowType::PharmaceuticalDrugs, 123456789123),
            timestamp: DateTime::parse_from_rfc3339("2020-06-23T08:30:00Z")
                .unwrap()
                .into(),
            entry: Entry {
                composition: Some((composition_url, test_composition())),
                medication_request: Some((medication_request_url, test_medication_request())),
                medication: Some((medication_url, test_medication_pzn())),
                patient: Some((patient_url, test_patient())),
                practitioner: Some((practitioner_url, test_practitioner())),
                organization: Some((organization_url, test_organization())),
                coverage: Some((coverage_url, test_coverage())),
                practitioner_role: Some((practitioner_role_url, test_practitioner_role())),
            },
        }
    }
}
