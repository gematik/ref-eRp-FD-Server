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
use resources::organization::{Identifier, Organization, Telecom};

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
impl Decode for Organization {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["id", "meta", "identifier", "name", "telecom", "address"]);

        stream.root("Organization").await?;

        let id = stream.decode(&mut fields, decode_any).await?;
        let meta = stream.decode::<Meta, _>(&mut fields, decode_any).await?;
        let identifier = stream.decode_opt(&mut fields, decode_identifier).await?;
        let name = stream.decode_opt(&mut fields, decode_any).await?;
        let telecom = stream.decode(&mut fields, decode_any).await?;
        let address = stream.decode_opt(&mut fields, decode_any).await?;

        stream.end().await?;

        if !meta.profiles.iter().any(|p| icase_eq(p, PROFILE)) {
            return Err(DecodeError::InvalidProfile {
                actual: meta.profiles,
                expected: vec![PROFILE.into()],
            });
        }

        Ok(Organization {
            id,
            identifier,
            name,
            telecom,
            address,
        })
    }
}

#[async_trait(?Send)]
impl Decode for Telecom {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut phone = None;
        let mut fax = None;
        let mut mail = None;

        let mut fields = Fields::new(&["telecom"]);
        while stream.begin_substream_vec(&mut fields).await? {
            stream.element().await?;

            let mut fields = Fields::new(&["system", "value"]);
            let system = stream.decode::<String, _>(&mut fields, decode_any).await?;

            match system.as_str() {
                "phone" => phone = Some(stream.decode(&mut fields, decode_any).await?),
                "fax" => fax = Some(stream.decode(&mut fields, decode_any).await?),
                "email" => mail = Some(stream.decode(&mut fields, decode_any).await?),
                _ => (),
            }

            stream.end().await?;
            stream.end_substream().await?;
        }

        let phone = phone.ok_or_else(|| DecodeError::MissingField {
            id: "phone".into(),
            path: stream.path().into(),
        })?;

        Ok(Telecom { phone, fax, mail })
    }
}

/* Encode */

impl Encode for &Organization {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let meta = Meta {
            profiles: vec![PROFILE.into()],
        };

        stream
            .root("Organization")?
            .encode("id", &self.id, encode_any)?
            .encode("meta", meta, encode_any)?
            .encode_vec("identifier", &self.identifier, encode_identifier)?
            .encode_opt("name", &self.name, encode_any)?
            .encode("telecom", &self.telecom, encode_any)?
            .encode_vec("address", &self.address, encode_any)?
            .end()?;

        Ok(())
    }
}

impl Encode for &Telecom {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .array()?
            .element()?
            .encode("system", "phone", encode_any)?
            .encode("value", &self.phone, encode_any)?
            .end()?;

        if let Some(fax) = &self.fax {
            stream
                .element()?
                .encode("system", "fax", encode_any)?
                .encode("value", fax, encode_any)?
                .end()?;
        }

        if let Some(mail) = &self.mail {
            stream
                .element()?
                .encode("system", "email", encode_any)?
                .encode("value", mail, encode_any)?
                .end()?;
        }

        stream.end()?;

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
            Identifier::IK(value) => {
                let _system = stream.ifixed(&mut fields, SYSTEM_IK).await?;
                *value = stream.decode(&mut fields, decode_any).await?;
            }
            Identifier::BS(value) => {
                let _system = stream.ifixed(&mut fields, SYSTEM_BS).await?;
                *value = stream.decode(&mut fields, decode_any).await?;
            }
            Identifier::KZV(value) => {
                let _system = stream.ifixed(&mut fields, SYSTEM_KZV).await?;
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
            Identifier::IK(value) => (SYSTEM_IK, value),
            Identifier::BS(value) => (SYSTEM_BS, value),
            Identifier::KZV(value) => (SYSTEM_KZV, value),
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
            (x, "XX") if icase_eq(x, SYSTEM_V2_0203) => Ok(Identifier::IK(Default::default())),
            (x, "BSNR") if icase_eq(x, SYSTEM_V2_0203) => Ok(Identifier::BS(Default::default())),
            (x, "ZANR") if icase_eq(x, SYSTEM_DE_BASIS) => Ok(Identifier::KZV(Default::default())),
            (system, code) => Err(DecodeError::InvalidFixedValue {
                actual: format!("{} {}", system, code).into(),
                expected: format!(
                    "{} {} | {} {} | {} {}",
                    SYSTEM_V2_0203, "XX", SYSTEM_V2_0203, "BSNR", SYSTEM_DE_BASIS, "ZANR"
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
            Identifier::IK(_) => (SYSTEM_V2_0203, "XX"),
            Identifier::BS(_) => (SYSTEM_V2_0203, "BSNR"),
            Identifier::KZV(_) => (SYSTEM_DE_BASIS, "ZANR"),
        };

        stream
            .element()?
            .encode("system", system, encode_any)?
            .encode("code", code, encode_any)?
            .end()?;

        Ok(())
    }
}

const PROFILE: &str = "https://fhir.kbv.de/StructureDefinition/KBV_PR_FOR_Organization|1.0.1";

const SYSTEM_IK: &str = "http://fhir.de/NamingSystem/arge-ik/iknr";
const SYSTEM_BS: &str = "https://fhir.kbv.de/NamingSystem/KBV_NS_Base_BSNR";
const SYSTEM_KZV: &str = "http://fhir.de/NamingSystem/kzbv/zahnarztnummer";

const SYSTEM_V2_0203: &str = "http://terminology.hl7.org/CodeSystem/v2-0203";
const SYSTEM_DE_BASIS: &str = "http://fhir.de/CodeSystem/identifier-type-de-basis";

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::convert::TryInto;
    use std::fs::read_to_string;
    use std::str::from_utf8;

    use resources::misc::Address;

    use crate::fhir::{
        decode::{tests::load_stream, JsonDecode, XmlDecode},
        encode::{JsonEncode, XmlEncode},
    };

    use super::super::super::tests::{trim_json_str, trim_xml_str};

    #[tokio::test]
    async fn test_decode_json() {
        let mut stream = load_stream("./examples/organization.json");

        let actual = stream.json::<Organization>().await.unwrap();
        let expected = test_organization();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_decode_xml() {
        let mut stream = load_stream("./examples/organization.xml");

        let actual = stream.xml::<Organization>().await.unwrap();
        let expected = test_organization();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_encode_json() {
        let value = test_organization();

        let actual = (&value).json().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/organization.json").unwrap();

        assert_eq!(trim_json_str(&actual), trim_json_str(&expected));
    }

    #[tokio::test]
    async fn test_encode_xml() {
        let value = test_organization();

        let actual = (&value).xml().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/organization.xml").unwrap();

        assert_eq!(trim_xml_str(&actual), trim_xml_str(&expected));
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
