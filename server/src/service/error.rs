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

use std::fmt::{Display, Formatter, Result as FmtResult};

use actix_web::{
    dev::HttpResponseBuilder,
    error::{PayloadError, ResponseError},
    http::StatusCode,
    HttpRequest, HttpResponse,
};
use openssl::error::ErrorStack as OpenSslError;
use resources::{
    audit_event::Outcome,
    operation_outcome::{Issue, IssueType, OperationOutcome, Severity},
};
use thiserror::Error;

use crate::{
    fhir::{
        decode::{DecodeError, JsonError as JsonDecodeError, XmlError as XmlDecodeError},
        encode::{EncodeError, JsonError as JsonEncodeError, XmlError as XmlEncodeError},
    },
    tasks::tsl::Error as TslError,
};

use super::{
    header::Accept,
    misc::{AccessTokenError, DataType},
    routes::{
        audit_event::Error as AuditEventError,
        capabilty_statement::Error as CapabiltyStatementError,
        communication::Error as CommunicationError,
        medication_dispense::Error as MedicationDispenseError, task::Error as TaskError,
    },
};

/* TypedRequestError */

#[derive(Debug)]
pub struct TypedRequestError {
    pub error: RequestError,
    pub data_type: DataType,
}

impl Display for TypedRequestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        self.error.fmt(f)
    }
}

impl ResponseError for TypedRequestError {
    fn error_response(&self) -> HttpResponse {
        use RequestError as E;

        let res = ResponseBuilder::new();
        let mut res = match &self.error {
            E::OpenSslError(_) => res.status(StatusCode::BAD_REQUEST),
            E::AccessTokenError(err) => match err {
                #[cfg(all(feature = "interface-supplier", not(feature = "interface-patient")))]
                AccessTokenError::Missing => res
                    .status(StatusCode::UNAUTHORIZED)
                    .code(IssueType::SecurityUnknown)
                    .header(
                        "WWW-Authenticate",
                        "Bearer realm='prescriptionserver.telematik',scope='openid profile prescriptionservice.lei'"),
                #[cfg(all(feature = "interface-patient", not(feature = "interface-supplier")))]
                AccessTokenError::Missing => res
                    .status(StatusCode::UNAUTHORIZED)
                    .code(IssueType::SecurityUnknown)
                    .header(
                        "WWW-Authenticate",
                        "Bearer realm='prescriptionserver.telematik',scope='openid profile prescriptionservice.vers'"),
                AccessTokenError::NoKvnr => res.status(StatusCode::BAD_REQUEST).code(IssueType::SecurityUnknown),
                AccessTokenError::NoTelematikId => res.status(StatusCode::BAD_REQUEST).code(IssueType::SecurityUnknown),
                AccessTokenError::InvalidProfession => res.status(StatusCode::FORBIDDEN).code(IssueType::SecurityForbidden),
                _ => res.status(StatusCode::UNAUTHORIZED).code(IssueType::SecurityUnknown).header(
                    "WWW-Authenticate",
                    "Bearer realm='prescriptionserver.telematik', error='invalACCESS_TOKEN'",
                ),
            },
            E::DecodeXml(err) => res.status(StatusCode::BAD_REQUEST).expression_opt(err.path()),
            E::DecodeJson(err) => res.status(StatusCode::BAD_REQUEST).expression_opt(err.path()),
            E::EncodeXml(_) => res.status(StatusCode::INTERNAL_SERVER_ERROR).severity(Severity::Fatal),
            E::EncodeJson(_) => res.status(StatusCode::INTERNAL_SERVER_ERROR).severity(Severity::Fatal),
            E::CapabiltyStatementError(err) => match err {
                CapabiltyStatementError::InvalidFormat(_) => res.status(StatusCode::BAD_REQUEST),
                CapabiltyStatementError::UnsupportedFormat => res.status(StatusCode::BAD_REQUEST),
            },
            E::AuditEventError(err) => match err {
                AuditEventError::NotFound(_) => res.status(StatusCode::NOT_FOUND).code(IssueType::ProcessingNotFound),
            },
            E::CommunicationError(err) => match err {
                CommunicationError::ContentSizeExceeded => res.status(StatusCode::BAD_REQUEST).code(IssueType::ProcessingTooLong),
                CommunicationError::MissingFieldBasedOn => res.status(StatusCode::BAD_REQUEST).code(IssueType::InvalidRequired).expression("/Communication/basedOn".into()),
                CommunicationError::SenderEqualRecipient => res.status(StatusCode::BAD_REQUEST),
                CommunicationError::InvalidSender => res.status(StatusCode::BAD_REQUEST),
                CommunicationError::UnknownTask(_) => res.status(StatusCode::BAD_REQUEST),
                CommunicationError::UnauthorizedTaskAccess => res.status(StatusCode::BAD_REQUEST),
                CommunicationError::InvalidTaskStatus => res.status(StatusCode::BAD_REQUEST),
                CommunicationError::InvalidTaskUri(_) => res.status(StatusCode::BAD_REQUEST),
                CommunicationError::NotFound(_) => res.status(StatusCode::NOT_FOUND).code(IssueType::ProcessingNotFound),
                CommunicationError::Unauthorized(_) => res.status(StatusCode::UNAUTHORIZED),
            },
            E::MedicationDispenseError(err) => match err {
                MedicationDispenseError::NotFound(_) => res.status(StatusCode::NOT_FOUND).code(IssueType::ProcessingNotFound),
                MedicationDispenseError::Forbidden(_) => res.status(StatusCode::FORBIDDEN).code(IssueType::SecurityForbidden),
            },
            E::TaskError(err) => match err {
                TaskError::SignedError(_) => res.status(StatusCode::INTERNAL_SERVER_ERROR).severity(Severity::Fatal),
                TaskError::NotFound(_) => res.status(StatusCode::NOT_FOUND).code(IssueType::ProcessingNotFound),
                TaskError::Forbidden(_) => res.status(StatusCode::FORBIDDEN).code(IssueType::SecurityForbidden),
                TaskError::Conflict(_) => res.status(StatusCode::CONFLICT).code(IssueType::ProcessingConflict),
                TaskError::Gone(_) => res.status(StatusCode::GONE).code(IssueType::ProcessingConflict),
                TaskError::EPrescriptionMissing => res.status(StatusCode::BAD_REQUEST),
                TaskError::EPrescriptionMismatch => res.status(StatusCode::BAD_REQUEST),
                TaskError::EPrescriptionNotFound(_) => res.status(StatusCode::INTERNAL_SERVER_ERROR).severity(Severity::Fatal),
                TaskError::EPrescriptionAlreadyRegistered(_) => res.status(StatusCode::BAD_REQUEST),
                TaskError::PatientReceiptNotFound(_) => res.status(StatusCode::INTERNAL_SERVER_ERROR).severity(Severity::Fatal),
                TaskError::ErxReceiptNotFound(_) => res.status(StatusCode::INTERNAL_SERVER_ERROR).severity(Severity::Fatal),
                TaskError::KvnrMissing => res.status(StatusCode::BAD_REQUEST).code(IssueType::SecurityUnknown),
                TaskError::KvnrInvalid => res.status(StatusCode::BAD_REQUEST).code(IssueType::SecurityUnknown),
                TaskError::SubjectMissing => res.status(StatusCode::BAD_REQUEST),
                TaskError::SubjectMismatch => res.status(StatusCode::BAD_REQUEST),
                TaskError::PerformerMismatch => res.status(StatusCode::BAD_REQUEST),
                TaskError::AcceptTimestampMissing => res.status(StatusCode::INTERNAL_SERVER_ERROR).severity(Severity::Fatal),
                TaskError::InvalidStatus => res.status(StatusCode::BAD_REQUEST),
                TaskError::InvalidUrl(_) => res.status(StatusCode::BAD_REQUEST),
                TaskError::GeneratePrescriptionId => res.status(StatusCode::SERVICE_UNAVAILABLE).severity(Severity::Error),
                TaskError::AuditEventAgentInvalid => res.status(StatusCode::BAD_REQUEST),
            },
            E::CmsContainerError(_) => res.status(StatusCode::BAD_REQUEST),
            E::NotFound(_) => res.status(StatusCode::NOT_FOUND).code(IssueType::ProcessingNotFound),
            E::HeaderInvalid(_) => res.status(StatusCode::BAD_REQUEST),
            E::HeaderMissing(_) => res.status(StatusCode::BAD_REQUEST),
            E::QueryInvalid(_) => res.status(StatusCode::BAD_REQUEST),
            E::ContentTypeNotSupported => res.status(StatusCode::BAD_REQUEST),
            E::AcceptUnsupported => res.status(StatusCode::BAD_REQUEST),
        };

        if res.details.is_none() {
            res.details = Some(self.error.to_string());
        }

        res.data_type(self.data_type).build()
    }
}

/* RequestError */

#[derive(Error, Debug)]
pub enum RequestError {
    #[error("OpenSSL Error: {0}")]
    OpenSslError(OpenSslError),

    #[error("Access Token Error: {0}")]
    AccessTokenError(AccessTokenError),

    #[error("Error while decoding XML: {0}")]
    DecodeXml(DecodeError<XmlDecodeError<PayloadError>>),

    #[error("Error while decoding JSON: {0}")]
    DecodeJson(DecodeError<JsonDecodeError<PayloadError>>),

    #[error("Error while encoding XML: {0}")]
    EncodeXml(EncodeError<XmlEncodeError>),

    #[error("Error while encoding JSON: {0}")]
    EncodeJson(EncodeError<JsonEncodeError>),

    #[error("Capabilty Statement Error {0}")]
    CapabiltyStatementError(CapabiltyStatementError),

    #[error("Audit Event Resource Error {0}")]
    AuditEventError(AuditEventError),

    #[error("Communication Resource Error {0}")]
    CommunicationError(CommunicationError),

    #[error("Medication Dispense Resource Error: {0}")]
    MedicationDispenseError(MedicationDispenseError),

    #[error("Task Resource Error: {0}")]
    TaskError(TaskError),

    #[error("Unable to verify CMS container: {0}")]
    CmsContainerError(TslError),

    #[error("Not Found: {0}!")]
    NotFound(String),

    #[error("Header Invalid: {0}!")]
    HeaderInvalid(String),

    #[error("Header Missing: {0}!")]
    HeaderMissing(String),

    #[error("Invalid Query: {0}!")]
    QueryInvalid(String),

    #[error("Content Type not Supported!")]
    ContentTypeNotSupported,

    #[error("Accept Value is not Supported!")]
    AcceptUnsupported,
}

impl RequestError {
    #[cfg(feature = "support-json")]
    #[allow(clippy::wrong_self_convention)]
    pub fn as_json(self) -> TypedRequestError {
        TypedRequestError {
            error: self,
            data_type: DataType::Json,
        }
    }

    #[cfg(feature = "support-xml")]
    #[allow(clippy::wrong_self_convention)]
    pub fn as_xml(self) -> TypedRequestError {
        TypedRequestError {
            error: self,
            data_type: DataType::Xml,
        }
    }

    pub fn with_type(self, data_type: DataType) -> TypedRequestError {
        TypedRequestError {
            error: self,
            data_type,
        }
    }

    pub fn with_type_from<T: DataTypeFrom>(self, from: &T) -> TypedRequestError {
        TypedRequestError {
            error: self,
            data_type: from.data_type(),
        }
    }

    pub fn with_type_default(self) -> TypedRequestError {
        TypedRequestError {
            error: self,
            data_type: DataType::default(),
        }
    }
}

impl<T> From<T> for RequestError
where
    T: AsReqErr,
{
    fn from(err: T) -> RequestError {
        err.as_req_err()
    }
}

/* DataTypeFrom */

pub trait DataTypeFrom {
    fn data_type(&self) -> DataType;
}

impl DataTypeFrom for Accept {
    fn data_type(&self) -> DataType {
        DataType::from_accept(&self).unwrap_or_default()
    }
}

impl DataTypeFrom for HttpRequest {
    fn data_type(&self) -> DataType {
        match Accept::from_headers(self.headers()) {
            Ok(accept) => DataType::from_accept(&accept).unwrap_or_default(),
            Err(_) => DataType::default(),
        }
    }
}

/* TypedRequestResult */

pub trait TypedRequestResult: Sized {
    type Value;

    #[cfg(feature = "support-json")]
    fn err_as_json(self) -> Result<Self::Value, TypedRequestError>;

    #[cfg(feature = "support-xml")]
    fn err_as_xml(self) -> Result<Self::Value, TypedRequestError>;

    fn err_with_type(self, data_type: DataType) -> Result<Self::Value, TypedRequestError>;

    fn err_with_type_from<F: DataTypeFrom>(
        self,
        from: &F,
    ) -> Result<Self::Value, TypedRequestError>;

    fn err_with_type_default(self) -> Result<Self::Value, TypedRequestError> {
        self.err_with_type(DataType::default())
    }
}

impl<T> TypedRequestResult for Result<T, RequestError> {
    type Value = T;

    #[cfg(feature = "support-json")]
    fn err_as_json(self) -> Result<T, TypedRequestError> {
        self.map_err(RequestError::as_json)
    }

    #[cfg(feature = "support-xml")]
    fn err_as_xml(self) -> Result<T, TypedRequestError> {
        self.map_err(RequestError::as_xml)
    }

    fn err_with_type(self, data_type: DataType) -> Result<T, TypedRequestError> {
        self.map_err(|err| err.with_type(data_type))
    }

    fn err_with_type_from<F: DataTypeFrom>(self, from: &F) -> Result<T, TypedRequestError> {
        self.map_err(|err| err.with_type_from(from))
    }
}

/* AsReqErrResult */

pub trait AsReqErrResult {
    type Value;

    fn as_req_err(self) -> Result<Self::Value, RequestError>;
}

impl<T, E> AsReqErrResult for Result<T, E>
where
    E: AsReqErr,
{
    type Value = T;

    fn as_req_err(self) -> Result<T, RequestError> {
        self.map_err(AsReqErr::as_req_err)
    }
}

/* AsReqErr */

pub trait AsReqErr {
    fn as_req_err(self) -> RequestError;
}

impl AsReqErr for OpenSslError {
    fn as_req_err(self) -> RequestError {
        RequestError::OpenSslError(self)
    }
}

impl AsReqErr for AccessTokenError {
    fn as_req_err(self) -> RequestError {
        RequestError::AccessTokenError(self)
    }
}

impl AsReqErr for DecodeError<XmlDecodeError<PayloadError>> {
    fn as_req_err(self) -> RequestError {
        RequestError::DecodeXml(self)
    }
}

impl AsReqErr for DecodeError<JsonDecodeError<PayloadError>> {
    fn as_req_err(self) -> RequestError {
        RequestError::DecodeJson(self)
    }
}

impl AsReqErr for EncodeError<XmlEncodeError> {
    fn as_req_err(self) -> RequestError {
        RequestError::EncodeXml(self)
    }
}

impl AsReqErr for EncodeError<JsonEncodeError> {
    fn as_req_err(self) -> RequestError {
        RequestError::EncodeJson(self)
    }
}

impl AsReqErr for CapabiltyStatementError {
    fn as_req_err(self) -> RequestError {
        RequestError::CapabiltyStatementError(self)
    }
}

impl AsReqErr for AuditEventError {
    fn as_req_err(self) -> RequestError {
        RequestError::AuditEventError(self)
    }
}

impl AsReqErr for CommunicationError {
    fn as_req_err(self) -> RequestError {
        RequestError::CommunicationError(self)
    }
}

impl AsReqErr for MedicationDispenseError {
    fn as_req_err(self) -> RequestError {
        RequestError::MedicationDispenseError(self)
    }
}

impl AsReqErr for TaskError {
    fn as_req_err(self) -> RequestError {
        RequestError::TaskError(self)
    }
}

/* AsAuditEventOutcome */

pub trait AsAuditEventOutcome {
    fn as_outcome(&self) -> Outcome {
        Outcome::MinorFailure
    }
}

impl AsAuditEventOutcome for TypedRequestError {}

/* ResponseBuilder */

struct ResponseBuilder {
    code: Option<IssueType>,
    status: Option<StatusCode>,
    details: Option<String>,
    severity: Option<Severity>,
    data_type: Option<DataType>,
    header: Vec<(&'static str, &'static str)>,
    expression: Vec<String>,
}

#[allow(dead_code)]
impl ResponseBuilder {
    fn new() -> Self {
        Self {
            code: None,
            status: None,
            details: None,
            severity: None,
            data_type: None,
            header: Vec::new(),
            expression: Vec::new(),
        }
    }

    fn code(mut self, code: IssueType) -> Self {
        self.code = Some(code);

        self
    }

    fn status(mut self, status: StatusCode) -> Self {
        self.status = Some(status);

        self
    }

    fn details(mut self, details: String) -> Self {
        self.details = Some(details);

        self
    }

    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = Some(severity);

        self
    }

    pub fn data_type(mut self, data_type: DataType) -> Self {
        self.data_type = Some(data_type);

        self
    }

    pub fn header(mut self, key: &'static str, value: &'static str) -> Self {
        self.header.push((key, value));

        self
    }

    pub fn expression(mut self, expression: String) -> Self {
        self.expression.push(expression);

        self
    }

    pub fn expression_opt(mut self, expression: Option<&String>) -> Self {
        if let Some(expression) = expression {
            self.expression.push(expression.clone());
        }

        self
    }

    pub fn build(self) -> HttpResponse {
        let status = self.status.unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let data_type = self.data_type.unwrap_or_default().replace_any_default();
        let severity = if status == StatusCode::INTERNAL_SERVER_ERROR {
            Severity::Fatal
        } else {
            Severity::Error
        };

        let mut res = HttpResponseBuilder::new(status);
        for (name, value) in self.header {
            res.header(name, value);
        }

        let out = OperationOutcome {
            issue: vec![Issue {
                severity: self.severity.unwrap_or(severity),
                code: self.code.unwrap_or(IssueType::Invalid),
                details: self.details,
                diagnostics: None,
                expression: Vec::new(),
            }],
        };

        #[allow(unreachable_patterns)]
        match data_type {
            #[cfg(feature = "support-xml")]
            DataType::Xml | DataType::Any | DataType::Unknown => {
                use crate::fhir::encode::XmlEncode;

                let xml = out.xml().unwrap();

                res.content_type(DataType::Xml.as_mime().to_string())
                    .body(xml)
            }

            #[cfg(feature = "support-json")]
            DataType::Json | DataType::Any | DataType::Unknown => {
                use crate::fhir::encode::JsonEncode;

                let json = out.json().unwrap();

                res.content_type(DataType::Json.as_mime().to_string())
                    .body(json)
            }
        }
    }
}
