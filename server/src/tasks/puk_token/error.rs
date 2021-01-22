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

use miscellaneous::jwt::Error as JwtError;
use openssl::error::ErrorStack as OpenSslError;
use reqwest::{Error as ReqwestError, StatusCode};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO Error: {0}")]
    IoError(IoError),

    #[error("Reqwest Error: {0}")]
    ReqwestError(ReqwestError),

    #[error("Open SSL Error: {0}")]
    OpenSslError(OpenSslError),

    #[error("JWT Error: {0}")]
    JwtError(JwtError),

    #[error("Unsupported URL Scheme: {0}")]
    UnsupportedScheme(String),

    #[error("Fetch failed: {0} - {1}")]
    FetchFailed(StatusCode, String),

    #[error("Missing Certificate in JWKS!")]
    MissingCert,
}

impl From<IoError> for Error {
    fn from(err: IoError) -> Self {
        Self::IoError(err)
    }
}

impl From<ReqwestError> for Error {
    fn from(err: ReqwestError) -> Self {
        Self::ReqwestError(err)
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
