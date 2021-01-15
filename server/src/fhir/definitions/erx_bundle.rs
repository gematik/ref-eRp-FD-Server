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

use async_trait::async_trait;
use miscellaneous::str::icase_eq;
use resources::{
    erx_bundle::{Entry, ErxBundle},
    Device, ErxComposition, SignatureFormat,
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
    Composition(ErxComposition),
    Device(Device),
}

#[async_trait(?Send)]
impl Decode for ErxBundle {
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
            let mut device = None;

            loop {
                if stream.begin_substream_vec(&mut fields).await? {
                    stream.element().await?;

                    let mut fields = Fields::new(&["resource"]);
                    let resource = stream.resource(&mut fields, decode_any).await?;

                    match resource {
                        Resource::Composition(v) => composition = Some(v),
                        Resource::Device(v) => device = Some(v),
                    }

                    stream.end().await?;
                    stream.end_substream().await?;
                } else {
                    break Entry {
                        composition,
                        device,
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

        Ok(ErxBundle {
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
            "Device" => Ok(Self::Device(
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

impl Encode for &ErxBundle {
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
                    (Some(SignatureFormat::Xml), Some(Format::Xml)) => true,
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
            .inline_opt(self.entry.composition.as_ref().map(EntryPair), encode_any)?
            .inline_opt(self.entry.device.as_ref().map(EntryPair), encode_any)?
            .end()?
            .encode_opt("signature", signature, encode_any)?
            .end()?;

        Ok(())
    }
}

struct EntryPair<'a, T>(&'a T);

impl<'a, T> Encode for EntryPair<'a, T>
where
    for<'x> &'x T: Encode,
{
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .resource("resource", self.0, encode_any)?
            .end()?;

        Ok(())
    }
}

const PROFILE: &str = "https://gematik.de/fhir/StructureDefinition/erxReceipt";

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::convert::TryInto;
    use std::fs::read_to_string;
    use std::str::from_utf8;

    use chrono::DateTime;
    use resources::{
        misc::PrescriptionId, types::FlowType, Signature, SignatureFormat, SignatureType,
    };

    use crate::fhir::{
        decode::{tests::load_stream, JsonDecode, XmlDecode},
        encode::{JsonEncode, XmlEncode},
    };

    use super::super::{
        super::tests::{trim_json_str, trim_xml_str},
        device::tests::test_device,
        erx_composition::tests::test_erx_composition,
    };

    #[tokio::test]
    async fn test_decode_json() {
        let mut stream = load_stream("./examples/erx_bundle.json");

        let actual = stream.json::<ErxBundle>().await.unwrap();
        let mut expected = test_erx_bundle();
        expected.signature.clear();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_decode_xml() {
        let mut stream = load_stream("./examples/erx_bundle.xml");

        let actual = stream.xml::<ErxBundle>().await.unwrap();
        let expected = test_erx_bundle();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_encode_json() {
        let value = test_erx_bundle();

        let actual = (&value).json().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/erx_bundle.json").unwrap();

        assert_eq!(trim_json_str(&actual), trim_json_str(&expected));
    }

    #[tokio::test]
    async fn test_encode_xml() {
        let value = test_erx_bundle();

        let actual = (&value).xml().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/erx_bundle.xml").unwrap();

        assert_eq!(trim_xml_str(&actual), trim_xml_str(&expected));
    }

    pub fn test_erx_bundle() -> ErxBundle {
        ErxBundle {
            id: "281a985c-f25b-4aae-91a6-41ad744080b0".try_into().unwrap(),
            identifier: PrescriptionId::new(FlowType::PharmaceuticalDrugs, 123456789123),
            timestamp: DateTime::parse_from_rfc3339("2020-03-20T07:31:34.328+00:00")
                .unwrap()
                .into(),
            entry: Entry {
                composition: Some(test_erx_composition()),
                device: Some(test_device()),
            },
            signature: vec![
                Signature {
                    type_: SignatureType::AuthorsSignature,
                    when: "2020-03-20T07:31:34.328+00:00".try_into().unwrap(),
                    who: "https://prescriptionserver.telematik/Device/eRxService".into(),
                    data: "QXVmZ3J1bmQgZGVyIENvcm9uYS1TaXR1YXRpb24ga29ubnRlIGhpZXIga3VyemZyaXN0aWcga2VpbiBCZWlzcGllbCBpbiBkZXIgTGFib3J1bWdlYnVuZyBkZXIgZ2VtYXRpayBlcnN0ZWxsdCB3ZWRlbi4gRGllc2VzIHdpcmQgbmFjaGdlcmVpY2h0LgoKSW5oYWx0bGljaCB1bmQgc3RydWt0dXJlbGwgaXN0IGRpZSBTZXJ2ZXJzaWduYXR1ciBkZXIgUXVpdHR1bmcgZWluZSBFbnZlbG9waW5nIENBZEVTLVNpZ25hdHVyLCBkaWUgZGVuIHNpZ25pZXJ0ZW4gRGF0ZW5zYXR6IGFuYWxvZyB6dXIgS29ubmVrdG9yLVNpZ25hdHVyIGlubmVyaGFsYiBkZXMgQVNOMS5Db250YWluZXJzIHRyYW5zcG9ydGllcnQu".into(),
                    format: Some(SignatureFormat::Xml),
                }
            ],
        }
    }
}
