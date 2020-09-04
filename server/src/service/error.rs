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

use std::io::Error as IoError;
use std::str::Utf8Error;

use actix_web::{
    error::{PayloadError, ResponseError},
    http::header::HeaderName,
    http::StatusCode,
};
use mime::Mime;
use openssl::error::ErrorStack as OpenSslError;
#[cfg(feature = "support-xml")]
use quick_xml::DeError as XmlError;
use resources::types::Profession;
#[cfg(feature = "support-json")]
use serde_json::Error as JsonError;
use thiserror::Error;

use super::idp_client::Error as IdpClientError;

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum Error {
    #[error("Expected Header was not set: {0}!")]
    ExpectHeader(HeaderName),

    #[error("Invalid Header: {0}!")]
    InvalidHeader(HeaderName),

    #[error("Content-Type is not supported: {0}!")]
    ContentTypeNotSupported(Mime),

    #[error("The requested accept types are not supported!")]
    AcceptUnsupported,

    #[error("KVNR is missing in the request data!")]
    MissingKvnr,

    #[error("Invalid Task Status!")]
    InvalidTaskStatus,

    #[error("Payload exceeds the configured limit: {0}!")]
    PayloadOverflow(usize),

    #[error("Payload Error: {0}")]
    PayloadError(PayloadError),

    #[error("Invalid profession: {0:?}!")]
    InvalidProfession(Profession),

    #[error("The query argument '_format' is malformed: {0}!")]
    InvalidFormatArgument(String),

    #[cfg(feature = "support-xml")]
    #[error("Error while reading XML: {0}!")]
    DeserializeXml(XmlError),

    #[cfg(feature = "support-json")]
    #[error("Error while reading JSON: {0}!")]
    DeserializeJson(JsonError),

    #[cfg(feature = "support-xml")]
    #[error("Error while writing XML: {0}!")]
    SerializeXml(XmlError),

    #[cfg(feature = "support-json")]
    #[error("Error while writing JSON: {0}!")]
    SerializeJson(JsonError),

    #[error("IDP Client Error: {0:?}")]
    IdpClientError(IdpClientError),

    #[error("UTF-8 Error: {0:?}")]
    Utf8Error(Utf8Error),

    #[error("Error in OpenSSL Library: {0}")]
    OpenSslError(OpenSslError),

    #[error("IO Error: {0}")]
    IoError(IoError),

    #[error("Internal Server Error!")]
    Internal,
}

#[cfg(feature = "support-xml")]
impl From<XmlError> for Error {
    fn from(v: XmlError) -> Self {
        Error::DeserializeXml(v)
    }
}

#[cfg(feature = "support-json")]
impl From<JsonError> for Error {
    fn from(v: JsonError) -> Self {
        Error::DeserializeJson(v)
    }
}

impl From<PayloadError> for Error {
    fn from(v: PayloadError) -> Self {
        Error::PayloadError(v)
    }
}

impl From<Utf8Error> for Error {
    fn from(v: Utf8Error) -> Self {
        Error::Utf8Error(v)
    }
}

impl From<OpenSslError> for Error {
    fn from(v: OpenSslError) -> Self {
        Error::OpenSslError(v)
    }
}

impl From<IoError> for Error {
    fn from(v: IoError) -> Self {
        Error::IoError(v)
    }
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            #[cfg(feature = "support-xml")]
            Error::SerializeXml(_) => StatusCode::INTERNAL_SERVER_ERROR,
            #[cfg(feature = "support-json")]
            Error::SerializeJson(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::PayloadOverflow(_) => StatusCode::PAYLOAD_TOO_LARGE,
            Error::Internal | Error::IdpClientError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::BAD_REQUEST,
        }
    }
}
