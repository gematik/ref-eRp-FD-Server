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
use resources::capability_statement::{
    CapabilityStatement, FhirVersion, Format, Interaction, Mode, Operation, Resource, Rest, Status,
    Type,
};

use crate::fhir::{
    decode::{decode_any, DataStream, Decode, DecodeError, DecodeStream, Fields, Search},
    encode::{encode_any, DataStorage, Encode, EncodeError, EncodeStream},
};

use super::primitives::{decode_coding, encode_coding, CodeEx, CodingEx};

/* Decode */

#[async_trait(?Send)]
impl Decode for CapabilityStatement {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&[
            "name",
            "title",
            "status",
            "date",
            "kind",
            "implementation",
            "fhirVersion",
            "format",
            "rest",
        ]);

        stream.root("CapabilityStatement").await?;

        let name = stream.decode(&mut fields, decode_any).await?;
        let title = stream.decode(&mut fields, decode_any).await?;
        let status = stream.decode(&mut fields, decode_any).await?;
        let date = stream.decode(&mut fields, decode_any).await?;
        let _kind = stream.fixed(&mut fields, "instance").await?;
        let description = stream.decode(&mut fields, decode_description).await?;
        let fhir_version = stream.decode(&mut fields, decode_any).await?;
        let format = stream.decode_vec(&mut fields, decode_any).await?;
        let rest = stream.decode_vec(&mut fields, decode_any).await?;

        stream.end().await?;

        Ok(CapabilityStatement {
            name,
            title,
            status,
            date,
            description,
            fhir_version,
            format,
            rest,
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

        match value.as_str() {
            "draft" => Ok(Self::Draft),
            "active" => Ok(Self::Active),
            "retired" => Ok(Self::Retired),
            "unknown" => Ok(Self::Unknown),
            _ => Err(DecodeError::InvalidValue {
                value,
                path: stream.path().into(),
            }),
        }
    }
}

#[async_trait(?Send)]
impl Decode for FhirVersion {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let value = stream.value(Search::Any).await?.unwrap();

        match value.as_str() {
            "4.0.0" => Ok(Self::V4_0_0),
            "4.0.1" => Ok(Self::V4_0_1),
            _ => Err(DecodeError::InvalidValue {
                value,
                path: stream.path().into(),
            }),
        }
    }
}

#[async_trait(?Send)]
impl Decode for Format {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let value = stream.value(Search::Any).await?.unwrap();

        match value.as_str() {
            "xml" => Ok(Self::XML),
            "json" => Ok(Self::JSON),
            _ => Err(DecodeError::InvalidValue {
                value,
                path: stream.path().into(),
            }),
        }
    }
}

#[async_trait(?Send)]
impl Decode for Rest {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["mode", "resource"]);

        stream.root("Rest").await?;

        let mode = stream.decode(&mut fields, decode_any).await?;
        let resource = stream.decode_vec(&mut fields, decode_any).await?;

        stream.end().await?;

        Ok(Rest { mode, resource })
    }
}

#[async_trait(?Send)]
impl Decode for Mode {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let value = stream.value(Search::Any).await?.unwrap();

        match value.as_str() {
            "client" => Ok(Self::Client),
            "server" => Ok(Self::Server),
            _ => Err(DecodeError::InvalidValue {
                value,
                path: stream.path().into(),
            }),
        }
    }
}

#[async_trait(?Send)]
impl Decode for Resource {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&[
            "type",
            "profile",
            "supportedProfile",
            "interaction",
            "operation",
        ]);

        stream.root("Rest").await?;

        let type_ = stream.decode(&mut fields, decode_any).await?;
        let profile = stream.decode(&mut fields, decode_any).await?;
        let supported_profiles = stream.decode_vec(&mut fields, decode_any).await?;
        let interaction = stream.decode_vec(&mut fields, decode_coding).await?;
        let operation = stream.decode_vec(&mut fields, decode_any).await?;

        stream.end().await?;

        Ok(Resource {
            type_,
            profile,
            supported_profiles,
            interaction,
            operation,
        })
    }
}

#[async_trait(?Send)]
impl Decode for Type {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let value = stream.value(Search::Any).await?.unwrap();

        match value.as_str() {
            "Task" => Ok(Self::Task),
            "Operation" => Ok(Self::Operation),
            "Communication" => Ok(Self::Communication),
            "MedicationDispense" => Ok(Self::MedicationDispense),
            _ => Err(DecodeError::InvalidValue {
                value,
                path: stream.path().into(),
            }),
        }
    }
}

#[async_trait(?Send)]
impl Decode for Operation {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["name", "definition"]);

        stream.root("Operation").await?;

        let name = stream.decode(&mut fields, decode_any).await?;
        let definition = stream.decode(&mut fields, decode_any).await?;

        stream.end().await?;

        Ok(Operation { name, definition })
    }
}

async fn decode_description<S>(
    stream: &mut DecodeStream<S>,
) -> Result<String, DecodeError<S::Error>>
where
    S: DataStream,
{
    let mut fields = Fields::new(&["description"]);

    stream.element().await?;

    let description = stream.decode(&mut fields, decode_any).await?;

    stream.end().await?;

    Ok(description)
}

/* Encode */

impl Encode for &CapabilityStatement {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .root("CapabilityStatement")?
            .encode("name", &self.name, encode_any)?
            .encode("title", &self.title, encode_any)?
            .encode("status", &self.status, encode_any)?
            .encode("date", &self.date, encode_any)?
            .encode("kind", "instance", encode_any)?
            .encode("implementation", &self.description, encode_description)?
            .encode("fhirVersion", &self.fhir_version, encode_any)?
            .encode_vec("format", &self.format, encode_any)?
            .encode_vec("rest", &self.rest, encode_any)?
            .end()?;

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
            Status::Active => "active",
            Status::Retired => "retired",
            Status::Unknown => "unknown",
        };

        stream.value(value)?;

        Ok(())
    }
}

impl Encode for &FhirVersion {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let value = match self {
            FhirVersion::V4_0_0 => "4.0.0",
            FhirVersion::V4_0_1 => "4.0.1",
        };

        stream.value(value)?;

        Ok(())
    }
}

impl Encode for &Format {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let value = match self {
            Format::XML => "xml",
            Format::JSON => "json",
        };

        stream.value(value)?;

        Ok(())
    }
}

impl Encode for &Rest {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode("mode", &self.mode, encode_any)?
            .encode_vec("resource", &self.resource, encode_any)?
            .end()?;

        Ok(())
    }
}

impl Encode for &Mode {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let value = match self {
            Mode::Client => "client",
            Mode::Server => "server",
        };

        stream.value(value)?;

        Ok(())
    }
}

impl Encode for &Resource {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode("type", &self.type_, encode_any)?
            .encode("profile", &self.profile, encode_any)?
            .encode_vec("supportedProfile", &self.supported_profiles, encode_any)?
            .encode_vec("interaction", &self.interaction, encode_coding)?
            .encode_vec("operation", &self.operation, encode_any)?
            .end()?;

        Ok(())
    }
}

impl Encode for &Type {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let value = match self {
            Type::Task => "Task",
            Type::Operation => "Operation",
            Type::Communication => "Communication",
            Type::MedicationDispense => "MedicationDispense",
        };

        stream.value(value)?;

        Ok(())
    }
}

impl Encode for &Operation {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode("name", &self.name, encode_any)?
            .encode("definition", &self.definition, encode_any)?
            .end()?;

        Ok(())
    }
}

#[allow(clippy::ptr_arg)]
fn encode_description<S>(
    value: &String,
    stream: &mut EncodeStream<S>,
) -> Result<(), EncodeError<S::Error>>
where
    S: DataStorage,
{
    stream
        .element()?
        .encode("description", value, encode_any)?
        .end()?;

    Ok(())
}

/* Misc */

impl CodeEx for Interaction {
    fn from_parts(value: String) -> Result<Self, String> {
        match value.as_str() {
            "read" => Ok(Self::Read),
            "vread" => Ok(Self::Vread),
            "update" => Ok(Self::Update),
            "patch" => Ok(Self::Patch),
            "delete" => Ok(Self::Delete),
            "history-instance" => Ok(Self::HistoryInstance),
            "history-type" => Ok(Self::HistoryType),
            "create" => Ok(Self::Create),
            "search-typ" => Ok(Self::SearchTyp),
            _ => Err(value),
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Vread => "vread",
            Self::Update => "update",
            Self::Patch => "patch",
            Self::Delete => "delete",
            Self::HistoryInstance => "history-instance",
            Self::HistoryType => "history-type",
            Self::Create => "create",
            Self::SearchTyp => "search-typ",
        }
    }
}

impl CodingEx for Interaction {
    type Code = Self;

    fn from_parts(code: Self::Code) -> Self {
        code
    }

    fn code(&self) -> &Self::Code {
        &self
    }
}

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
        let mut stream = load_stream("./examples/capability_statement_both.json");

        let actual = stream.json::<CapabilityStatement>().await.unwrap();
        let expected = test_capability_statement();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_decode_xml() {
        let mut stream = load_stream("./examples/capability_statement_both.xml");

        let actual = stream.xml::<CapabilityStatement>().await.unwrap();
        let expected = test_capability_statement();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_encode_json() {
        let value = test_capability_statement();

        let actual = (&value).json().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/capability_statement_both.json").unwrap();

        assert_eq!(trim_json_str(&actual), trim_json_str(&expected));
    }

    #[tokio::test]
    async fn test_encode_xml() {
        let value = test_capability_statement();

        let actual = (&value).xml().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/capability_statement_both.xml").unwrap();

        assert_eq!(trim_xml_str(&actual), trim_xml_str(&expected));
    }

    fn test_capability_statement() -> CapabilityStatement {
        CapabilityStatement {
            name: "Gem_erxCapabilityStatement".into(),
            title: "E-Rezept Workflow CapabilityStatement".into(),
            description: "E-Rezept Fachdienst Server Referenzimplementierung".into(),
            status: Status::Draft,
            date: "2020-01-01T00:00:00Z".try_into().unwrap(),
            fhir_version: FhirVersion::V4_0_0,
            format: vec![
                Format::XML,
                Format::JSON,
            ],
            rest: vec![
                Rest {
                    mode: Mode::Server,
                    resource: vec![
                        Resource {
                            type_: Type::Task,
                            profile: "https://gematik.de/fhir/StructureDefinition/erxTask".into(),
                            supported_profiles: vec![],
                            operation: vec![Operation{
                                name: "create".into(),
                                definition: "http://gematik.de/fhir/OperationDefinition/CreateOperationDefinition".into(),
                            },Operation{
                                name: "activate".into(),
                                definition: "http://gematik.de/fhir/OperationDefinition/ActivateOperationDefinition".into(),
                            },Operation{
                                name: "abort".into(),
                                definition: "http://gematik.de/fhir/OperationDefinition/AbortOperationDefinition".into(),
                            }],
                            interaction: vec![Interaction::Read],
                        },
                        Resource {
                            type_: Type::Communication,
                            profile: "http://hl7.org/fhir/StructureDefinition/Communication".into(),
                            supported_profiles: vec![
                                "https://gematik.de/fhir/StructureDefinition/erxCommunicationInfoReq".into(),
                                "https://gematik.de/fhir/StructureDefinition/erxCommunicationReply".into(),
                                "https://gematik.de/fhir/StructureDefinition/erxCommunicationDispReq".into(),
                                "https://gematik.de/fhir/StructureDefinition/erxCommunicationRepresentative".into(),
                            ],
                            operation: vec![],
                            interaction: vec![Interaction::Create, Interaction::Read, Interaction::Delete],
                        },
                        Resource {
                            type_: Type::MedicationDispense,
                            profile: "https://gematik.de/fhir/StructureDefinition/ERxMedicationDispense".into(),
                            supported_profiles: vec![],
                            operation: vec![],
                            interaction: vec![Interaction::Read],
                        }
                    ],
                }
            ],
        }
    }
}
