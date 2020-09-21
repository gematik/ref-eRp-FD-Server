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

use std::fmt::Display;
use std::io::Error as IoError;
use std::str::Utf8Error;

use actix_web::{
    error::{PayloadError, ResponseError},
    http::StatusCode,
    HttpResponse,
};
use openssl::error::ErrorStack as OpenSslError;
#[cfg(feature = "support-xml")]
use quick_xml::DeError as XmlError;
#[cfg(feature = "support-json")]
use serde_json::Error as JsonError;
use thiserror::Error;
use vau::Error as VauError;

use super::misc::AccessTokenError;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO Error: {0}")]
    IoError(IoError),

    #[error("OpenSSL Error: {0}")]
    OpenSslError(OpenSslError),

    #[error("VAU Error: {0}")]
    VauError(VauError),

    #[error("Unsupported Scheme {0}!")]
    UnsupportedScheme(String),
}

#[derive(Error, Debug)]
pub enum RequestError {
    #[error("Internal Error: {0}")]
    Internal(String),

    #[error("Unauthorized: {0}!")]
    Unauthorized(String),

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

    #[error("Access Token Error: {0}!")]
    AccessTokenError(AccessTokenError),

    #[error("Payload Error: {0}")]
    PayloadError(PayloadError),

    #[error("UTF-8 Error: {0:?}")]
    Utf8Error(Utf8Error),

    #[error("Header is missing: {0}!")]
    HeaderMissing(String),

    #[error("Header has invalid value: {0}!")]
    HeaderInvalid(String),

    #[error("Payload exceeds the configured limit: {0}!")]
    PayloadToLarge(usize),

    #[error("Bad Request: {0}")]
    BadRequest(String),

    #[error("The query argument '_format' is malformed: {0}!")]
    InvalidFormatArgument(String),

    #[error("Content-Type is not supported: {0}!")]
    ContentTypeNotSupported(String),

    #[error("None of the accepted content types of the client are not supported by the server!")]
    AcceptUnsupported,
}

impl From<IoError> for Error {
    fn from(v: IoError) -> Self {
        Error::IoError(v)
    }
}

impl From<OpenSslError> for Error {
    fn from(v: OpenSslError) -> Self {
        Error::OpenSslError(v)
    }
}

impl From<VauError> for Error {
    fn from(v: VauError) -> Self {
        Error::VauError(v)
    }
}

impl RequestError {
    pub fn internal<T: Display>(t: T) -> Self {
        Self::Internal(t.to_string())
    }

    pub fn header_invalid<T: Display>(t: T) -> Self {
        Self::HeaderInvalid(t.to_string())
    }

    pub fn header_missing<T: Display>(t: T) -> Self {
        Self::HeaderMissing(t.to_string())
    }
}

impl ResponseError for RequestError {
    fn error_response(&self) -> HttpResponse {
        let mut res = HttpResponse::InternalServerError();

        match self {
            Self::Internal(_) => res.status(StatusCode::INTERNAL_SERVER_ERROR),

            Self::Unauthorized(_) => res.status(StatusCode::UNAUTHORIZED),

            #[cfg(feature = "support-xml")]
            Self::DeserializeXml(_) => res.status(StatusCode::BAD_REQUEST),

            #[cfg(feature = "support-json")]
            Self::DeserializeJson(_) => res.status(StatusCode::BAD_REQUEST),

            #[cfg(feature = "support-xml")]
            Self::SerializeXml(_) => res.status(StatusCode::INTERNAL_SERVER_ERROR),

            #[cfg(feature = "support-json")]
            Self::SerializeJson(_) => res.status(StatusCode::INTERNAL_SERVER_ERROR),

            #[cfg(feature = "interface-supplier")]
            Self::AccessTokenError(AccessTokenError::Missing) => res
                .status(StatusCode::UNAUTHORIZED)
                .header(
                    "WWW-Authenticate",
                    "Bearer realm='prescriptionserver.telematik',scope='openid profile prescriptionservice.lei'"),

            #[cfg(all(feature = "interface-patient", not(feature = "interface-supplier")))]
            Self::AccessTokenError(AccessTokenError::Missing) => res
                .status(StatusCode::UNAUTHORIZED)
                .header(
                    "WWW-Authenticate",
                    "Bearer realm='prescriptionserver.telematik',scope='openid profile prescriptionservice.vers'"),

            Self::AccessTokenError(AccessTokenError::NoKvnr)
            | Self::AccessTokenError(AccessTokenError::NoTelematikId) =>
                res.status(StatusCode::BAD_REQUEST),

            Self::AccessTokenError(_) => res.status(StatusCode::UNAUTHORIZED).header(
                "WWW-Authenticate",
                "Bearer realm='prescriptionserver.telematik', error='invalACCESS_TOKEN'",
            ),

            Self::PayloadError(_)
            | Self::Utf8Error(_)
            | Self::HeaderMissing(_)
            | Self::HeaderInvalid(_)
            | Self::PayloadToLarge(_)
            | Self::BadRequest(_)
            | Self::InvalidFormatArgument(_)
            | Self::ContentTypeNotSupported(_)
            | Self::AcceptUnsupported => res.status(StatusCode::BAD_REQUEST),
        };

        res.header("Content-Type", "text/plain; charset=utf-8")
            .body(format!("{}", self))
    }
}

impl From<AccessTokenError> for RequestError {
    fn from(err: AccessTokenError) -> Self {
        Self::AccessTokenError(err)
    }
}

impl From<PayloadError> for RequestError {
    fn from(err: PayloadError) -> Self {
        Self::PayloadError(err)
    }
}

impl From<Utf8Error> for RequestError {
    fn from(err: Utf8Error) -> Self {
        Self::Utf8Error(err)
    }
}
