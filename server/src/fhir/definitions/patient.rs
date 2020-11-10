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

use std::convert::TryInto;
use std::iter::once;

use async_trait::async_trait;
use resources::patient::{Identifier, Patient};

use crate::fhir::{
    decode::{decode_any, DataStream, Decode, DecodeError, DecodeStream, Fields},
    encode::{encode_any, DataStorage, Encode, EncodeError, EncodeStream},
};

use super::{
    meta::Meta,
    primitives::{
        decode_codeable_concept, decode_identifier, encode_codeable_concept, encode_identifier,
        CodeEx, CodeableConceptEx, CodingEx, Identifier as IdentifierTrait,
    },
};

/* Decode */

#[async_trait(?Send)]
impl Decode for Patient {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["id", "meta", "identifier", "name", "birthDate", "address"]);

        stream.root("Patient").await?;

        let id = stream.decode(&mut fields, decode_any).await?;
        let meta = stream.decode::<Meta, _>(&mut fields, decode_any).await?;
        let identifier = stream.decode_opt(&mut fields, decode_identifier).await?;
        let name = stream.decode(&mut fields, decode_any).await?;
        let birth_date = stream.decode(&mut fields, decode_any).await?;
        let address = stream.decode(&mut fields, decode_any).await?;

        stream.end().await?;

        if !meta.profiles.iter().any(|p| p == PROFILE) {
            return Err(DecodeError::InvalidProfile {
                actual: meta.profiles,
                expected: vec![PROFILE.into()],
            });
        }

        Ok(Patient {
            id,
            identifier,
            name,
            birth_date,
            address,
        })
    }
}

/* Encode */

impl Encode for &Patient {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let meta = Meta {
            profiles: vec![PROFILE.into()],
        };

        stream
            .root("Patient")?
            .encode("id", &self.id, encode_any)?
            .encode("meta", meta, encode_any)?
            .encode_vec("identifier", &self.identifier, encode_identifier)?
            .encode_vec("name", once(&self.name), encode_any)?
            .encode("birthDate", &self.birth_date, encode_any)?
            .encode_vec("address", once(&self.address), encode_any)?
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

        let mut identifier = stream.decode(&mut fields, decode_codeable_concept).await?;

        match &mut identifier {
            Identifier::GKV { value } => {
                let _system = stream.fixed(&mut fields, SYSTEM_KVID).await?;
                *value = stream.decode(&mut fields, decode_any).await?;
            }
            Identifier::PKV { system, value } => {
                *system = stream.decode_opt(&mut fields, decode_any).await?;
                *value = stream.decode(&mut fields, decode_any).await?;
            }
            Identifier::KVK { value } => {
                let _system = stream.fixed(&mut fields, SYSTEM_KVK).await?;
                *value = stream.decode(&mut fields, decode_any).await?;
            }
        }

        stream.end().await?;

        Ok(identifier)
    }

    fn encode_identifier<S>(
        &self,
        stream: &mut EncodeStream<S>,
    ) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode("type", self, encode_codeable_concept)?;

        match &self {
            Identifier::GKV { value } => {
                stream
                    .encode("system", SYSTEM_KVID, encode_any)?
                    .encode("value", value, encode_any)?;
            }
            Identifier::PKV { system, value } => {
                stream
                    .encode_opt("system", system, encode_any)?
                    .encode("value", value, encode_any)?;
            }
            Identifier::KVK { value } => {
                stream
                    .encode("system", SYSTEM_KVK, encode_any)?
                    .encode("value", value, encode_any)?;
            }
        }

        stream.end()?;

        Ok(())
    }
}

impl CodeableConceptEx for Identifier {
    type Coding = Self;

    fn from_parts(coding: Self::Coding, _text: Option<String>) -> Self {
        coding
    }

    fn coding(&self) -> &Self::Coding {
        &self
    }
}

impl CodingEx for Identifier {
    type Code = Self;

    fn from_parts(code: Self::Code) -> Self {
        code
    }

    fn code(&self) -> &Self::Code {
        &self
    }

    fn system() -> Option<&'static str> {
        Some(SYSTEM_IDENTIFIER_TYPE)
    }
}

impl CodeEx for Identifier {
    fn from_parts(value: String) -> Result<Self, String> {
        match value.as_str() {
            "GKV" => Ok(Identifier::GKV {
                value: "0000000000".to_owned().try_into().unwrap(),
            }),
            "PKV" => Ok(Identifier::PKV {
                value: Default::default(),
                system: None,
            }),
            "KVK" => Ok(Identifier::KVK {
                value: Default::default(),
            }),
            _ => Err(value),
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Identifier::GKV { .. } => "GKV",
            Identifier::PKV { .. } => "PKV",
            Identifier::KVK { .. } => "KVK",
        }
    }
}

const PROFILE: &str = "https://fhir.kbv.de/StructureDefinition/KBV_PR_FOR_Patient|1.0.1";

const SYSTEM_IDENTIFIER_TYPE: &str = "http://fhir.de/CodeSystem/identifier-type-de-basis";
const SYSTEM_KVID: &str = "http://fhir.de/NamingSystem/gkv/kvid-10";
const SYSTEM_KVK: &str = "http://fhir.de/NamingSystem/gkv/kvk-versichertennummer";

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::convert::TryInto;
    use std::fs::read_to_string;
    use std::str::from_utf8;

    use resources::misc::{Address, Family, Kvnr, Name};

    use crate::fhir::{
        decode::{tests::load_stream, JsonDecode, XmlDecode},
        encode::{JsonEncode, XmlEncode},
    };

    use super::super::super::tests::{trim_json_str, trim_xml_str};

    #[tokio::test]
    async fn test_decode_json() {
        let mut stream = load_stream("./examples/patient.json");

        let actual: Patient = stream.json().await.unwrap();
        let expected = test_patient();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_decode_xml() {
        let mut stream = load_stream("./examples/patient.xml");

        let actual: Patient = stream.xml().await.unwrap();
        let expected = test_patient();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_encode_json() {
        let value = test_patient();

        let actual = (&value).json().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/patient.json").unwrap();

        assert_eq!(trim_json_str(&actual), trim_json_str(&expected));
    }

    #[tokio::test]
    async fn test_encode_xml() {
        let value = test_patient();

        let actual = (&value).xml().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/patient.xml").unwrap();

        assert_eq!(trim_xml_str(&actual), trim_xml_str(&expected));
    }

    pub fn test_patient() -> Patient {
        Patient {
            id: "9774f67f-a238-4daf-b4e6-679deeef3811".try_into().unwrap(),
            identifier: Some(Identifier::GKV {
                value: Kvnr::new("X234567890").unwrap(),
            }),
            name: Name {
                prefix: None,
                given: "Ludger".into(),
                family: Family {
                    value: "Ludger Königsstein".into(),
                    prefix: None,
                    family: Some("Königsstein".into()),
                    extension: None,
                },
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
