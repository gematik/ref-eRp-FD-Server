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
use resources::practitioner_role::PractitionerRole;

use crate::fhir::{
    decode::{decode_any, DataStream, Decode, DecodeError, DecodeStream, Fields},
    encode::{encode_any, DataStorage, Encode, EncodeError, EncodeStream},
};

use super::{
    meta::Meta,
    primitives::{decode_reference, encode_reference},
};

/* Decode */

#[async_trait(?Send)]
impl Decode for PractitionerRole {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["id", "meta", "practitioner", "organization"]);

        stream.root("PractitionerRole").await?;

        let id = stream.decode(&mut fields, decode_any).await?;
        let meta = stream.decode::<Meta, _>(&mut fields, decode_any).await?;
        let practitioner = stream.decode(&mut fields, decode_reference).await?;

        stream.begin_substream(&mut fields).await?;
        stream.element().await?;

        let mut fields = Fields::new(&["identifier"]);

        stream.begin_substream(&mut fields).await?;
        stream.element().await?;

        let mut fields = Fields::new(&["system", "value"]);

        let _system = stream.fixed(&mut fields, SYSTEM_ORGANIZATION).await?;
        let organization = stream.decode(&mut fields, decode_any).await?;

        stream.end().await?;
        stream.end_substream().await?;

        stream.end().await?;
        stream.end_substream().await?;
        stream.end().await?;

        if !meta.profiles.iter().any(|p| p == PROFILE) {
            return Err(DecodeError::InvalidProfile {
                actual: meta.profiles,
                expected: vec![PROFILE.into()],
            });
        }

        Ok(PractitionerRole {
            id,
            practitioner,
            organization,
        })
    }
}

/* Encode */

impl Encode for &PractitionerRole {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let meta = Meta {
            profiles: vec![PROFILE.into()],
        };

        stream
            .root("PractitionerRole")?
            .encode("id", &self.id, encode_any)?
            .encode("meta", meta, encode_any)?
            .encode("practitioner", &self.practitioner, encode_reference)?
            .field_name("organization")?
            .element()?
            .field_name("identifier")?
            .element()?
            .encode("system", SYSTEM_ORGANIZATION, encode_any)?
            .encode("value", &self.organization, encode_any)?
            .end()?
            .end()?
            .end()?;

        Ok(())
    }
}

const PROFILE: &str = "https://fhir.kbv.de/StructureDefinition/KBV_PR_FOR_PractitionerRole|1.0.1";

const SYSTEM_ORGANIZATION: &str = "http://fhir.de/NamingSystem/asv/teamnummer";

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
        let mut stream = load_stream("./examples/practitioner_role.json");

        let actual = stream.json::<PractitionerRole>().await.unwrap();
        let expected = test_practitioner_role();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_decode_xml() {
        let mut stream = load_stream("./examples/practitioner_role.xml");

        let actual = stream.xml::<PractitionerRole>().await.unwrap();
        let expected = test_practitioner_role();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_encode_json() {
        let value = test_practitioner_role();

        let actual = (&value).json().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/practitioner_role.json").unwrap();

        assert_eq!(trim_json_str(&actual), trim_json_str(&expected));
    }

    #[tokio::test]
    async fn test_encode_xml() {
        let value = test_practitioner_role();

        let actual = (&value).xml().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/practitioner_role.xml").unwrap();

        assert_eq!(trim_xml_str(&actual), trim_xml_str(&expected));
    }

    pub fn test_practitioner_role() -> PractitionerRole {
        PractitionerRole {
            id: "9a4090f8-8c5a-11ea-bc55-0242ac13000".try_into().unwrap(),
            practitioner: "Practitioner/20597e0e-cb2a-45b3-95f0-dc3dbdb617c3".into(),
            organization: "003456789".into(),
        }
    }
}
