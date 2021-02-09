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
    misc::Kvnr,
    primitives::{Id, Instant},
    task::{Extension, Identifier, Input, Output, Status, Task},
    types::DocumentType,
};

use crate::fhir::{
    decode::{decode_any, DataStream, Decode, DecodeError, DecodeStream, Fields, Search},
    encode::{encode_any, DataStorage, Encode, EncodeError, EncodeStream},
};

use super::{
    meta::Meta,
    primitives::{
        decode_codeable_concept, decode_coding, decode_identifier, decode_reference,
        encode_codeable_concept, encode_coding, encode_identifier, encode_reference,
    },
};

/* Decode */

#[async_trait(?Send)]
impl Decode for Task {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&[
            "id",
            "meta",
            "extension",
            "identifier",
            "status",
            "intent",
            "for",
            "authoredOn",
            "lastModified",
            "performerType",
            "input",
            "output",
        ]);

        stream.root("Task").await?;

        let id = stream.decode_opt(&mut fields, decode_any).await?;
        let meta = stream.decode::<Meta, _>(&mut fields, decode_any).await?;
        let extension = stream.decode(&mut fields, decode_any).await?;
        let identifier = stream.decode(&mut fields, decode_any).await?;
        let status = stream.decode(&mut fields, decode_any).await?;
        let _intent = stream.fixed(&mut fields, "order").await?;
        let for_ = stream.decode_opt(&mut fields, decode_for).await?;
        let authored_on = stream.decode_opt(&mut fields, decode_any).await?;
        let last_modified = stream.decode_opt(&mut fields, decode_any).await?;
        let performer_type = stream
            .decode_vec(&mut fields, decode_codeable_concept)
            .await?;
        let input = stream.decode(&mut fields, decode_any).await?;
        let output = stream.decode(&mut fields, decode_any).await?;

        stream.end().await?;

        if !meta.profiles.iter().any(|p| icase_eq(p, PROFILE)) {
            return Err(DecodeError::InvalidProfile {
                actual: meta.profiles,
                expected: vec![PROFILE.into()],
            });
        }

        Ok(Task {
            id,
            extension,
            identifier,
            status,
            for_,
            authored_on,
            last_modified,
            performer_type,
            input,
            output,
        })
    }
}

#[async_trait(?Send)]
impl Decode for Extension {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut flow_type = None;
        let mut accept_date = None;
        let mut expiry_date = None;

        let mut fields = Fields::new(&["extension"]);
        while stream.begin_substream_vec(&mut fields).await? {
            stream.element().await?;

            let url = stream.value(Search::Exact("url")).await?.unwrap();

            match url.as_str() {
                x if icase_eq(x, URL_FLOW_TYPE) => {
                    let mut fields = Fields::new(&["valueCoding"]);

                    flow_type = Some(stream.decode(&mut fields, decode_coding).await?);
                }
                x if icase_eq(x, URL_ACCEPT_DATE) => {
                    let mut fields = Fields::new(&["valueDate"]);

                    accept_date = Some(stream.decode(&mut fields, decode_any).await?)
                }
                x if icase_eq(x, URL_EXPIRY_DATE) => {
                    let mut fields = Fields::new(&["valueDate"]);

                    expiry_date = Some(stream.decode(&mut fields, decode_any).await?)
                }
                _ => (),
            }

            stream.end().await?;
            stream.end_substream().await?;
        }

        let flow_type = flow_type.ok_or_else(|| DecodeError::MissingExtension {
            url: URL_FLOW_TYPE.into(),
            path: Default::default(),
        })?;

        Ok(Extension {
            flow_type,
            accept_date,
            expiry_date,
        })
    }
}

#[async_trait(?Send)]
impl Decode for Identifier {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut prescription_id = None;
        let mut access_code = None;
        let mut secret = None;

        let mut fields = Fields::new(&["identifier"]);
        while stream.begin_substream_vec(&mut fields).await? {
            stream.element().await?;

            let mut fields = Fields::new(&["system", "value"]);
            let system: String = stream.decode(&mut fields, decode_any).await?;

            match system.as_str() {
                x if icase_eq(x, SYSTEM_PRESCRIPTION_ID) => {
                    prescription_id = Some(stream.decode(&mut fields, decode_any).await?);
                }
                x if icase_eq(x, SYSTEM_ACCESS_CODE) => {
                    access_code = Some(stream.decode(&mut fields, decode_any).await?);
                }
                x if icase_eq(x, SYSTEM_SECRET) => {
                    secret = Some(stream.decode(&mut fields, decode_any).await?);
                }
                _ => {
                    return Err(DecodeError::UnexpectedValue {
                        value: system.into(),
                        path: Default::default(),
                    })
                }
            }

            stream.end().await?;
            stream.end_substream().await?;
        }

        Ok(Identifier {
            prescription_id,
            access_code,
            secret,
        })
    }
}

#[async_trait(?Send)]
impl Decode for Status {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let value = stream.value(Search::Any).await?.unwrap();

        #[allow(clippy::single_match)]
        match value.as_str() {
            "draft" => Ok(Status::Draft),
            "requested" => Ok(Status::Requested),
            "received" => Ok(Status::Received),
            "accepted" => Ok(Status::Accepted),
            "rejected" => Ok(Status::Rejected),
            "ready" => Ok(Status::Ready),
            "cancelled" => Ok(Status::Cancelled),
            "in-progress" => Ok(Status::InProgress),
            "on-hold" => Ok(Status::OnHold),
            "failed" => Ok(Status::Failed),
            "completed" => Ok(Status::Completed),
            "entered-in-error" => Ok(Status::EnteredInError),
            _ => Err(DecodeError::InvalidValue {
                value,
                path: Default::default(),
            }),
        }
    }
}

#[async_trait(?Send)]
impl Decode for Input {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut e_prescription = None;
        let mut patient_receipt = None;

        let mut fields = Fields::new(&["input"]);
        while stream.begin_substream_vec(&mut fields).await? {
            let mut fields = Fields::new(&["type", "valueReference"]);

            stream.element().await?;

            let document_type = stream.decode(&mut fields, decode_codeable_concept).await?;

            match document_type {
                DocumentType::EPrescription => {
                    e_prescription = Some(stream.decode(&mut fields, decode_reference).await?)
                }
                DocumentType::PatientReceipt => {
                    patient_receipt = Some(stream.decode(&mut fields, decode_reference).await?)
                }
                DocumentType::Receipt => {
                    return Err(DecodeError::UnexpectedValue {
                        value: "3".into(),
                        path: Default::default(),
                    })
                }
            }

            stream.end().await?;
            stream.end_substream().await?;
        }

        Ok(Input {
            e_prescription,
            patient_receipt,
        })
    }
}

#[async_trait(?Send)]
impl Decode for Output {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut receipt = None;

        let mut fields = Fields::new(&["output"]);
        while stream.begin_substream_vec(&mut fields).await? {
            let mut fields = Fields::new(&["type", "valueReference"]);

            stream.element().await?;

            let document_type = stream.decode(&mut fields, decode_codeable_concept).await?;

            match document_type {
                DocumentType::EPrescription => {
                    return Err(DecodeError::UnexpectedValue {
                        value: "1".into(),
                        path: Default::default(),
                    })
                }
                DocumentType::PatientReceipt => {
                    return Err(DecodeError::UnexpectedValue {
                        value: "2".into(),
                        path: Default::default(),
                    })
                }
                DocumentType::Receipt => {
                    receipt = Some(stream.decode(&mut fields, decode_reference).await?)
                }
            }

            stream.end().await?;
            stream.end_substream().await?;
        }

        Ok(Output { receipt })
    }
}

async fn decode_for<S>(stream: &mut DecodeStream<S>) -> Result<Kvnr, DecodeError<S::Error>>
where
    S: DataStream,
{
    let mut fields = Fields::new(&["identifier"]);

    stream.element().await?;

    let kvnr = stream.decode(&mut fields, decode_identifier).await?;

    stream.end().await?;

    Ok(kvnr)
}

/* Encode */

pub struct TaskContainer<'a, T: AsTask>(pub &'a T);

pub trait AsTask {
    fn task(&self) -> &Task;
    fn version_id(&self) -> Option<Id> {
        None
    }
    fn last_updated(&self) -> Option<Instant> {
        None
    }
}

impl<T: AsTask> Encode for TaskContainer<'_, T> {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let task = self.0;
        let meta = Meta {
            version_id: task.version_id(),
            last_updated: task.last_updated(),
            profiles: vec![PROFILE.into()],
        };

        let task = task.task();

        stream
            .root("Task")?
            .encode_opt("id", &task.id, encode_any)?
            .encode("meta", meta, encode_any)?
            .encode("extension", &task.extension, encode_any)?
            .encode("identifier", &task.identifier, encode_any)?
            .encode("status", &task.status, encode_any)?
            .encode("intent", "order", encode_any)?
            .encode_opt("for", &task.for_, encode_for)?
            .encode_opt("authoredOn", &task.authored_on, encode_any)?
            .encode_opt("lastModified", &task.last_modified, encode_any)?
            .encode_vec(
                "performerType",
                &task.performer_type,
                encode_codeable_concept,
            )?
            .encode("input", &task.input, encode_any)?
            .encode("output", &task.output, encode_any)?
            .end()?;

        Ok(())
    }
}

impl Encode for &Extension {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .array()?
            .element()?
            .attrib("url", URL_FLOW_TYPE, encode_any)?
            .encode("valueCoding", &self.flow_type, encode_coding)?
            .end()?;

        if let Some(accept_date) = &self.accept_date {
            stream
                .element()?
                .attrib("url", URL_ACCEPT_DATE, encode_any)?
                .encode("valueDate", accept_date, encode_any)?
                .end()?;
        }

        if let Some(expiry_date) = &self.expiry_date {
            stream
                .element()?
                .attrib("url", URL_EXPIRY_DATE, encode_any)?
                .encode("valueDate", expiry_date, encode_any)?
                .end()?;
        }

        stream.end()?;

        Ok(())
    }
}

impl Encode for &Identifier {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream.array()?;

        if let Some(prescription_id) = &self.prescription_id {
            stream
                .element()?
                .encode("system", SYSTEM_PRESCRIPTION_ID, encode_any)?
                .encode("value", prescription_id, encode_any)?
                .end()?;
        }

        if let Some(access_code) = &self.access_code {
            stream
                .element()?
                .encode("system", SYSTEM_ACCESS_CODE, encode_any)?
                .encode("value", access_code, encode_any)?
                .end()?;
        }

        if let Some(secret) = &self.secret {
            stream
                .element()?
                .encode("system", SYSTEM_SECRET, encode_any)?
                .encode("value", secret, encode_any)?
                .end()?;
        }

        stream.end()?;

        Ok(())
    }
}

impl Encode for &Status {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let value = match self {
            Status::Draft => "draft",
            Status::Requested => "requested",
            Status::Received => "received",
            Status::Accepted => "accepted",
            Status::Rejected => "rejected",
            Status::Ready => "ready",
            Status::Cancelled => "cancelled",
            Status::InProgress => "in-progress",
            Status::OnHold => "on-hold",
            Status::Failed => "failed",
            Status::Completed => "completed",
            Status::EnteredInError => "entered-in-error",
        };

        stream.value(value)?;

        Ok(())
    }
}

impl Encode for &Input {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream.array()?;

        if let Some(e_prescription) = &self.e_prescription {
            stream
                .element()?
                .encode(
                    "type",
                    &DocumentType::EPrescription,
                    encode_codeable_concept,
                )?
                .encode("valueReference", e_prescription, encode_reference)?
                .end()?;
        }

        if let Some(patient_receipt) = &self.patient_receipt {
            stream
                .element()?
                .encode(
                    "type",
                    &DocumentType::PatientReceipt,
                    encode_codeable_concept,
                )?
                .encode("valueReference", patient_receipt, encode_reference)?
                .end()?;
        }

        stream.end()?;

        Ok(())
    }
}

impl Encode for &Output {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream.array()?;

        if let Some(receipt) = &self.receipt {
            stream
                .element()?
                .encode("type", &DocumentType::Receipt, encode_codeable_concept)?
                .encode("valueReference", receipt, encode_reference)?
                .end()?;
        }

        stream.end()?;

        Ok(())
    }
}

fn encode_for<S>(value: &Kvnr, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
where
    S: DataStorage,
{
    stream
        .element()?
        .encode("identifier", value, encode_identifier)?
        .end()?;

    Ok(())
}

pub const PROFILE: &str = "https://gematik.de/fhir/StructureDefinition/erxTask";

pub const OPERATION_CREATE: &str =
    "http://gematik.de/fhir/OperationDefinition/CreateOperationDefinition";
pub const OPERATION_ACTIVATE: &str =
    "http://gematik.de/fhir/OperationDefinition/ActivateOperationDefinition";
pub const OPERATION_ACCEPT: &str =
    "http://gematik.de/fhir/OperationDefinition/AcceptOperationDefinition";
pub const OPERATION_REJECT: &str =
    "http://gematik.de/fhir/OperationDefinition/RejectOperationDefinition";
pub const OPERATION_CLOSE: &str =
    "http://gematik.de/fhir/OperationDefinition/CloseOperationDefinition";
pub const OPERATION_ABORT: &str =
    "http://gematik.de/fhir/OperationDefinition/AbortOperationDefinition";

const URL_FLOW_TYPE: &str = "https://gematik.de/fhir/StructureDefinition/PrescriptionType";
const URL_ACCEPT_DATE: &str = "https://example.org/fhir/StructureDefinition/AcceptDate";
const URL_EXPIRY_DATE: &str = "https://gematik.de/fhir/StructureDefinition/ExpiryDate";

const SYSTEM_PRESCRIPTION_ID: &str = "https://gematik.de/fhir/NamingSystem/PrescriptionID";
const SYSTEM_ACCESS_CODE: &str = "https://gematik.de/fhir/Namingsystem/AccessCode";
const SYSTEM_SECRET: &str = "https://gematik.de/fhir/Namingsystem/Secret";

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::convert::TryInto;
    use std::fs::read_to_string;
    use std::str::from_utf8;

    use resources::types::{FlowType, PerformerType};

    use crate::fhir::{
        decode::{tests::load_stream, JsonDecode, XmlDecode},
        encode::{JsonEncode, XmlEncode},
    };

    use super::super::super::tests::{trim_json_str, trim_xml_str};

    impl AsTask for Task {
        fn task(&self) -> &Task {
            self
        }
    }

    #[tokio::test]
    async fn test_decode_json() {
        let mut stream = load_stream("./examples/task.json");

        let actual = stream.json::<Task>().await.unwrap();
        let expected = test_task();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_decode_xml() {
        let mut stream = load_stream("./examples/task.xml");

        let actual = stream.xml::<Task>().await.unwrap();
        let expected = test_task();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_encode_json() {
        let value = test_task();

        let actual = TaskContainer(&value).json().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/task.json").unwrap();

        assert_eq!(trim_json_str(&actual), trim_json_str(&expected));
    }

    #[tokio::test]
    async fn test_encode_xml() {
        let value = test_task();

        let actual = TaskContainer(&value).xml().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/task.xml").unwrap();

        assert_eq!(trim_xml_str(&actual), trim_xml_str(&expected));
    }

    fn test_task() -> Task {
        Task {
            id: None,
            extension: Extension {
                accept_date: Some("2020-03-02".try_into().unwrap()),
                expiry_date: Some("2020-05-02".try_into().unwrap()),
                flow_type: FlowType::PharmaceuticalDrugs,
            },
            identifier: Identifier {
                prescription_id: Some("160.123.456.789.123.58".parse().unwrap()),
                access_code: Some(
                    "777bea0e13cc9c42ceec14aec3ddee2263325dc2c6c699db115f58fe423607ea".into(),
                ),
                secret: Some(
                    "c36ca26502892b371d252c99b496e31505ff449aca9bc69e231c58148f6233cf".into(),
                ),
            },
            status: Status::InProgress,
            for_: Some(Kvnr::new("X123456789").unwrap()),
            authored_on: Some("2020-03-02T08:25:05+00:00".try_into().unwrap()),
            last_modified: Some("2020-03-02T08:45:05+00:00".try_into().unwrap()),
            performer_type: vec![PerformerType::PublicPharmacy],
            input: Input {
                e_prescription: Some("Bundle/KbvPrescriptionExample".try_into().unwrap()),
                patient_receipt: Some("Bundle/KbvPatientReceiptExample".try_into().unwrap()),
            },
            output: Output {
                receipt: Some("Bundle/KbvReceiptExample".try_into().unwrap()),
            },
        }
    }
}
