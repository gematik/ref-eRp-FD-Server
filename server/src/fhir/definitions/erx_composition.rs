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
use resources::{types::DocumentType, ErxComposition};

use crate::fhir::{
    decode::{decode_any, DataStream, Decode, DecodeError, DecodeStream, Fields, Search},
    encode::{encode_any, DataStorage, Encode, EncodeError, EncodeStream},
};

use super::{
    meta::Meta,
    primitives::{
        decode_codeable_concept, decode_identifier, decode_reference, encode_codeable_concept,
        encode_identifier, encode_reference,
    },
};

/* Decode */

#[async_trait(?Send)]
impl Decode for ErxComposition {
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
            "date",
            "author",
            "title",
            "event",
        ]);

        stream.root("Composition").await?;

        let id = stream.decode(&mut fields, decode_any).await?;
        let meta = stream.decode::<Meta, _>(&mut fields, decode_any).await?;
        let beneficiary = {
            let mut beneficiary = None;

            while stream.begin_substream_vec(&mut fields).await? {
                stream.element().await?;

                let url = stream.value(Search::Exact("url")).await?.unwrap();
                if icase_eq(url, URL_BENEFICIARY) {
                    let mut fields = Fields::new(&["valueIdentifier"]);

                    beneficiary = Some(stream.decode(&mut fields, decode_identifier).await?);
                }

                stream.end().await?;
                stream.end_substream().await?;
            }

            beneficiary.ok_or_else(|| DecodeError::MissingExtension {
                url: URL_BENEFICIARY.into(),
                path: stream.path().into(),
            })?
        };
        let _status = stream.fixed(&mut fields, "final").await?;
        let type_ = stream
            .decode::<DocumentType, _>(&mut fields, decode_codeable_concept)
            .await?;
        let date = stream.decode(&mut fields, decode_any).await?;
        let author = stream.decode(&mut fields, decode_reference).await?;
        let _title = stream.fixed(&mut fields, "Quittung").await?;
        let (event_start, event_end) = {
            stream.begin_substream(&mut fields).await?;
            stream.element().await?;

            let mut fields = Fields::new(&["period"]);
            stream.begin_substream(&mut fields).await?;
            stream.element().await?;

            let mut fields = Fields::new(&["start", "end"]);
            let event_start = stream.decode(&mut fields, decode_any).await?;
            let event_end = stream.decode(&mut fields, decode_any).await?;

            stream.end().await?;
            stream.end_substream().await?;

            stream.end().await?;
            stream.end_substream().await?;

            (event_start, event_end)
        };

        stream.end().await?;

        if type_ != DocumentType::Receipt {
            return Err(DecodeError::InvalidFixedValue {
                actual: type_.to_string().into(),
                expected: DocumentType::Receipt.to_string().into(),
                path: stream.path().into(),
            });
        }

        if !meta.profiles.iter().any(|p| icase_eq(p, PROFILE)) {
            return Err(DecodeError::InvalidProfile {
                actual: meta.profiles,
                expected: vec![PROFILE.into()],
            });
        }

        Ok(ErxComposition {
            id,
            beneficiary,
            date,
            author,
            event_start,
            event_end,
        })
    }
}

/* Encode */

impl Encode for &ErxComposition {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let meta = Meta {
            profiles: vec![PROFILE.into()],
            ..Default::default()
        };

        stream
            .root("Composition")?
            .encode("id", &self.id, encode_any)?
            .encode("meta", meta, encode_any)?
            .field_name("extension")?
            .array()?
            .element()?
            .attrib("url", URL_BENEFICIARY, encode_any)?
            .encode("valueIdentifier", &self.beneficiary, encode_identifier)?
            .end()?
            .end()?
            .encode("status", "final", encode_any)?
            .encode("type", &DocumentType::Receipt, encode_codeable_concept)?
            .encode("date", &self.date, encode_any)?
            .encode_vec("author", once(&self.author), encode_reference)?
            .encode("title", "Quittung", encode_any)?
            .field_name("event")?
            .element()?
            .field_name("period")?
            .element()?
            .encode("start", &self.event_start, encode_any)?
            .encode("end", &self.event_end, encode_any)?
            .end()?
            .end()?
            .end()?;

        Ok(())
    }
}

const PROFILE: &str = "https://gematik.de/fhir/StructureDefinition/ErxComposition";

const URL_BENEFICIARY: &str = "https://gematik.de/fhir/StructureDefinition/BeneficiaryExtension";

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::convert::TryInto;
    use std::fs::read_to_string;
    use std::str::from_utf8;

    use resources::misc::TelematikId;

    use crate::fhir::{
        decode::{tests::load_stream, JsonDecode, XmlDecode},
        encode::{JsonEncode, XmlEncode},
    };

    use super::super::super::tests::{trim_json_str, trim_xml_str};

    #[tokio::test]
    async fn test_decode_json() {
        let mut stream = load_stream("./examples/erx_composition.json");

        let actual: ErxComposition = stream.json().await.unwrap();
        let expected = test_erx_composition();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_decode_xml() {
        let mut stream = load_stream("./examples/erx_composition.xml");

        let actual: ErxComposition = stream.xml().await.unwrap();
        let expected = test_erx_composition();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_encode_json() {
        let value = test_erx_composition();

        let actual = (&value).json().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/erx_composition.json").unwrap();

        assert_eq!(trim_json_str(&actual), trim_json_str(&expected));
    }

    #[tokio::test]
    async fn test_encode_xml() {
        let value = test_erx_composition();

        let actual = (&value).xml().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/erx_composition.xml").unwrap();

        assert_eq!(trim_xml_str(&actual), trim_xml_str(&expected));
    }

    pub fn test_erx_composition() -> ErxComposition {
        ErxComposition {
            id: "0123456789".try_into().unwrap(),
            beneficiary: TelematikId::new("606358757"),
            date: "2020-03-20T07:31:34.328+00:00".try_into().unwrap(),
            author: "https://prescriptionserver.telematik/Device/ErxService".into(),
            event_start: "2020-03-20T07:23:34.328+00:00".try_into().unwrap(),
            event_end: "2020-03-20T07:31:34.328+00:00".try_into().unwrap(),
        }
    }
}
