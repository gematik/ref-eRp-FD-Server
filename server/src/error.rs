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

use log::SetLoggerError;
use log4rs::config::Errors as Log4RsError;
use openssl::error::ErrorStack as OpenSslError;
use serde_json::Error as JsonError;
use thiserror::Error;
use vau::Error as VauError;

use crate::{pki_store::Error as PkiError, state::HistoryError};

#[derive(Error, Debug)]
pub enum Error {
    #[error("Generic Error: {0}")]
    Generic(String),

    #[error("IO Error: {0}")]
    IoError(IoError),

    #[error("VAU Error: {0}")]
    VauError(VauError),

    #[error("Json Error: {0}")]
    JsonError(JsonError),

    #[error("OpenSSL Error: {0}")]
    OpenSslError(OpenSslError),

    #[error("Unable to set logger: {0}")]
    SetLoggerError(SetLoggerError),

    #[error("Unable to setup log4rs: {0}")]
    Log4RsError(Log4RsError),

    #[error("History Error: {0}")]
    HistoryError(HistoryError),

    #[error("PkiError: {0}")]
    PkiError(PkiError),
}

impl From<String> for Error {
    fn from(v: String) -> Self {
        Self::Generic(v)
    }
}

impl From<IoError> for Error {
    fn from(v: IoError) -> Self {
        Self::IoError(v)
    }
}

impl From<VauError> for Error {
    fn from(v: VauError) -> Self {
        Self::VauError(v)
    }
}

impl From<JsonError> for Error {
    fn from(v: JsonError) -> Self {
        Self::JsonError(v)
    }
}

impl From<OpenSslError> for Error {
    fn from(v: OpenSslError) -> Self {
        Self::OpenSslError(v)
    }
}

impl From<SetLoggerError> for Error {
    fn from(v: SetLoggerError) -> Self {
        Self::SetLoggerError(v)
    }
}

impl From<Log4RsError> for Error {
    fn from(v: Log4RsError) -> Self {
        Self::Log4RsError(v)
    }
}

impl From<HistoryError> for Error {
    fn from(v: HistoryError) -> Self {
        Self::HistoryError(v)
    }
}

impl From<PkiError> for Error {
    fn from(v: PkiError) -> Self {
        Self::PkiError(v)
    }
}
