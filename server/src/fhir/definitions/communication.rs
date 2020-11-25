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

use std::iter::once;

use async_trait::async_trait;
use miscellaneous::str::icase_eq;
use resources::{
    communication::{
        Availability, Communication, DispenseReqExtensions, InfoReqExtensions, Inner, Payload,
        ReplyExtensions, RepresentativeExtensions, SupplyOptions,
    },
    misc::{Kvnr, TelematikId},
    primitives::Id,
    Medication,
};

use crate::fhir::{
    decode::{decode_any, DataStream, Decode, DecodeError, DecodeStream, Fields, Search},
    encode::{encode_any, DataStorage, Encode, EncodeError, EncodeStream},
};

use super::{
    bundle::{DecodeBundleResource, EncodeBundleResource},
    meta::Meta,
    primitives::{
        decode_coding, decode_identifier, decode_reference, encode_coding, encode_identifier,
        encode_reference, CodeEx, CodingEx, Identifier,
    },
};

/* Decode */

impl DecodeBundleResource for Communication {}

#[async_trait(?Send)]
impl Decode for Communication {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["id", "meta"]);

        stream.root("Communication").await?;

        let id = stream.decode_opt(&mut fields, decode_any).await?;
        let meta = stream.decode::<Meta, _>(&mut fields, decode_any).await?;

        let p = meta.profiles;
        let communication = if p.iter().any(|p| icase_eq(p, PROFILE_INFO_REQ)) {
            let mut inner = Inner::<InfoReqExtensions, TelematikId, Kvnr>::decode(stream).await?;
            inner.id = id;

            Communication::InfoReq(inner)
        } else if p.iter().any(|p| icase_eq(p, PROFILE_REPLY)) {
            let mut inner = Inner::<ReplyExtensions, Kvnr, TelematikId>::decode(stream).await?;
            inner.id = id;

            Communication::Reply(inner)
        } else if p.iter().any(|p| icase_eq(p, PROFILE_DISPENSE_REQ)) {
            let mut inner =
                Inner::<DispenseReqExtensions, TelematikId, Kvnr>::decode(stream).await?;
            inner.id = id;

            Communication::DispenseReq(inner)
        } else if p.iter().any(|p| icase_eq(p, PROFILE_REPRESENTATIVE)) {
            let mut inner = Inner::<RepresentativeExtensions, Kvnr, Kvnr>::decode(stream).await?;
            inner.id = id;

            Communication::Representative(inner)
        } else {
            return Err(DecodeError::InvalidProfile {
                actual: p,
                expected: vec![
                    PROFILE_INFO_REQ.into(),
                    PROFILE_REPLY.into(),
                    PROFILE_DISPENSE_REQ.into(),
                    PROFILE_REPRESENTATIVE.into(),
                ],
            });
        };

        stream.end().await?;

        Ok(communication)
    }
}

#[async_trait(?Send)]
impl<Ex, Recipient, Sender> Decode for Inner<Ex, Recipient, Sender>
where
    Ex: Clone + PartialEq + Decode,
    Recipient: Clone + PartialEq + Identifier,
    Sender: Clone + PartialEq + Identifier,
{
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&[
            "basedOn",
            "contained",
            "status",
            "about",
            "sent",
            "received",
            "recipient",
            "sender",
            "payload",
        ]);

        let based_on = stream.decode_opt(&mut fields, decode_reference).await?;
        let mut about = stream
            .resource_vec::<Vec<Medication>, _>(&mut fields, decode_any)
            .await?;
        let _status = stream.fixed(&mut fields, "unknown").await?;
        let about_ids = stream
            .decode_vec::<Vec<Id>, _>(&mut fields, decode_reference)
            .await?;
        let sent = stream.decode_opt(&mut fields, decode_any).await?;
        let received = stream.decode_opt(&mut fields, decode_any).await?;
        let recipient = stream
            .decode(&mut fields, decode_reference_identifier)
            .await?;
        let sender = stream
            .decode_opt(&mut fields, decode_reference_identifier)
            .await?;
        let payload = stream.decode(&mut fields, decode_any).await?;

        about.retain(|r| about_ids.contains(&r.id));

        for id in about_ids {
            if !about.iter().any(|r| r.id == id) {
                return Err(DecodeError::Custom {
                    message: format!(
                        "Unable to find referenced resource `about` with id `{}`",
                        id
                    ),
                    path: stream.path().into(),
                });
            }
        }

        Ok(Inner {
            id: None,
            based_on,
            about,
            sent,
            received,
            recipient,
            sender,
            payload,
        })
    }
}

#[async_trait(?Send)]
impl<Ex> Decode for Payload<Ex>
where
    Ex: Clone + PartialEq + Decode,
{
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["extension", "contentString"]);

        stream.element().await?;

        let extensions = stream.decode_opt(&mut fields, decode_any).await?;
        let content = stream.decode(&mut fields, decode_any).await?;

        stream.end().await?;

        Ok(Payload {
            extensions,
            content,
        })
    }
}

#[async_trait(?Send)]
impl Decode for InfoReqExtensions {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut insurance_provider = None;
        let mut substitution_allowed = None;
        let mut prescription_type = None;
        let mut preferred_supply_options = None;

        let mut fields = Fields::new(&["extension"]);
        while stream.begin_substream_vec(&mut fields).await? {
            stream.element().await?;

            let url = stream.value(Search::Exact("url")).await?.unwrap();
            match url.as_str() {
                x if icase_eq(x, URL_INSURANCE_PROVIDER) => {
                    let mut fields = Fields::new(&["valueIdentifier"]);

                    insurance_provider = Some(stream.decode(&mut fields, decode_identifier).await?);
                }
                x if icase_eq(x, URL_SUBSTITUTION_ALLOWED) => {
                    let mut fields = Fields::new(&["valueBoolean"]);

                    substitution_allowed = Some(stream.decode(&mut fields, decode_any).await?);
                }
                x if icase_eq(x, URL_PRESCRIPTION_TYPE) => {
                    let mut fields = Fields::new(&["valueCoding"]);

                    prescription_type = Some(stream.decode(&mut fields, decode_coding).await?);
                }
                x if icase_eq(x, URL_SUPPLY_OPTIONS) => {
                    preferred_supply_options = Some(decode_supply_options(stream).await?);
                }
                _ => (),
            }

            stream.end().await?;
            stream.end_substream().await?;
        }

        let insurance_provider =
            insurance_provider.ok_or_else(|| DecodeError::MissingExtension {
                url: URL_INSURANCE_PROVIDER.into(),
                path: stream.path().into(),
            })?;
        let substitution_allowed =
            substitution_allowed.ok_or_else(|| DecodeError::MissingExtension {
                url: URL_SUBSTITUTION_ALLOWED.into(),
                path: stream.path().into(),
            })?;
        let prescription_type = prescription_type.ok_or_else(|| DecodeError::MissingExtension {
            url: URL_PRESCRIPTION_TYPE.into(),
            path: stream.path().into(),
        })?;

        Ok(InfoReqExtensions {
            insurance_provider,
            substitution_allowed,
            prescription_type,
            preferred_supply_options,
        })
    }
}

#[async_trait(?Send)]
impl Decode for ReplyExtensions {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut availability = None;
        let mut offered_supply_options = None;

        let mut fields = Fields::new(&["extension"]);
        while stream.begin_substream_vec(&mut fields).await? {
            stream.element().await?;

            let url = stream.value(Search::Exact("url")).await?.unwrap();
            match url.as_str() {
                x if icase_eq(x, URL_AVAILABILITY) => {
                    let mut fields = Fields::new(&["valueCoding"]);

                    availability = Some(stream.decode(&mut fields, decode_coding).await?);
                }
                x if icase_eq(x, URL_SUPPLY_OPTIONS) => {
                    offered_supply_options = Some(decode_supply_options(stream).await?);
                }
                _ => (),
            }

            stream.end().await?;
            stream.end_substream().await?;
        }

        Ok(ReplyExtensions {
            availability,
            offered_supply_options,
        })
    }
}

#[async_trait(?Send)]
impl Decode for DispenseReqExtensions {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut insurance_provider = None;

        let mut fields = Fields::new(&["extension"]);
        while stream.begin_substream_vec(&mut fields).await? {
            stream.element().await?;

            let url = stream.value(Search::Exact("url")).await?.unwrap();
            if icase_eq(&url, URL_INSURANCE_PROVIDER) {
                let mut fields = Fields::new(&["valueIdentifier	"]);

                insurance_provider = Some(stream.decode(&mut fields, decode_identifier).await?);
            }

            stream.end().await?;
            stream.end_substream().await?;
        }

        Ok(DispenseReqExtensions { insurance_provider })
    }
}

#[async_trait(?Send)]
impl Decode for RepresentativeExtensions {
    async fn decode<S>(_stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        Ok(RepresentativeExtensions)
    }
}

async fn decode_reference_identifier<T, S>(
    stream: &mut DecodeStream<S>,
) -> Result<T, DecodeError<S::Error>>
where
    T: Identifier,
    S: DataStream,
{
    stream.element().await?;

    let mut fields = Fields::new(&["identifier"]);
    let ret = stream.decode(&mut fields, decode_identifier).await?;

    stream.end().await?;

    Ok(ret)
}

async fn decode_supply_options<S>(
    stream: &mut DecodeStream<S>,
) -> Result<SupplyOptions, DecodeError<S::Error>>
where
    S: DataStream,
{
    let mut on_premise = false;
    let mut delivery = false;
    let mut shipment = false;

    let mut fields = Fields::new(&["extension"]);
    while stream.begin_substream_vec(&mut fields).await? {
        stream.element().await?;

        let mut fields = Fields::new(&["url", "valueBoolean"]);
        let url = stream.decode::<String, _>(&mut fields, decode_any).await?;

        match url.as_str() {
            "onPremise" => on_premise = stream.decode(&mut fields, decode_any).await?,
            "delivery" => delivery = stream.decode(&mut fields, decode_any).await?,
            "shipment" => shipment = stream.decode(&mut fields, decode_any).await?,
            _ => (),
        }

        stream.end().await?;
        stream.end_substream().await?;
    }

    Ok(SupplyOptions {
        on_premise,
        delivery,
        shipment,
    })
}

/* Encode */

impl EncodeBundleResource for &Communication {}

impl Encode for &Communication {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let meta = Meta {
            profiles: vec![match self {
                Communication::InfoReq(_) => PROFILE_INFO_REQ.into(),
                Communication::Reply(_) => PROFILE_REPLY.into(),
                Communication::DispenseReq(_) => PROFILE_DISPENSE_REQ.into(),
                Communication::Representative(_) => PROFILE_REPRESENTATIVE.into(),
            }],
        };

        stream
            .root("Communication")?
            .encode_opt("id", self.id(), encode_any)?
            .encode("meta", meta, encode_any)?;

        match self {
            Communication::InfoReq(inner) => stream.inline(inner, encode_any)?,
            Communication::Reply(inner) => stream.inline(inner, encode_any)?,
            Communication::DispenseReq(inner) => stream.inline(inner, encode_any)?,
            Communication::Representative(inner) => stream.inline(inner, encode_any)?,
        };

        stream.end()?;

        Ok(())
    }
}

impl<'a, Extension, Recipient, Sender> Encode for &'a Inner<Extension, Recipient, Sender>
where
    &'a Extension: Encode,
    Extension: Clone + PartialEq,
    Recipient: Clone + PartialEq + Identifier,
    Sender: Clone + PartialEq + Identifier,
{
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .encode_vec("basedOn", &self.based_on, encode_reference)?
            .resource_vec("contained", &self.about, encode_any)?
            .encode("status", "unknown", encode_any)?
            .encode_vec("about", self.about.iter().map(|x| &x.id), encode_reference)?
            .encode_opt("sent", &self.sent, encode_any)?
            .encode_opt("received", &self.received, encode_any)?
            .encode_vec(
                "recipient",
                once(&self.recipient),
                encode_reference_identifier,
            )?
            .encode_opt("sender", &self.sender, encode_reference_identifier)?
            .encode_vec("payload", once(&self.payload), encode_any)?;

        Ok(())
    }
}

impl<'a, T> Encode for &'a Payload<T>
where
    &'a T: Encode,
    T: Clone + PartialEq,
{
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode_opt("extension", &self.extensions, encode_any)?
            .encode("contentString", &self.content, encode_any)?
            .end()?;

        Ok(())
    }
}

impl Encode for &InfoReqExtensions {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .array()?
            .element()?
            .attrib("url", URL_INSURANCE_PROVIDER, encode_any)?
            .encode(
                "valueIdentifier",
                &self.insurance_provider,
                encode_identifier,
            )?
            .end()?
            .element()?
            .attrib("url", URL_SUBSTITUTION_ALLOWED, encode_any)?
            .encode("valueBoolean", &self.substitution_allowed, encode_any)?
            .end()?
            .element()?
            .attrib("url", URL_PRESCRIPTION_TYPE, encode_any)?
            .encode("valueCoding", &self.prescription_type, encode_coding)?
            .end()?
            .inline_opt(&self.preferred_supply_options, encode_any)?
            .end()?;

        Ok(())
    }
}

impl Encode for &ReplyExtensions {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream.array()?;

        if let Some(availability) = &self.availability {
            stream
                .element()?
                .attrib("url", URL_AVAILABILITY, encode_any)?
                .encode("valueCoding", availability, encode_coding)?
                .end()?;
        }

        stream
            .inline_opt(&self.offered_supply_options, encode_any)?
            .end()?;

        Ok(())
    }
}

impl Encode for &DispenseReqExtensions {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream.array()?;

        if let Some(insurance_provider) = &self.insurance_provider {
            stream
                .element()?
                .attrib("url", URL_AVAILABILITY, encode_any)?
                .encode("valueIdentifier", insurance_provider, encode_identifier)?
                .end()?;
        }

        stream.end()?;

        Ok(())
    }
}

impl Encode for &RepresentativeExtensions {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream.array()?.end()?;

        Ok(())
    }
}

impl Encode for &SupplyOptions {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .attrib("url", URL_SUPPLY_OPTIONS, encode_any)?
            .field_name("extension")?
            .array()?
            .element()?
            .attrib("url", "onPremise", encode_any)?
            .encode("valueBoolean", &self.on_premise, encode_any)?
            .end()?
            .element()?
            .attrib("url", "delivery", encode_any)?
            .encode("valueBoolean", &self.delivery, encode_any)?
            .end()?
            .element()?
            .attrib("url", "shipment", encode_any)?
            .encode("valueBoolean", &self.shipment, encode_any)?
            .end()?
            .end()?
            .end()?;

        Ok(())
    }
}

fn encode_reference_identifier<T, S>(
    value: &T,
    stream: &mut EncodeStream<S>,
) -> Result<(), EncodeError<S::Error>>
where
    T: Identifier,
    S: DataStorage,
{
    stream
        .element()?
        .encode("identifier", value, encode_identifier)?
        .end()?;

    Ok(())
}

/* Misc */

impl CodingEx for Availability {
    type Code = Self;

    fn from_parts(code: Self::Code) -> Self {
        code
    }

    fn code(&self) -> &Self::Code {
        &self
    }

    fn system() -> Option<&'static str> {
        Some(SYSTEM_AVAILABILITY)
    }
}

impl CodeEx for Availability {
    fn from_parts(value: String) -> Result<Self, String> {
        match value.as_str() {
            "10" => Ok(Self::Now),
            "20" => Ok(Self::Today),
            "30" => Ok(Self::MorningNextDay),
            "40" => Ok(Self::AfternoonNextDay),
            "50" => Ok(Self::Unavailable),
            "90" => Ok(Self::Unknown),
            _ => Err(value),
        }
    }

    fn code(&self) -> &'static str {
        match *self {
            Self::Now => "10",
            Self::Today => "20",
            Self::MorningNextDay => "30",
            Self::AfternoonNextDay => "40",
            Self::Unavailable => "50",
            Self::Unknown => "90",
        }
    }
}

pub const PROFILE_BASE: &str = "http://hl7.org/fhir/StructureDefinition/Communication";
pub const PROFILE_INFO_REQ: &str =
    "https://gematik.de/fhir/StructureDefinition/erxCommunicationInfoReq";
pub const PROFILE_REPLY: &str = "https://gematik.de/fhir/StructureDefinition/erxCommunicationReply";
pub const PROFILE_DISPENSE_REQ: &str =
    "https://gematik.de/fhir/StructureDefinition/erxCommunicationDispReq";
pub const PROFILE_REPRESENTATIVE: &str =
    "https://gematik.de/fhir/StructureDefinition/erxCommunicationRepresentative";

const URL_INSURANCE_PROVIDER: &str =
    "https://gematik.de/fhir/StructureDefinition/InsuranceProvider";
const URL_SUBSTITUTION_ALLOWED: &str =
    "https://gematik.de/fhir/StructureDefinition/SubstitutionAllowedType";
const URL_PRESCRIPTION_TYPE: &str = "https://gematik.de/fhir/StructureDefinition/PrescriptionType";
const URL_SUPPLY_OPTIONS: &str = "https://gematik.de/fhir/StructureDefinition/SupplyOptionsType";
const URL_AVAILABILITY: &str = "https://gematik.de/fhir/StructureDefinition/AvailabilityStatus";

const SYSTEM_AVAILABILITY: &str = "https://gematik.de/fhir/CodeSystem/AvailabilityStatus";

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::convert::TryInto;
    use std::fs::read_to_string;
    use std::str::from_utf8;

    use resources::{
        medication::{
            Amount, Category, Data, Extension, Medication, PznCode, PznData, PznForm, StandardSize,
        },
        misc::InsuranceId,
        types::FlowType,
    };

    use crate::fhir::{
        decode::{tests::load_stream, JsonDecode, XmlDecode},
        encode::{JsonEncode, XmlEncode},
    };

    use super::super::super::tests::{trim_json_str, trim_xml_str};

    #[tokio::test]
    async fn test_decode_json_communication_info_req() {
        let mut stream = load_stream("./examples/communication_info_req.json");

        let actual = stream.json::<Communication>().await.unwrap();
        let expected = test_communication_info_req();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_decode_xml_communication_info_req() {
        let mut stream = load_stream("./examples/communication_info_req.xml");

        let actual = stream.xml::<Communication>().await.unwrap();
        let expected = test_communication_info_req();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_encode_json_communication_info_req() {
        let value = test_communication_info_req();

        let actual = (&value).json().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/communication_info_req.json").unwrap();

        assert_eq!(trim_json_str(&actual), trim_json_str(&expected));
    }

    #[tokio::test]
    async fn test_encode_xml_communication_info_req() {
        let value = test_communication_info_req();

        let actual = (&value).xml().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/communication_info_req.xml").unwrap();

        assert_eq!(trim_xml_str(&actual), trim_xml_str(&expected));
    }

    #[tokio::test]
    async fn test_decode_json_communication_reply() {
        let mut stream = load_stream("./examples/communication_reply.json");

        let actual = stream.json::<Communication>().await.unwrap();
        let expected = test_communication_reply();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_decode_xml_communication_reply() {
        let mut stream = load_stream("./examples/communication_reply.xml");

        let actual = stream.xml::<Communication>().await.unwrap();
        let expected = test_communication_reply();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_encode_json_communication_reply() {
        let value = test_communication_reply();

        let actual = (&value).json().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/communication_reply.json").unwrap();

        assert_eq!(trim_json_str(&actual), trim_json_str(&expected));
    }

    #[tokio::test]
    async fn test_encode_xml_communication_reply() {
        let value = test_communication_reply();

        let actual = (&value).xml().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/communication_reply.xml").unwrap();

        assert_eq!(trim_xml_str(&actual), trim_xml_str(&expected));
    }

    #[tokio::test]
    async fn test_decode_json_communication_dispense_req() {
        let mut stream = load_stream("./examples/communication_dispense_req.json");

        let actual = stream.json::<Communication>().await.unwrap();
        let expected = test_communication_dispense_req();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_decode_xml_communication_dispense_req() {
        let mut stream = load_stream("./examples/communication_dispense_req.xml");

        let actual = stream.xml::<Communication>().await.unwrap();
        let expected = test_communication_dispense_req();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_encode_json_communication_dispense_req() {
        let value = test_communication_dispense_req();

        let actual = (&value).json().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/communication_dispense_req.json").unwrap();

        assert_eq!(trim_json_str(&actual), trim_json_str(&expected));
    }

    #[tokio::test]
    async fn test_encode_xml_communication_dispense_req() {
        let value = test_communication_dispense_req();

        let actual = (&value).xml().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/communication_dispense_req.xml").unwrap();

        assert_eq!(trim_xml_str(&actual), trim_xml_str(&expected));
    }

    pub fn test_communication_info_req() -> Communication {
        Communication::InfoReq(Inner {
            id: None,
            based_on: None,
            about: vec![test_medication()],
            sent: Some("2020-03-12T18:01:10+00:00".try_into().unwrap()),
            received: None,
            recipient: TelematikId("606358757".into()),
            sender: Some(Kvnr::new("X234567890").unwrap()),
            payload: Payload {
                content:
                    "Hallo, ich wollte gern fragen, ob das Medikament bei Ihnen vorraetig ist."
                        .into(),
                extensions: Some(InfoReqExtensions {
                    insurance_provider: InsuranceId::Iknr("104212059".into()),
                    substitution_allowed: true,
                    prescription_type: FlowType::PharmaceuticalDrugs,
                    preferred_supply_options: Some(SupplyOptions {
                        on_premise: true,
                        delivery: true,
                        shipment: false,
                    }),
                }),
            },
        })
    }

    pub fn test_communication_reply() -> Communication {
        Communication::Reply(Inner {
            id: None,
            based_on: None,
            about: vec![],
            sent: Some("2020-03-12T18:01:10+00:00".try_into().unwrap()),
            received: None,
            recipient: Kvnr::new("X234567890").unwrap(),
            sender: Some(TelematikId("606358757".into())),
            payload: Payload {
                content:
                    "Hallo, wir haben das Medikament vorraetig. Kommen Sie gern in die Filiale oder wir schicken einen Boten."
                        .into(),
                extensions: Some(ReplyExtensions {
                    availability: Some(Availability::Now),
                    offered_supply_options: Some(SupplyOptions {
                        on_premise: true,
                        delivery: true,
                        shipment: true,
                    }),
                }),
            },
        })
    }

    pub fn test_communication_dispense_req() -> Communication {
        Communication::DispenseReq(Inner {
            id: None,
            based_on: Some("Task/4711/$accept?ac=777bea0e13cc9c42ceec14aec3ddee2263325dc2c6c699db115f58fe423607ea".into()),
            about: vec![],
            sent: Some("2020-03-12T18:01:10+00:00".try_into().unwrap()),
            received: None,
            recipient: TelematikId("606358757".into()),
            sender: Some(Kvnr::new("X234567890").unwrap()),
            payload: Payload {
                content: "Bitte schicken Sie einen Boten.".into(),
                extensions: None,
            },
        })
    }

    pub fn test_medication() -> Medication {
        Medication {
            id: "5fe6e06c-8725-46d5-aecd-e65e041ca3de".try_into().unwrap(),
            data: Data::Pzn(PznData {
                code: PznCode {
                    text: "Sumatriptan-1a Pharma 100 mg Tabletten".into(),
                    code: "06313728".into(),
                },
                form: PznForm {
                    system: "https://fhir.kbv.de/CodeSystem/KBV_CS_SFHIR_KBV_DARREICHUNGSFORM"
                        .into(),
                    code: "TAB".into(),
                },
                amount: Some(Amount {
                    value: 12,
                    unit: "TAB".into(),
                    code: Some("{tbl}".into()),
                }),
            }),
            extension: Some(Extension {
                category: Category::Medicine,
                vaccine: false,
                instruction: None,
                packaging: None,
                standard_size: Some(StandardSize::N1),
            }),
        }
    }
}
