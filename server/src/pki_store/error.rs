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

use std::io::Error as IoError;

use base64::DecodeError as Base64Error;
use chrono::ParseError as ChronoError;
use miscellaneous::jwt::Error as JwtError;
use openssl::{error::ErrorStack as OpenSslError, ocsp::OcspResponseStatus};
use quick_xml::DeError as XmlError;
use reqwest::{Error as ReqwestError, StatusCode};
use thiserror::Error;
use url::ParseError;

#[derive(Debug, Error)]
#[allow(clippy::enum_variant_names)]
pub enum Error {
    #[error("IO Error: {0}")]
    IoError(IoError),

    #[error("Url Parse Error: {0}")]
    ParseError(ParseError),

    #[error("Reqwest Error: {0}")]
    ReqwestError(ReqwestError),

    #[error("XML Error: {0}")]
    XmlError(XmlError),

    #[error("Chrono Error: {0}")]
    ChronoError(ChronoError),

    #[error("Base64 Error: {0}")]
    Base64Error(Base64Error),

    #[error("OpenSSL Error: {0}")]
    OpenSslError(OpenSslError),

    #[error("JWT Error: {0}")]
    JwtError(JwtError),

    #[error("Invalid Response ({0} - {1})")]
    InvalidResponse(StatusCode, String),

    #[error("Invalid URL: {0}!")]
    InvalidUrl(String),

    #[error("Invalid OCSP Status: {0:?}!")]
    InvalidOcspStatus(OcspResponseStatus),

    #[error("Unable to find Signer Certificate!")]
    UnknownSignerCert,

    #[error("Unable to find Issuer Certificate!")]
    UnknownIssuerCert,

    #[error("Missing signing time in bundle CMS signature!")]
    UnknownSigningTime,

    #[error("Certificate is not valid yet!")]
    CertNotValidYet,

    #[error("Certificate is not valid anymore!")]
    CertNotValidAnymore,

    #[error("Empty Certificate Key!")]
    EmptyCertKey,

    #[error("Missing or empty service supply points!")]
    MissingServiceSupplyPoints,

    #[error("Fetching OCSP Response failed!")]
    FetchingOcspResponseFailed,
}

impl From<IoError> for Error {
    fn from(err: IoError) -> Error {
        Error::IoError(err)
    }
}

impl From<ParseError> for Error {
    fn from(err: ParseError) -> Error {
        Error::ParseError(err)
    }
}

impl From<ReqwestError> for Error {
    fn from(err: ReqwestError) -> Error {
        Error::ReqwestError(err)
    }
}

impl From<XmlError> for Error {
    fn from(err: XmlError) -> Self {
        Self::XmlError(err)
    }
}

impl From<ChronoError> for Error {
    fn from(err: ChronoError) -> Self {
        Self::ChronoError(err)
    }
}

impl From<Base64Error> for Error {
    fn from(err: Base64Error) -> Self {
        Self::Base64Error(err)
    }
}

impl From<OpenSslError> for Error {
    fn from(err: OpenSslError) -> Self {
        Self::OpenSslError(err)
    }
}

impl From<JwtError> for Error {
    fn from(err: JwtError) -> Self {
        Self::JwtError(err)
    }
}
