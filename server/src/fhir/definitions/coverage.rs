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
use futures::future::FutureExt;
use miscellaneous::str::icase_eq;
use resources::coverage::{Coverage, Extension, Payor};

use crate::fhir::{
    decode::{
        decode_any, DataStream, Decode, DecodeError, DecodeFuture, DecodeStream, Fields, Search,
    },
    encode::{encode_any, DataStorage, Encode, EncodeError, EncodeStream},
};

use super::{
    meta::Meta,
    primitives::{
        decode_codeable_concept, decode_coding, decode_reference, encode_codeable_concept,
        encode_coding, encode_reference,
    },
};

/* Decode */

#[async_trait(?Send)]
impl Decode for Coverage {
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
            "beneficiary",
            "periodEnd",
            "payor",
        ]);

        stream.root("Coverage").await?;

        let id = stream.decode(&mut fields, decode_any).await?;
        let meta = stream.decode::<Meta, _>(&mut fields, decode_any).await?;
        let extension = stream.decode(&mut fields, decode_any).await?;
        let _status = stream.fixed(&mut fields, "active").await?;
        let type_ = stream.decode(&mut fields, decode_codeable_concept).await?;
        let beneficiary = stream.decode(&mut fields, decode_reference).await?;
        let period_end = stream.decode_opt(&mut fields, decode_any).await?;
        let payor = stream.decode(&mut fields, decode_any).await?;

        stream.end().await?;

        if !meta.profiles.iter().any(|p| icase_eq(p, PROFILE)) {
            return Err(DecodeError::InvalidProfile {
                actual: meta.profiles,
                expected: vec![PROFILE.into()],
            });
        }

        Ok(Coverage {
            id,
            extension,
            type_,
            beneficiary,
            period_end,
            payor,
        })
    }
}

#[async_trait(?Send)]
impl Decode for Extension {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut special_group = None;
        let mut dmp_mark = None;
        let mut insured_type = None;
        let mut wop = None;

        let mut fields = Fields::new(&["extension"]);
        while stream.begin_substream_vec(&mut fields).await? {
            stream.element().await?;

            let url = stream.value(Search::Exact("url")).await?.unwrap();

            match url.as_str() {
                x if icase_eq(x, URL_SPECIAL_GROUP) => {
                    let mut fields = Fields::new(&["valueCoding"]);

                    special_group = Some(stream.decode(&mut fields, decode_coding).await?);
                }
                x if icase_eq(x, URL_DMP_MARK) => {
                    let mut fields = Fields::new(&["valueCoding"]);

                    dmp_mark = Some(stream.decode(&mut fields, decode_coding).await?);
                }
                x if icase_eq(x, URL_INSURED_TYPE) => {
                    let mut fields = Fields::new(&["valueCoding"]);

                    insured_type = Some(stream.decode(&mut fields, decode_coding).await?);
                }
                x if icase_eq(x, URL_WOP) => {
                    let mut fields = Fields::new(&["valueCoding"]);

                    wop = Some(stream.decode(&mut fields, decode_coding).await?);
                }
                _ => (),
            }

            stream.end().await?;
            stream.end_substream().await?;
        }

        Ok(Extension {
            special_group,
            dmp_mark,
            insured_type,
            wop,
        })
    }
}

#[async_trait(?Send)]
impl Decode for Payor {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["identifier", "display"]);

        stream.element().await?;

        let identifier = stream.decode_opt(&mut fields, decode_identifier).await?;
        let display = stream.decode(&mut fields, decode_any).await?;

        stream.end().await?;

        let (value, alternative_id) = match identifier {
            Some((value, alternative_id)) => (Some(value), alternative_id),
            None => (None, None),
        };

        Ok(Payor {
            display,
            value,
            alternative_id,
        })
    }
}

fn decode_identifier<'a, S>(
    stream: &'a mut DecodeStream<S>,
) -> DecodeFuture<'a, (String, Option<String>), S::Error>
where
    S: DataStream,
{
    async move {
        let mut extension = None;

        let mut fields = Fields::new(&["extension", "system", "value"]);

        stream.element().await?;

        while stream.begin_substream_vec(&mut fields).await? {
            stream.element().await?;

            let url = stream.value(Search::Exact("url")).await?.unwrap();

            if icase_eq(url, URL_ALTERNATIVE_ID) {
                let mut fields = Fields::new(&["valueIdentifier"]);

                let (ext, _) = stream
                    .decode::<(String, Option<String>), _>(&mut fields, decode_identifier)
                    .await?;

                extension = Some(ext);
            }

            stream.end().await?;
            stream.end_substream().await?;
        }

        let _system = stream.ifixed(&mut fields, SYSTEM_PAYOR).await?;
        let value = stream.decode(&mut fields, decode_any).await?;

        stream.end().await?;

        Ok((value, extension))
    }
    .boxed_local()
}

/* Encode */

impl Encode for &Coverage {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let meta = Meta {
            profiles: vec![PROFILE.into()],
        };

        stream
            .root("Coverage")?
            .encode("id", &self.id, encode_any)?
            .encode("meta", meta, encode_any)?
            .encode("extension", &self.extension, encode_any)?
            .encode("status", "active", encode_any)?
            .encode("type", &self.type_, encode_codeable_concept)?
            .encode("beneficiary", &self.beneficiary, encode_reference)?
            .encode_opt("periodEnd", &self.period_end, encode_any)?
            .encode_vec("payor", once(&self.payor), encode_any)?
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

        if let Some(special_group) = &self.special_group {
            stream
                .element()?
                .attrib("url", URL_SPECIAL_GROUP, encode_any)?
                .encode("valueCoding", special_group, encode_coding)?
                .end()?;
        }

        if let Some(dmp_mark) = &self.dmp_mark {
            stream
                .element()?
                .attrib("url", URL_DMP_MARK, encode_any)?
                .encode("valueCoding", dmp_mark, encode_coding)?
                .end()?;
        }

        if let Some(insured_type) = &self.insured_type {
            stream
                .element()?
                .attrib("url", URL_INSURED_TYPE, encode_any)?
                .encode("valueCoding", insured_type, encode_coding)?
                .end()?;
        }

        if let Some(wop) = &self.wop {
            stream
                .element()?
                .attrib("url", URL_WOP, encode_any)?
                .encode("valueCoding", wop, encode_coding)?
                .end()?;
        }

        stream.end()?;

        Ok(())
    }
}

impl Encode for &Payor {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let ident = self
            .value
            .as_ref()
            .map(|ident| (ident, self.alternative_id.as_ref()));

        stream
            .element()?
            .encode_opt("identifier", ident, encode_identifier)?
            .encode("display", &self.display, encode_any)?
            .end()?;

        Ok(())
    }
}

fn encode_identifier<S>(
    (value, ext): (&String, Option<&String>),
    stream: &mut EncodeStream<S>,
) -> Result<(), EncodeError<S::Error>>
where
    S: DataStorage,
{
    stream.element()?;

    if let Some(ext) = ext {
        stream
            .field_name("extension")?
            .array()?
            .element()?
            .attrib("url", URL_ALTERNATIVE_ID, encode_any)?
            .encode("valueIdentifier", (ext, None), encode_identifier)?
            .end()?
            .end()?;
    }

    stream
        .encode("system", SYSTEM_PAYOR, encode_any)?
        .encode("value", value, encode_any)?
        .end()?;

    Ok(())
}

const PROFILE: &str = "https://fhir.kbv.de/StructureDefinition/KBV_PR_FOR_Coverage|1.0.1";

const URL_SPECIAL_GROUP: &str = "http://fhir.de/StructureDefinition/gkv/besondere-personengruppe";
const URL_DMP_MARK: &str = "http://fhir.de/StructureDefinition/gkv/dmp-kennzeichen";
const URL_INSURED_TYPE: &str = "http://fhir.de/StructureDefinition/gkv/versichertenart";
const URL_WOP: &str = "http://fhir.de/StructureDefinition/gkv/wop";
const URL_ALTERNATIVE_ID: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_EX_FOR_Alternative_IK";

const SYSTEM_PAYOR: &str = "http://fhir.de/NamingSystem/arge-ik/iknr";

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::convert::TryInto;
    use std::fs::read_to_string;
    use std::str::from_utf8;

    use resources::misc::Code;

    use crate::fhir::{
        decode::{tests::load_stream, JsonDecode, XmlDecode},
        encode::{JsonEncode, XmlEncode},
    };

    use super::super::super::tests::{trim_json_str, trim_xml_str};

    #[tokio::test]
    async fn test_decode_json() {
        let mut stream = load_stream("./examples/coverage.json");

        let actual = stream.json::<Coverage>().await.unwrap();
        let expected = test_coverage();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_decode_xml() {
        let mut stream = load_stream("./examples/coverage.xml");

        let actual = stream.xml::<Coverage>().await.unwrap();
        let expected = test_coverage();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_encode_json() {
        let value = test_coverage();

        let actual = (&value).json().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/coverage.json").unwrap();

        assert_eq!(trim_json_str(&actual), trim_json_str(&expected));
    }

    #[tokio::test]
    async fn test_encode_xml() {
        let value = test_coverage();

        let actual = (&value).xml().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/coverage.xml").unwrap();

        assert_eq!(trim_xml_str(&actual), trim_xml_str(&expected));
    }

    pub fn test_coverage() -> Coverage {
        Coverage {
            id: "1b1ffb6e-eb05-43d7-87eb-e7818fe9661a".try_into().unwrap(),
            extension: Extension {
                special_group: Some(Code {
                    system: "https://fhir.kbv.de/CodeSystem/KBV_CS_SFHIR_KBV_PERSONENGRUPPE".into(),
                    code: "00".into(),
                }),
                dmp_mark: Some(Code {
                    system: "https://fhir.kbv.de/CodeSystem/KBV_CS_SFHIR_KBV_DMP".into(),
                    code: "00".into(),
                }),
                insured_type: Some(Code {
                    system: "https://fhir.kbv.de/CodeSystem/KBV_CS_SFHIR_KBV_VERSICHERTENSTATUS"
                        .into(),
                    code: "1".into(),
                }),
                wop: Some(Code {
                    system: "https://fhir.kbv.de/CodeSystem/KBV_CS_SFHIR_ITA_WOP".into(),
                    code: "03".into(),
                }),
            },
            type_: Code {
                system: "http://fhir.de/CodeSystem/versicherungsart-de-basis".into(),
                code: "GKV".into(),
            },
            beneficiary: "Patient/9774f67f-a238-4daf-b4e6-679deeef3811".into(),
            period_end: None,
            payor: Payor {
                display: "AOK Rheinland/Hamburg".into(),
                value: Some("104212059".into()),
                alternative_id: None,
            },
        }
    }
}
