/*
 * Copyright (c) 2021 gematik GmbH
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

use std::iter::once;

use async_trait::async_trait;
use miscellaneous::str::icase_eq;
use resources::practitioner::{Identifier, Practitioner, Qualification};

use crate::fhir::{
    decode::{decode_any, DataStream, Decode, DecodeError, DecodeStream, Fields},
    encode::{encode_any, DataStorage, Encode, EncodeError, EncodeStream},
};

use super::{
    meta::Meta,
    primitives::{
        decode_codeable_concept, decode_coding, decode_identifier, encode_codeable_concept,
        encode_coding, encode_identifier, CodeableConcept, Coding, Identifier as IdentifierTrait,
    },
};

/* Decode */

#[async_trait(?Send)]
impl Decode for Practitioner {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["id", "meta", "identifier", "name", "qualification"]);

        stream.root("Practitioner").await?;

        let id = stream.decode(&mut fields, decode_any).await?;
        let meta = stream.decode::<Meta, _>(&mut fields, decode_any).await?;
        let identifier = stream.decode_opt(&mut fields, decode_identifier).await?;
        let name = stream.decode(&mut fields, decode_any).await?;
        let qualification = stream.decode(&mut fields, decode_any).await?;

        stream.end().await?;

        if !meta.profiles.iter().any(|p| icase_eq(p, PROFILE)) {
            return Err(DecodeError::InvalidProfile {
                actual: meta.profiles,
                expected: vec![PROFILE.into()],
            });
        }

        Ok(Practitioner {
            id,
            identifier,
            name,
            qualification,
        })
    }
}

#[async_trait(?Send)]
impl Decode for Qualification {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut type_ = None;
        let mut job_title = None;

        let mut fields = Fields::new(&["qualification"]);
        while stream.begin_substream_vec(&mut fields).await? {
            stream.element().await?;

            let mut fields = Fields::new(&["code"]);
            stream.begin_substream(&mut fields).await?;
            stream.element().await?;

            let mut fields = Fields::new(&["coding", "text"]);
            type_ = type_.or(stream.decode_opt(&mut fields, decode_coding).await?);
            job_title = job_title.or(stream.decode_opt(&mut fields, decode_any).await?);

            stream.end().await?;
            stream.end_substream().await?;

            stream.end().await?;
            stream.end_substream().await?;
        }

        let type_ = type_.ok_or_else(|| DecodeError::MissingField {
            id: "Typ".into(),
            path: stream.path().into(),
        })?;
        let job_title = job_title.ok_or_else(|| DecodeError::MissingField {
            id: "Berufsbezeichnung".into(),
            path: stream.path().into(),
        })?;

        Ok(Qualification { type_, job_title })
    }
}

/* Encode */

impl Encode for &Practitioner {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let meta = Meta {
            profiles: vec![PROFILE.into()],
            ..Default::default()
        };

        stream
            .root("Practitioner")?
            .encode("id", &self.id, encode_any)?
            .encode("meta", meta, encode_any)?
            .encode_vec("identifier", &self.identifier, encode_identifier)?
            .encode_vec("name", once(&self.name), encode_any)?
            .encode("qualification", &self.qualification, encode_any)?
            .end()?;

        Ok(())
    }
}

impl Encode for &Qualification {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .array()?
            .element()?
            .field_name("code")?
            .element()?
            .encode_vec("coding", once(&self.type_), encode_coding)?
            .end()?
            .end()?
            .element()?
            .field_name("code")?
            .element()?
            .encode("text", &self.job_title, encode_any)?
            .end()?
            .end()?
            .end()?;

        Ok(())
    }
}

/* Misc */

#[async_trait(?Send)]
impl IdentifierTrait for Identifier {
    async fn decode_identifier<S>(
        stream: &mut DecodeStream<S>,
    ) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["type", "system", "value"]);

        stream.element().await?;

        let mut ret = stream.decode(&mut fields, decode_codeable_concept).await?;
        match &mut ret {
            Identifier::ANR(value) => {
                let _system = stream.ifixed(&mut fields, SYSTEM_ANR).await?;
                *value = stream.decode(&mut fields, decode_any).await?;
            }
            Identifier::ZANR(value) => {
                let _system = stream.ifixed(&mut fields, SYSTEM_ZANR).await?;
                *value = stream.decode(&mut fields, decode_any).await?;
            }
        }

        stream.end().await?;

        Ok(ret)
    }

    fn encode_identifier<S>(
        &self,
        stream: &mut EncodeStream<S>,
    ) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let (system, value) = match self {
            Identifier::ANR(value) => (SYSTEM_ANR, value),
            Identifier::ZANR(value) => (SYSTEM_ZANR, value),
        };

        stream
            .element()?
            .encode("type", self, encode_codeable_concept)?
            .encode("system", system, encode_any)?
            .encode("value", value, encode_any)?
            .end()?;

        Ok(())
    }
}

#[async_trait(?Send)]
impl CodeableConcept for Identifier {
    async fn decode_codeable_concept<S>(
        stream: &mut DecodeStream<S>,
    ) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["coding"]);

        stream.element().await?;

        let ret = stream.decode(&mut fields, decode_coding).await?;

        stream.end().await?;

        Ok(ret)
    }

    fn encode_codeable_concept<S>(
        &self,
        stream: &mut EncodeStream<S>,
    ) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode_vec("coding", once(self), encode_coding)?
            .end()?;

        Ok(())
    }
}

#[async_trait(?Send)]
impl Coding for Identifier {
    async fn decode_coding<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["system", "code"]);

        stream.element().await?;

        let system = stream.decode::<String, _>(&mut fields, decode_any).await?;
        let code = stream.decode::<String, _>(&mut fields, decode_any).await?;

        stream.end().await?;

        match (system.as_str(), code.as_str()) {
            (x, "LANR") if icase_eq(x, SYSTEM_V2_0203) => Ok(Identifier::ANR(Default::default())),
            (x, "ZANR") if icase_eq(x, SYSTEM_DE_BASIS) => Ok(Identifier::ZANR(Default::default())),
            (system, code) => Err(DecodeError::InvalidFixedValue {
                actual: format!("{} {}", system, code).into(),
                expected: format!(
                    "{} {} | {} {}",
                    SYSTEM_V2_0203, "LANR", SYSTEM_DE_BASIS, "ZANR"
                )
                .into(),
                path: stream.path().into(),
            }),
        }
    }

    fn encode_coding<S>(&self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let (system, code) = match self {
            Identifier::ANR(_) => (SYSTEM_V2_0203, "LANR"),
            Identifier::ZANR(_) => (SYSTEM_DE_BASIS, "ZANR"),
        };

        stream
            .element()?
            .encode("system", system, encode_any)?
            .encode("code", code, encode_any)?
            .end()?;

        Ok(())
    }
}

const PROFILE: &str = "https://fhir.kbv.de/StructureDefinition/KBV_PR_FOR_Practitioner|1.0.3";

const SYSTEM_ANR: &str = "https://fhir.kbv.de/NamingSystem/KBV_NS_Base_ANR";
const SYSTEM_ZANR: &str = "https://fhir.kbv.de/NamingSystem/KBV_NS_Base_BSNR";

const SYSTEM_V2_0203: &str = "http://terminology.hl7.org/CodeSystem/v2-0203";
const SYSTEM_DE_BASIS: &str = "http://fhir.de/CodeSystem/identifier-type-de-basis";

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::convert::TryInto;
    use std::fs::read_to_string;
    use std::str::from_utf8;

    use resources::misc::{Code, Family, Name, Prefix};

    use crate::fhir::{
        decode::{tests::load_stream, JsonDecode, XmlDecode},
        encode::{JsonEncode, XmlEncode},
    };

    use super::super::super::tests::{trim_json_str, trim_xml_str};

    #[tokio::test]
    async fn test_decode_json() {
        let mut stream = load_stream("./examples/practitioner.json");

        let actual = stream.json::<Practitioner>().await.unwrap();
        let expected = test_practitioner();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_decode_xml() {
        let mut stream = load_stream("./examples/practitioner.xml");

        let actual = stream.xml::<Practitioner>().await.unwrap();
        let expected = test_practitioner();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_encode_json() {
        let value = test_practitioner();

        let actual = (&value).json().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/practitioner.json").unwrap();

        assert_eq!(trim_json_str(&actual), trim_json_str(&expected));
    }

    #[tokio::test]
    async fn test_encode_xml() {
        let value = test_practitioner();

        let actual = (&value).xml().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/practitioner.xml").unwrap();

        assert_eq!(trim_xml_str(&actual), trim_xml_str(&expected));
    }

    pub fn test_practitioner() -> Practitioner {
        Practitioner {
            id: "20597e0e-cb2a-45b3-95f0-dc3dbdb617c3".try_into().unwrap(),
            identifier: Some(Identifier::ANR("838382202".into())),
            name: Name {
                given: "Hans".into(),
                prefix: Some(Prefix {
                    value: "Dr. med.".into(),
                    qualifier: true,
                }),
                family: Family {
                    value: "Topp-Glücklich".into(),
                    prefix: None,
                    family: Some("Topp-Glücklich".into()),
                    extension: None,
                },
            },
            qualification: Qualification {
                type_: Code {
                    system: "https://fhir.kbv.de/CodeSystem/KBV_CS_FOR_Qualification_Type".into(),
                    code: "00".into(),
                },
                job_title: "Hausarzt".into(),
            },
        }
    }
}
