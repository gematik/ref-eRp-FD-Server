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
use resources::audit_event::{
    Action, Agent, AuditEvent, Entity, Outcome, ParticipationRoleType, Source, SubType,
};

use crate::fhir::{
    decode::{decode_any, DataStream, Decode, DecodeError, DecodeStream, Fields},
    encode::{encode_any, DataStorage, Encode, EncodeError, EncodeStream},
};

use super::{
    bundle::{DecodeBundleResource, EncodeBundleResource},
    meta::Meta,
    primitives::{
        decode_code, decode_codeable_concept, decode_coding, decode_identifier_reference,
        decode_reference, encode_code, encode_codeable_concept, encode_coding,
        encode_identifier_reference, encode_reference, CodeEx, CodeableConceptEx, Coding, CodingEx,
    },
};

/* Decode */

impl DecodeBundleResource for AuditEvent {}

#[async_trait(?Send)]
impl Decode for AuditEvent {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&[
            "id", "meta", "type", "subType", "action", "recorded", "outcome", "agent", "source",
            "entity",
        ]);

        stream.root("AuditEvent").await?;

        let id = stream.decode_opt(&mut fields, decode_any).await?;
        let meta = stream.decode::<Meta, _>(&mut fields, decode_any).await?;
        let _type = {
            stream.begin_substream(&mut fields).await?;
            stream.element().await?;

            let mut fields = Fields::new(&["system", "code"]);
            stream.fixed(&mut fields, SYSTEM_TYPE).await?;
            stream.fixed(&mut fields, "rest").await?;

            stream.end().await?;
            stream.end_substream().await?;
        };
        let sub_type = stream.decode(&mut fields, decode_coding).await?;
        let action = stream.decode(&mut fields, decode_code).await?;
        let recorded = stream.decode(&mut fields, decode_any).await?;
        let outcome = stream.decode(&mut fields, decode_code).await?;
        let agent = stream.decode(&mut fields, decode_any).await?;
        let source = stream.decode(&mut fields, decode_any).await?;
        let entity = stream.decode(&mut fields, decode_any).await?;

        stream.end().await?;

        if !meta.profiles.iter().any(|p| icase_eq(p, PROFILE)) {
            return Err(DecodeError::InvalidProfile {
                actual: meta.profiles,
                expected: vec![PROFILE.into()],
            });
        }

        Ok(AuditEvent {
            id,
            sub_type,
            action,
            recorded,
            outcome,
            agent,
            source,
            entity,
        })
    }
}

#[async_trait(?Send)]
impl Decode for Agent {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["type", "who", "name", "requestor"]);

        stream.element().await?;

        let type_ = stream.decode(&mut fields, decode_codeable_concept).await?;
        let who = stream
            .decode(&mut fields, decode_identifier_reference)
            .await?;
        let name = stream.decode(&mut fields, decode_any).await?;
        let requestor = stream.decode(&mut fields, decode_any).await?;

        stream.end().await?;

        Ok(Agent {
            type_,
            who,
            name,
            requestor,
        })
    }
}

#[async_trait(?Send)]
impl Decode for Source {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["observer"]);

        stream.element().await?;

        let observer = stream.decode(&mut fields, decode_reference).await?;

        stream.end().await?;

        Ok(Source { observer })
    }
}

#[async_trait(?Send)]
impl Decode for Entity {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["what", "name", "description"]);

        stream.element().await?;

        let what = stream.decode(&mut fields, decode_reference).await?;
        let name = stream.decode(&mut fields, decode_any).await?;
        let description = stream.decode(&mut fields, decode_any).await?;

        stream.end().await?;

        Ok(Entity {
            what,
            name,
            description,
        })
    }
}

/* Encode */

impl EncodeBundleResource for &AuditEvent {}

impl Encode for &AuditEvent {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let meta = Meta {
            profiles: vec![PROFILE.into()],
        };

        stream
            .root("AuditEvent")?
            .encode_opt("id", &self.id, encode_any)?
            .encode("meta", meta, encode_any)?
            .field_name("type")?
            .element()?
            .encode("system", SYSTEM_TYPE, encode_any)?
            .encode("code", "rest", encode_any)?
            .end()?
            .encode("subType", &self.sub_type, encode_coding)?
            .encode("action", &self.action, encode_code)?
            .encode("recorded", &self.recorded, encode_any)?
            .encode("outcome", &self.outcome, encode_code)?
            .encode_vec("agent", once(&self.agent), encode_any)?
            .encode("source", &self.source, encode_any)?
            .encode_vec("entity", once(&self.entity), encode_any)?
            .end()?;

        Ok(())
    }
}

impl Encode for &Agent {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode("type", &self.type_, encode_codeable_concept)?
            .encode("who", &self.who, encode_identifier_reference)?
            .encode("name", &self.name, encode_any)?
            .encode("requestor", &self.requestor, encode_any)?
            .end()?;

        Ok(())
    }
}

impl Encode for &Source {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode("observer", &self.observer, encode_reference)?
            .end()?;

        Ok(())
    }
}

impl Encode for &Entity {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode("what", &self.what, encode_reference)?
            .encode("name", &self.name, encode_any)?
            .encode("description", &self.description, encode_any)?
            .end()?;

        Ok(())
    }
}

/* Misc */

#[async_trait(?Send)]
impl Coding for SubType {
    async fn decode_coding<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["system", "code"]);

        stream.element().await?;

        let system = stream.decode::<String, _>(&mut fields, decode_any).await?;
        let code = stream.decode::<String, _>(&mut fields, decode_any).await?;

        stream.end().await?;

        match system.as_str() {
            x if icase_eq(x, SYSTEM_REST) => match code.as_str() {
                "read" => Ok(Self::Read),
                "vread" => Ok(Self::VRead),
                "update" => Ok(Self::Update),
                "patch" => Ok(Self::Patch),
                "delete" => Ok(Self::Delete),
                "history" => Ok(Self::History),
                "history-instance" => Ok(Self::HistoryInstance),
                "history-type" => Ok(Self::HistoryType),
                "history-system" => Ok(Self::HistorySystem),
                "create" => Ok(Self::Create),
                "search" => Ok(Self::Search),
                "search-type" => Ok(Self::SearchType),
                "search-system" => Ok(Self::SearchSystem),
                "capabilities" => Ok(Self::Capabilities),
                "transaction" => Ok(Self::Transaction),
                "batch" => Ok(Self::Batch),
                "operation" => Ok(Self::Operation),
                _ => Err(DecodeError::InvalidValue {
                    value: system,
                    path: stream.path().into(),
                }),
            },
            x if icase_eq(x, SYSTEM_DCM) => match code.as_str() {
                "110120" => Ok(Self::ApplicationStart),
                "110121" => Ok(Self::ApplicationStop),
                "110122" => Ok(Self::Login),
                "110123" => Ok(Self::Logout),
                "110124" => Ok(Self::Attach),
                "110125" => Ok(Self::Detach),
                "110126" => Ok(Self::NodeAuthentication),
                "110127" => Ok(Self::EmergencyOverrideStarted),
                "110128" => Ok(Self::NetworkConfiguration),
                "110129" => Ok(Self::SecurityConfiguration),
                "110130" => Ok(Self::HardwareConfiguration),
                "110131" => Ok(Self::SoftwareConfiguration),
                "110132" => Ok(Self::UseOfRestrictedFunction),
                "110133" => Ok(Self::AuditRecordingStopped),
                "110134" => Ok(Self::AuditRecordingStarted),
                "110135" => Ok(Self::ObjectSecurityAttributesChanged),
                "110136" => Ok(Self::SecurityRolesChanged),
                "110137" => Ok(Self::UserSecurityAttributesChanged),
                "110138" => Ok(Self::EmergencyOverrideStopped),
                "110139" => Ok(Self::RemoteServiceOperationStarted),
                "110140" => Ok(Self::RemoteServiceOperationStopped),
                "110141" => Ok(Self::LocalServiceOperationStarted),
                "110142" => Ok(Self::LocalServiceOperationStopped),
                _ => Err(DecodeError::InvalidValue {
                    value: system,
                    path: stream.path().into(),
                }),
            },
            x => Err(DecodeError::InvalidFixedValue {
                actual: Some(x).into(),
                expected: Some(format!("{} | {}", SYSTEM_REST, SYSTEM_DCM)).into(),
                path: stream.path().into(),
            }),
        }
    }

    fn encode_coding<S>(&self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let (system, code) = match self {
            Self::Read => (SYSTEM_REST, "read"),
            Self::VRead => (SYSTEM_REST, "vread"),
            Self::Update => (SYSTEM_REST, "update"),
            Self::Patch => (SYSTEM_REST, "patch"),
            Self::Delete => (SYSTEM_REST, "delete"),
            Self::History => (SYSTEM_REST, "history"),
            Self::HistoryInstance => (SYSTEM_REST, "history-instance"),
            Self::HistoryType => (SYSTEM_REST, "history-type"),
            Self::HistorySystem => (SYSTEM_REST, "history-system"),
            Self::Create => (SYSTEM_REST, "create"),
            Self::Search => (SYSTEM_REST, "search"),
            Self::SearchType => (SYSTEM_REST, "search-type"),
            Self::SearchSystem => (SYSTEM_REST, "search-system"),
            Self::Capabilities => (SYSTEM_REST, "capabilities"),
            Self::Transaction => (SYSTEM_REST, "transaction"),
            Self::Batch => (SYSTEM_REST, "batch"),
            Self::Operation => (SYSTEM_REST, "operation"),
            Self::ApplicationStart => (SYSTEM_DCM, "110120"),
            Self::ApplicationStop => (SYSTEM_DCM, "110121"),
            Self::Login => (SYSTEM_DCM, "110122"),
            Self::Logout => (SYSTEM_DCM, "110123"),
            Self::Attach => (SYSTEM_DCM, "110124"),
            Self::Detach => (SYSTEM_DCM, "110125"),
            Self::NodeAuthentication => (SYSTEM_DCM, "110126"),
            Self::EmergencyOverrideStarted => (SYSTEM_DCM, "110127"),
            Self::NetworkConfiguration => (SYSTEM_DCM, "110128"),
            Self::SecurityConfiguration => (SYSTEM_DCM, "110129"),
            Self::HardwareConfiguration => (SYSTEM_DCM, "110130"),
            Self::SoftwareConfiguration => (SYSTEM_DCM, "110131"),
            Self::UseOfRestrictedFunction => (SYSTEM_DCM, "110132"),
            Self::AuditRecordingStopped => (SYSTEM_DCM, "110133"),
            Self::AuditRecordingStarted => (SYSTEM_DCM, "110134"),
            Self::ObjectSecurityAttributesChanged => (SYSTEM_DCM, "110135"),
            Self::SecurityRolesChanged => (SYSTEM_DCM, "110136"),
            Self::UserSecurityAttributesChanged => (SYSTEM_DCM, "110137"),
            Self::EmergencyOverrideStopped => (SYSTEM_DCM, "110138"),
            Self::RemoteServiceOperationStarted => (SYSTEM_DCM, "110139"),
            Self::RemoteServiceOperationStopped => (SYSTEM_DCM, "110140"),
            Self::LocalServiceOperationStarted => (SYSTEM_DCM, "110141"),
            Self::LocalServiceOperationStopped => (SYSTEM_DCM, "110142"),
        };

        stream
            .element()?
            .encode("system", system, encode_any)?
            .encode("code", code, encode_any)?
            .end()?;

        Ok(())
    }
}

impl CodeEx for Action {
    fn from_parts(value: String) -> Result<Self, String> {
        match value.as_str() {
            "C" => Ok(Self::Create),
            "R" => Ok(Self::Read),
            "U" => Ok(Self::Update),
            "D" => Ok(Self::Delete),
            "E" => Ok(Self::Execute),
            _ => Err(value),
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Create => "C",
            Self::Read => "R",
            Self::Update => "U",
            Self::Delete => "D",
            Self::Execute => "E",
        }
    }
}

impl CodeEx for Outcome {
    fn from_parts(value: String) -> Result<Self, String> {
        match value.as_str() {
            "0" => Ok(Self::Success),
            "4" => Ok(Self::MinorFailure),
            "8" => Ok(Self::SeriousFailure),
            "12" => Ok(Self::MajorFailure),
            _ => Err(value),
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Success => "0",
            Self::MinorFailure => "4",
            Self::SeriousFailure => "8",
            Self::MajorFailure => "12",
        }
    }
}

impl CodeableConceptEx for ParticipationRoleType {
    type Coding = Self;

    fn from_parts(coding: Self::Coding, _text: Option<String>) -> Self {
        coding
    }

    fn coding(&self) -> &Self::Coding {
        &self
    }
}

impl CodingEx for ParticipationRoleType {
    type Code = Self;

    fn from_parts(code: Self::Code) -> Self {
        code
    }

    fn code(&self) -> &Self::Code {
        &self
    }

    fn display(&self) -> Option<&'static str> {
        Some("human user")
    }

    fn system() -> Option<&'static str> {
        Some(SYSTEM_ROLE_TYPE)
    }
}

impl CodeEx for ParticipationRoleType {
    fn from_parts(value: String) -> Result<Self, String> {
        match value.as_str() {
            "humanuser" => Ok(Self::HumanUser),
            _ => Err(value),
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::HumanUser => "humanuser",
        }
    }
}

pub const PROFILE: &str = "https://gematik.de/fhir/StructureDefinition/erxAuditEvent";

const SYSTEM_DCM: &str = "http://dicom.nema.org/resources/ontology/DCM";
const SYSTEM_REST: &str = "http://hl7.org/fhir/restful-interaction";
const SYSTEM_TYPE: &str = "http://terminology.hl7.org/CodeSystem/audit-event-type";
const SYSTEM_ROLE_TYPE: &str = "http://terminology.hl7.org/CodeSystem/extra-security-role-type";

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::convert::TryInto;
    use std::fs::read_to_string;
    use std::str::from_utf8;

    use resources::misc::{Kvnr, TelematikId};

    use crate::fhir::{
        decode::{tests::load_stream, JsonDecode, XmlDecode},
        encode::{JsonEncode, XmlEncode},
    };

    use super::super::super::tests::{trim_json_str, trim_xml_str};

    #[tokio::test]
    async fn test_decode_json() {
        let mut stream = load_stream("./examples/audit_event.json");

        let actual = stream.json::<AuditEvent>().await.unwrap();
        let expected = test_audit_event();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_decode_xml() {
        let mut stream = load_stream("./examples/audit_event.xml");

        let actual = stream.xml::<AuditEvent>().await.unwrap();
        let expected = test_audit_event();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_encode_json() {
        let value = test_audit_event();

        let actual = (&value).json().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/audit_event.json").unwrap();

        assert_eq!(trim_json_str(&actual), trim_json_str(&expected));
    }

    #[tokio::test]
    async fn test_encode_xml() {
        let value = test_audit_event();

        let actual = (&value).xml().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/audit_event.xml").unwrap();

        assert_eq!(trim_xml_str(&actual), trim_xml_str(&expected));
    }

    pub fn test_audit_event() -> AuditEvent {
        AuditEvent {
            id: None,
            sub_type: SubType::Read,
            action: Action::Create,
            recorded: "2020-02-27T08:04:27.434+00:00".try_into().unwrap(),
            outcome: Outcome::Success,
            agent: Agent {
                type_: ParticipationRoleType::HumanUser,
                who: TelematikId::new("606358750"),
                name: "Praxis Dr. Müller".into(),
                requestor: false,
            },
            source: Source {
                observer: "Device/eRx-Fachdienst".into(),
            },
            entity: Entity {
                what: "Task/4711".into(),
                name: Kvnr::new("X123456789").unwrap(),
                description: "160.123.456.789.123.58".parse().unwrap(),
            },
        }
    }
}
