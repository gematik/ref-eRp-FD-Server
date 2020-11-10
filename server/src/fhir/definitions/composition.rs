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
use std::iter::once;

use async_trait::async_trait;
use resources::composition::{Author, Composition, Extension, LegalBasis, Section};

use crate::fhir::{
    decode::{decode_any, DataStream, Decode, DecodeError, DecodeStream, Fields},
    encode::{encode_any, DataStorage, Encode, EncodeError, EncodeStream},
};

use super::{
    meta::Meta,
    primitives::{
        decode_codeable_concept, decode_coding, decode_reference, encode_codeable_concept,
        encode_coding, encode_reference, CodeEx, CodeableConceptEx, Coding, CodingEx,
    },
};

/* Decode */

#[async_trait(?Send)]
impl Decode for Composition {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&[
            "id",
            "meta",
            "extension",
            "status",
            "type",
            "subject",
            "date",
            "author",
            "title",
            "attester",
            "custodian",
            "section",
        ]);

        stream.root("Composition").await?;

        let id = stream.decode(&mut fields, decode_any).await?;
        let meta = stream.decode::<Meta, _>(&mut fields, decode_any).await?;
        let extension = stream.decode(&mut fields, decode_any).await?;
        let _status = stream.fixed(&mut fields, "final").await?;
        let _type = stream
            .decode::<CompositionType, _>(&mut fields, decode_codeable_concept)
            .await?;
        let subject = stream.decode_opt(&mut fields, decode_reference).await?;
        let date = stream.decode(&mut fields, decode_any).await?;
        let author = stream.decode(&mut fields, decode_any).await?;
        let _title = stream
            .fixed(&mut fields, "elektronische Arzneimittelverordnung")
            .await?;
        let attester = if stream.begin_substream_opt(&mut fields).await? {
            let mut fields = Fields::new(&["mode", "party"]);

            stream.element().await?;

            let _mode = stream.fixed(&mut fields, "legal").await?;
            let attester = stream.decode(&mut fields, decode_reference).await?;

            stream.end().await?;
            stream.end_substream().await?;

            Some(attester)
        } else {
            None
        };
        let custodian = stream.decode(&mut fields, decode_reference).await?;
        let section = stream.decode(&mut fields, decode_any).await?;

        stream.end().await?;

        if !meta.profiles.iter().any(|p| p == PROFILE) {
            return Err(DecodeError::InvalidProfile {
                actual: meta.profiles,
                expected: vec![PROFILE.into()],
            });
        }

        Ok(Composition {
            id,
            extension,
            subject,
            date,
            author,
            attester,
            custodian,
            section,
        })
    }
}

#[async_trait(?Send)]
impl Decode for Extension {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut legal_basis = None;

        let mut fields = Fields::new(&["extension"]);
        while stream.begin_substream_vec(&mut fields).await? {
            stream.element().await?;

            let mut fields = Fields::new(&["url", "valueCoding"]);
            let url = stream.decode::<String, _>(&mut fields, decode_any).await?;

            if url == URL_LEGAL_BASIS {
                legal_basis = Some(stream.decode(&mut fields, decode_coding).await?);
            }

            stream.end().await?;
            stream.end_substream().await?;
        }

        Ok(Extension { legal_basis })
    }
}

#[async_trait(?Send)]
impl Decode for Author {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut doctor = None;
        let mut prf = None;

        let mut fields = Fields::new(&["author"]);
        while stream.begin_substream_vec(&mut fields).await? {
            let mut fields = Fields::new(&["reference", "type", "identifier"]);

            stream.element().await?;

            let reference = stream
                .decode_opt::<Option<_>, _>(&mut fields, decode_any)
                .await?;
            let type_ = stream.decode::<String, _>(&mut fields, decode_any).await?;

            match type_.as_str() {
                "Practitioner" => {
                    doctor = Some(reference.ok_or_else(|| DecodeError::MissingField {
                        id: "reference".into(),
                        path: stream.path().into(),
                    })?);
                }
                "Device" => {
                    stream.begin_substream(&mut fields).await?;
                    stream.element().await?;

                    let mut fields = Fields::new(&["system", "value"]);

                    let _system = stream.fixed(&mut fields, SYSTEM_PRF).await?;
                    prf = Some(stream.decode(&mut fields, decode_any).await?);

                    stream.end().await?;
                    stream.end_substream().await?;
                }
                type_ => {
                    return Err(DecodeError::InvalidFixedValue {
                        actual: type_.into(),
                        expected: "Practitioner | Device".into(),
                        path: stream.path().into(),
                    })
                }
            }

            stream.end().await?;
            stream.end_substream().await?;
        }

        let doctor = doctor.ok_or_else(|| DecodeError::MissingField {
            id: "Practitioner".into(),
            path: stream.path().into(),
        })?;

        Ok(Author { doctor, prf })
    }
}

#[async_trait(?Send)]
impl Decode for Section {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut prescription = None;
        let mut practice_supply = None;
        let mut coverage = None;
        let mut practitioner_role = None;

        let mut fields = Fields::new(&["section"]);
        while stream.begin_substream_vec(&mut fields).await? {
            match decode_any::<SectionItem<'static>, _>(stream).await? {
                SectionItem::Prescription(v) => prescription = Some(v.into_owned()),
                SectionItem::PracticeSupply(v) => practice_supply = Some(v.into_owned()),
                SectionItem::Coverage(v) => coverage = Some(v.into_owned()),
                SectionItem::PractitionerRole(v) => practitioner_role = Some(v.into_owned()),
            }

            stream.end_substream().await?;
        }

        Ok(Section {
            prescription,
            practice_supply,
            coverage,
            practitioner_role,
        })
    }
}

#[async_trait(?Send)]
impl Decode for SectionItem<'static> {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["code", "entry"]);

        stream.element().await?;

        let item = stream
            .decode::<SectionItem, _>(&mut fields, decode_codeable_concept)
            .await?;
        let value = stream.decode(&mut fields, decode_reference).await?;

        stream.end().await?;

        Ok(item.update(value))
    }
}

/* Encode */

impl Encode for &Composition {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let meta = Meta {
            profiles: vec![PROFILE.into()],
        };

        stream
            .root("Composition")?
            .encode("id", &self.id, encode_any)?
            .encode("meta", meta, encode_any)?
            .encode("extension", &self.extension, encode_any)?
            .encode("status", "final", encode_any)?
            .encode("type", &CompositionType, encode_codeable_concept)?
            .encode_opt("subject", &self.subject, encode_reference)?
            .encode("date", &self.date, encode_any)?
            .encode("author", &self.author, encode_any)?
            .encode("title", "elektronische Arzneimittelverordnung", encode_any)?;

        if let Some(attester) = &self.attester {
            stream
                .field_name("attester")?
                .element()?
                .encode("mode", "legal", encode_any)?
                .encode("party", attester, encode_reference)?
                .end()?;
        }

        stream
            .encode("custodian", &self.custodian, encode_reference)?
            .encode("section", &self.section, encode_any)?
            .end()?;

        Ok(())
    }
}

impl Encode for &Extension {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream.array()?;

        if let Some(legal_basis) = &self.legal_basis {
            stream
                .element()?
                .attrib("url", URL_LEGAL_BASIS, encode_any)?
                .encode("valueCoding", legal_basis, encode_coding)?
                .end()?;
        }

        stream.end()?;

        Ok(())
    }
}

impl Encode for &Author {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .array()?
            .element()?
            .encode("reference", &self.doctor, encode_any)?
            .encode("type", "Practitioner", encode_any)?
            .end()?;

        if let Some(prf) = &self.prf {
            stream
                .element()?
                .encode("type", "Device", encode_any)?
                .field_name("identifier")?
                .element()?
                .encode("system", SYSTEM_PRF, encode_any)?
                .encode("value", prf, encode_any)?
                .end()?
                .end()?;
        }

        stream.end()?;

        Ok(())
    }
}

impl Encode for &Section {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream.array()?;

        if let Some(prescription) = &self.prescription {
            encode_any(
                SectionItem::Prescription(Cow::Borrowed(prescription)),
                stream,
            )?;
        }

        if let Some(practice_supply) = &self.practice_supply {
            encode_any(
                SectionItem::PracticeSupply(Cow::Borrowed(practice_supply)),
                stream,
            )?;
        }

        if let Some(coverage) = &self.coverage {
            encode_any(SectionItem::Coverage(Cow::Borrowed(coverage)), stream)?;
        }

        if let Some(practitioner_role) = &self.practitioner_role {
            encode_any(
                SectionItem::PractitionerRole(Cow::Borrowed(practitioner_role)),
                stream,
            )?;
        }

        stream.end()?;

        Ok(())
    }
}

impl Encode for SectionItem<'_> {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode("code", &self, encode_codeable_concept)?
            .encode_vec("entry", once(&self.into_owned()), encode_reference)?
            .end()?;

        Ok(())
    }
}

/* Misc */

impl CodingEx for LegalBasis {
    type Code = Self;

    fn from_parts(code: Self::Code) -> Self {
        code
    }

    fn code(&self) -> &Self::Code {
        &self
    }

    fn system() -> Option<&'static str> {
        Some(SYSTEM_LEGAL_BASIS)
    }
}

impl CodeEx for LegalBasis {
    fn from_parts(value: String) -> Result<Self, String> {
        match value.as_str() {
            "00" => Ok(Self::None),
            "01" => Ok(Self::Asv),
            "04" => Ok(Self::DischargeManagement),
            "07" => Ok(Self::Tss),
            "10" => Ok(Self::SubstituteRegulation),
            "11" => Ok(Self::SubstituteRegulationWithAsv),
            "14" => Ok(Self::SubstituteRegulationWithDischargeManagement),
            "17" => Ok(Self::SubstituteRegulationWithTss),
            _ => Err(value),
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::None => "00",
            Self::Asv => "01",
            Self::DischargeManagement => "04",
            Self::Tss => "07",
            Self::SubstituteRegulation => "10",
            Self::SubstituteRegulationWithAsv => "11",
            Self::SubstituteRegulationWithDischargeManagement => "14",
            Self::SubstituteRegulationWithTss => "17",
        }
    }
}

struct CompositionType;

impl CodeableConceptEx for CompositionType {
    type Coding = Self;

    fn from_parts(coding: Self::Coding, _text: Option<String>) -> Self {
        coding
    }

    fn coding(&self) -> &Self::Coding {
        &self
    }
}

#[async_trait(?Send)]
impl Coding for CompositionType {
    async fn decode_coding<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["system", "code"]);

        stream.element().await?;

        let _system = stream.fixed(&mut fields, SYSTEM_TYPE).await?;
        let _code = stream.fixed(&mut fields, "e16A").await?;

        stream.end().await?;

        Ok(CompositionType)
    }

    fn encode_coding<S>(&self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode("system", SYSTEM_TYPE, encode_any)?
            .encode("code", "e16A", encode_any)?
            .end()?;

        Ok(())
    }
}

enum SectionItem<'a> {
    Prescription(Cow<'a, str>),
    PracticeSupply(Cow<'a, str>),
    Coverage(Cow<'a, str>),
    PractitionerRole(Cow<'a, str>),
}

impl SectionItem<'_> {
    fn into_owned(self) -> String {
        match self {
            Self::Prescription(value) => value.into_owned(),
            Self::PracticeSupply(value) => value.into_owned(),
            Self::Coverage(value) => value.into_owned(),
            Self::PractitionerRole(value) => value.into_owned(),
        }
    }

    fn update(mut self, value: String) -> Self {
        let v = match &mut self {
            Self::Prescription(v) => v.to_mut(),
            Self::PracticeSupply(v) => v.to_mut(),
            Self::Coverage(v) => v.to_mut(),
            Self::PractitionerRole(v) => v.to_mut(),
        };

        *v = value;

        self
    }
}

impl CodeableConceptEx for SectionItem<'_> {
    type Coding = Self;

    fn from_parts(coding: Self::Coding, _text: Option<String>) -> Self {
        coding
    }

    fn coding(&self) -> &Self::Coding {
        &self
    }
}

impl CodingEx for SectionItem<'_> {
    type Code = Self;

    fn from_parts(code: Self::Code) -> Self {
        code
    }

    fn code(&self) -> &Self::Code {
        &self
    }

    fn system() -> Option<&'static str> {
        Some(SYSTEM_SECTION)
    }
}

impl CodeEx for SectionItem<'_> {
    fn from_parts(value: String) -> Result<Self, String> {
        match value.as_str() {
            "Prescription" => Ok(SectionItem::Prescription(Cow::Owned(Default::default()))),
            "PracticeSupply" => Ok(SectionItem::PracticeSupply(Cow::Owned(Default::default()))),
            "Coverage" => Ok(SectionItem::Coverage(Cow::Owned(Default::default()))),
            "FOR_PractitionerRole" => Ok(SectionItem::PractitionerRole(Cow::Owned(
                Default::default(),
            ))),
            _ => Err(value),
        }
    }

    fn code(&self) -> &'static str {
        match self {
            SectionItem::Prescription(_) => "Prescription",
            SectionItem::PracticeSupply(_) => "PracticeSupply",
            SectionItem::Coverage(_) => "Coverage",
            SectionItem::PractitionerRole(_) => "FOR_PractitionerRole",
        }
    }
}

const PROFILE: &str = "https://gematik.de/fhir/StructureDefinition/erxComposition";

const URL_LEGAL_BASIS: &str = "https://fhir.kbv.de/StructureDefinition/KBV_EX_FOR_Legal_basis";

const SYSTEM_LEGAL_BASIS: &str =
    "https://fhir.kbv.de/CodeSystem/KBV_CS_SFHIR_KBV_STATUSKENNZEICHEN";
const SYSTEM_TYPE: &str = "https://fhir.kbv.de/CodeSystem/KBV_CS_FOR_Formular_Art";
const SYSTEM_PRF: &str = "https://fhir.kbv.de/NamingSystem/KBV_NS_FOR_Pruefnummer";
const SYSTEM_SECTION: &str = "https://fhir.kbv.de/CodeSystem/KBV_CS_FOR_Section_Type";

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::convert::TryInto;
    use std::fs::read_to_string;
    use std::str::from_utf8;

    use crate::fhir::{
        decode::{tests::load_stream, JsonDecode, XmlDecode},
        encode::{JsonEncode, XmlEncode},
    };

    use super::super::super::tests::{trim_json_str, trim_xml_str};

    #[tokio::test]
    async fn test_decode_json() {
        let mut stream = load_stream("./examples/composition.json");

        let actual = stream.json::<Composition>().await.unwrap();
        let expected = test_composition();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_decode_xml() {
        let mut stream = load_stream("./examples/composition.xml");

        let actual = stream.xml::<Composition>().await.unwrap();
        let expected = test_composition();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_encode_json() {
        let value = test_composition();

        let actual = (&value).json().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/composition.json").unwrap();

        assert_eq!(trim_json_str(&actual), trim_json_str(&expected));
    }

    #[tokio::test]
    async fn test_encode_xml() {
        let value = test_composition();

        let actual = (&value).xml().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/composition.xml").unwrap();

        assert_eq!(trim_xml_str(&actual), trim_xml_str(&expected));
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
            attester: None,
            custodian: "Organization/cf042e44-086a-4d51-9c77-172f9a972e3b".into(),
            section: Section {
                prescription: Some("MedicationRequest/e930cdee-9eb5-4b44-88b5-2a18b69f3b9a".into()),
                practice_supply: None,
                coverage: Some("Coverage/1b1ffb6e-eb05-43d7-87eb-e7818fe9661a".into()),
                practitioner_role: Some(
                    "PractitionerRole/9a4090f8-8c5a-11ea-bc55-0242ac13000".into(),
                ),
            },
        }
    }
}
