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
use resources::operation_outcome::{Issue, IssueType, OperationOutcome, Severity};

use crate::fhir::{
    decode::{decode_any, DataStream, Decode, DecodeError, DecodeStream, Fields},
    encode::{encode_any, DataStorage, Encode, EncodeError, EncodeStream},
};

use super::{
    meta::Meta,
    primitives::{
        decode_code, decode_codeable_concept, encode_code, encode_codeable_concept, CodeEx,
    },
};

/* Decode */

#[async_trait(?Send)]
impl Decode for OperationOutcome {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["meta", "issue"]);

        stream.root("OperationOutcome").await?;

        let meta = stream.decode::<Meta, _>(&mut fields, decode_any).await?;
        let issue = stream.decode_vec(&mut fields, decode_any).await?;

        stream.end().await?;

        if !meta.profiles.iter().any(|p| icase_eq(p, PROFILE)) {
            return Err(DecodeError::InvalidProfile {
                actual: meta.profiles,
                expected: vec![PROFILE.into()],
            });
        }

        Ok(OperationOutcome { issue })
    }
}

#[async_trait(?Send)]
impl Decode for Issue {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["severity", "code", "details", "diagnostics", "expression"]);

        stream.element().await?;

        let severity = stream.decode(&mut fields, decode_code).await?;
        let code = stream.decode(&mut fields, decode_code).await?;
        let details = stream
            .decode_opt(&mut fields, decode_codeable_concept)
            .await?;
        let diagnostics = stream.decode_opt(&mut fields, decode_any).await?;
        let expression = stream.decode_vec(&mut fields, decode_any).await?;

        stream.end().await?;

        Ok(Issue {
            severity,
            code,
            details,
            diagnostics,
            expression,
        })
    }
}

/* Encode */

impl Encode for &OperationOutcome {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let meta = Meta {
            profiles: vec![PROFILE.into()],
            ..Default::default()
        };

        stream
            .root("OperationOutcome")?
            .encode("meta", meta, encode_any)?
            .encode_vec("issue", &self.issue, encode_any)?
            .end()?;

        Ok(())
    }
}

impl Encode for &Issue {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode("severity", &self.severity, encode_code)?
            .encode("code", &self.code, encode_code)?
            .encode_opt("details", &self.details, encode_codeable_concept)?
            .encode_opt("diagnostics", &self.diagnostics, encode_any)?
            .encode_vec("expression", &self.expression, encode_any)?
            .end()?;

        Ok(())
    }
}

/* Misc */

impl CodeEx for Severity {
    fn from_parts(value: String) -> Result<Self, String> {
        match value.as_str() {
            "fatal" => Ok(Self::Fatal),
            "error" => Ok(Self::Error),
            "warning" => Ok(Self::Warning),
            "information" => Ok(Self::Information),
            _ => Err(value),
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Fatal => "fatal",
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Information => "information",
        }
    }
}

impl CodeEx for IssueType {
    fn from_parts(value: String) -> Result<Self, String> {
        match value.as_str() {
            "invalid" => Ok(Self::Invalid),
            "structure" => Ok(Self::InvalidStructure),
            "required" => Ok(Self::InvalidRequired),
            "value" => Ok(Self::InvalidValue),
            "invariant" => Ok(Self::InvalidInvariant),
            "security" => Ok(Self::Security),
            "login" => Ok(Self::SecurityLogin),
            "unknown" => Ok(Self::SecurityUnknown),
            "expired" => Ok(Self::SecurityExpired),
            "forbidden" => Ok(Self::SecurityForbidden),
            "suppressed" => Ok(Self::SecuritySuppressed),
            "processing" => Ok(Self::Processing),
            "not-supported" => Ok(Self::ProcessingNotSupported),
            "duplicate" => Ok(Self::ProcessingDuplicate),
            "multiple-matches" => Ok(Self::ProcessingMultipleMatches),
            "not-found" => Ok(Self::ProcessingNotFound),
            "deleted" => Ok(Self::ProcessingDeleted),
            "too-long" => Ok(Self::ProcessingTooLong),
            "code-invalid" => Ok(Self::ProcessingCodeInvalid),
            "extension" => Ok(Self::ProcessingExtension),
            "too-costly" => Ok(Self::ProcessingTooCostly),
            "business-rule" => Ok(Self::ProcessingBusinessRule),
            "conflict" => Ok(Self::ProcessingConflict),
            "transient" => Ok(Self::Transient),
            "lock-error" => Ok(Self::TransientLockError),
            "no-store" => Ok(Self::TransientNoStore),
            "exception" => Ok(Self::TransientException),
            "timeout" => Ok(Self::TransientTimeout),
            "incomplete" => Ok(Self::TransientIncomplete),
            "throttled" => Ok(Self::TransientThrottled),
            "informational" => Ok(Self::Informational),
            _ => Err(value),
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Invalid => "invalid",
            Self::InvalidStructure => "structure",
            Self::InvalidRequired => "required",
            Self::InvalidValue => "value",
            Self::InvalidInvariant => "invariant",
            Self::Security => "security",
            Self::SecurityLogin => "login",
            Self::SecurityUnknown => "unknown",
            Self::SecurityExpired => "expired",
            Self::SecurityForbidden => "forbidden",
            Self::SecuritySuppressed => "suppressed",
            Self::Processing => "processing",
            Self::ProcessingNotSupported => "not-supported",
            Self::ProcessingDuplicate => "duplicate",
            Self::ProcessingMultipleMatches => "multiple-matches",
            Self::ProcessingNotFound => "not-found",
            Self::ProcessingDeleted => "deleted",
            Self::ProcessingTooLong => "too-long",
            Self::ProcessingCodeInvalid => "code-invalid",
            Self::ProcessingExtension => "extension",
            Self::ProcessingTooCostly => "too-costly",
            Self::ProcessingBusinessRule => "business-rule",
            Self::ProcessingConflict => "conflict",
            Self::Transient => "transient",
            Self::TransientLockError => "lock-error",
            Self::TransientNoStore => "no-store",
            Self::TransientException => "exception",
            Self::TransientTimeout => "timeout",
            Self::TransientIncomplete => "incomplete",
            Self::TransientThrottled => "throttled",
            Self::Informational => "informational",
        }
    }
}

const PROFILE: &str = "http://hl7.org/fhir/StructureDefinition/OperationOutcome";

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::fs::read_to_string;
    use std::str::from_utf8;

    use crate::fhir::{
        decode::{tests::load_stream, JsonDecode, XmlDecode},
        encode::{JsonEncode, XmlEncode},
    };

    use super::super::super::tests::{trim_json_str, trim_xml_str};

    #[tokio::test]
    async fn test_decode_json() {
        let mut stream = load_stream("./examples/operation_outcome.json");

        let actual = stream.json::<OperationOutcome>().await.unwrap();
        let expected = test_operation_outcome();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_decode_xml() {
        let mut stream = load_stream("./examples/operation_outcome.xml");

        let actual = stream.xml::<OperationOutcome>().await.unwrap();
        let expected = test_operation_outcome();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_encode_json() {
        let value = test_operation_outcome();

        let actual = (&value).json().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/operation_outcome.json").unwrap();

        assert_eq!(trim_json_str(&actual), trim_json_str(&expected));
    }

    #[tokio::test]
    async fn test_encode_xml() {
        let value = test_operation_outcome();

        let actual = (&value).xml().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/operation_outcome.xml").unwrap();

        assert_eq!(trim_xml_str(&actual), trim_xml_str(&expected));
    }

    pub fn test_operation_outcome() -> OperationOutcome {
        OperationOutcome {
            issue: vec![Issue {
                severity: Severity::Error,
                code: IssueType::ProcessingCodeInvalid,
                details: Some("The code 'W' is not known and not legal in this context".into()),
                diagnostics: Some(
                    "Acme.Interop.FHIRProcessors.Patient.processGender line 2453".into(),
                ),
                expression: vec!["Patient.gender".into()],
            }],
        }
    }
}
