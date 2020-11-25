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

use async_trait::async_trait;
use miscellaneous::str::icase_eq;
use resources::{
    kbv_bundle::{Entry, KbvBundle},
    Composition, Coverage, Medication, MedicationRequest, Organization, Patient, Practitioner,
    PractitionerRole, SignatureFormat,
};

use crate::fhir::{
    decode::{decode_any, DataStream, Decode, DecodeError, DecodeStream, Fields},
    encode::{encode_any, DataStorage, Encode, EncodeError, EncodeStream},
    Format,
};

use super::{
    meta::Meta,
    primitives::{decode_identifier, encode_identifier},
};

/* Decode */

enum Resource {
    Composition(Composition),
    MedicationRequest(MedicationRequest),
    Medication(Medication),
    Patient(Patient),
    Practitioner(Practitioner),
    Organization(Organization),
    Coverage(Coverage),
    PractitionerRole(PractitionerRole),
}

#[async_trait(?Send)]
impl Decode for KbvBundle {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&[
            "id",
            "meta",
            "identifier",
            "type",
            "timestamp",
            "entry",
            "signature",
        ]);

        stream.root("Bundle").await?;

        let id = stream.decode(&mut fields, decode_any).await?;
        let meta = stream.decode::<Meta, _>(&mut fields, decode_any).await?;
        let identifier = stream.decode(&mut fields, decode_identifier).await?;
        let _type = stream.fixed(&mut fields, "document").await?;
        let timestamp = stream.decode(&mut fields, decode_any).await?;
        let entry = {
            let mut composition = None;
            let mut medication_request = None;
            let mut medication = None;
            let mut patient = None;
            let mut practitioner = None;
            let mut organization = None;
            let mut coverage = None;
            let mut practitioner_role = None;

            loop {
                if stream.begin_substream_vec(&mut fields).await? {
                    stream.element().await?;

                    let mut fields = Fields::new(&["fullUrl", "resource"]);
                    let url = stream.decode(&mut fields, decode_any).await?;
                    let resource = stream.resource(&mut fields, decode_any).await?;

                    match resource {
                        Resource::Composition(v) => composition = Some((url, v)),
                        Resource::MedicationRequest(v) => medication_request = Some((url, v)),
                        Resource::Medication(v) => medication = Some((url, v)),
                        Resource::Patient(v) => patient = Some((url, v)),
                        Resource::Practitioner(v) => practitioner = Some((url, v)),
                        Resource::Organization(v) => organization = Some((url, v)),
                        Resource::Coverage(v) => coverage = Some((url, v)),
                        Resource::PractitionerRole(v) => practitioner_role = Some((url, v)),
                    }

                    stream.end().await?;
                    stream.end_substream().await?;
                } else {
                    break Entry {
                        composition,
                        medication_request,
                        medication,
                        patient,
                        practitioner,
                        organization,
                        coverage,
                        practitioner_role,
                    };
                }
            }
        };
        let signature = stream.decode_vec(&mut fields, decode_any).await?;

        stream.end().await?;

        if !meta.profiles.iter().any(|p| icase_eq(p, PROFILE)) {
            return Err(DecodeError::InvalidProfile {
                actual: meta.profiles,
                expected: vec![PROFILE.into()],
            });
        }

        Ok(KbvBundle {
            id,
            identifier,
            timestamp,
            entry,
            signature,
        })
    }
}

#[async_trait(?Send)]
impl Decode for Resource {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let element = stream.peek_element().await?;

        match element.as_str() {
            "Composition" => Ok(Self::Composition(
                stream.decode(&mut Fields::Any, decode_any).await?,
            )),
            "MedicationRequest" => Ok(Self::MedicationRequest(
                stream.decode(&mut Fields::Any, decode_any).await?,
            )),
            "Medication" => Ok(Self::Medication(
                stream.decode(&mut Fields::Any, decode_any).await?,
            )),
            "Patient" => Ok(Self::Patient(
                stream.decode(&mut Fields::Any, decode_any).await?,
            )),
            "Practitioner" => Ok(Self::Practitioner(
                stream.decode(&mut Fields::Any, decode_any).await?,
            )),
            "Organization" => Ok(Self::Organization(
                stream.decode(&mut Fields::Any, decode_any).await?,
            )),
            "Coverage" => Ok(Self::Coverage(
                stream.decode(&mut Fields::Any, decode_any).await?,
            )),
            "PractitionerRole" => Ok(Self::PractitionerRole(
                stream.decode(&mut Fields::Any, decode_any).await?,
            )),
            _ => Err(DecodeError::UnexpectedElement {
                id: element.into(),
                path: stream.path().into(),
            }),
        }
    }
}

/* Encode */

impl Encode for &KbvBundle {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let meta = Meta {
            profiles: vec![PROFILE.into()],
        };

        let signature =
            self.signature
                .iter()
                .find(|s| match (s.format.as_ref(), stream.format()) {
                    (Some(SignatureFormat::Json), Some(Format::Json)) => true,
                    (_, _) => false,
                });

        stream
            .root("Bundle")?
            .encode("id", &self.id, encode_any)?
            .encode("meta", meta, encode_any)?
            .encode("identifier", &self.identifier, encode_identifier)?
            .encode("type", "document", encode_any)?
            .encode("timestamp", &self.timestamp, encode_any)?
            .field_name("entry")?
            .array()?
            .inline_opt(&self.entry.composition, encode_any)?
            .inline_opt(&self.entry.medication_request, encode_any)?
            .inline_opt(&self.entry.medication, encode_any)?
            .inline_opt(&self.entry.patient, encode_any)?
            .inline_opt(&self.entry.practitioner, encode_any)?
            .inline_opt(&self.entry.organization, encode_any)?
            .inline_opt(&self.entry.coverage, encode_any)?
            .inline_opt(&self.entry.practitioner_role, encode_any)?
            .end()?
            .encode_opt("signature", signature, encode_any)?
            .end()?;

        Ok(())
    }
}

impl<T> Encode for &(String, T)
where
    for<'a> &'a T: Encode,
{
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode("fullUrl", &self.0, encode_any)?
            .resource("resource", &self.1, encode_any)?
            .end()?;

        Ok(())
    }
}

const PROFILE: &str = "https://fhir.kbv.de/StructureDefinition/KBV_PR_ERP_Bundle|1.00.000";

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::convert::TryInto;
    use std::fs::read_to_string;
    use std::str::from_utf8;

    use chrono::DateTime;
    use resources::{misc::PrescriptionId, types::FlowType};

    use crate::fhir::{
        decode::{tests::load_stream, JsonDecode, XmlDecode},
        encode::{JsonEncode, XmlEncode},
    };

    use super::super::{
        super::tests::{trim_json_str, trim_xml_str},
        composition::tests::test_composition,
        coverage::tests::test_coverage,
        medication::tests::test_medication_pzn,
        medication_request::tests::test_medication_request,
        organization::tests::test_organization,
        patient::tests::test_patient,
        practitioner::tests::test_practitioner,
        practitioner_role::tests::test_practitioner_role,
    };

    #[tokio::test]
    async fn test_decode_json() {
        let mut stream = load_stream("./examples/kbv_bundle.json");

        let actual = stream.json::<KbvBundle>().await.unwrap();
        let expected = test_kbv_bundle();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_decode_xml() {
        let mut stream = load_stream("./examples/kbv_bundle.xml");

        let actual = stream.xml::<KbvBundle>().await.unwrap();
        let expected = test_kbv_bundle();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_encode_json() {
        let value = test_kbv_bundle();

        let actual = (&value).json().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/kbv_bundle.json").unwrap();

        assert_eq!(trim_json_str(&actual), trim_json_str(&expected));
    }

    #[tokio::test]
    async fn test_encode_xml() {
        let value = test_kbv_bundle();

        let actual = (&value).xml().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/kbv_bundle.xml").unwrap();

        assert_eq!(trim_xml_str(&actual), trim_xml_str(&expected));
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
            signature: vec![],
        }
    }
}
