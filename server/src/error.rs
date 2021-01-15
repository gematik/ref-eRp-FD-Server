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
use thiserror::Error;

use crate::{service::Error as ServiceError, tasks::puk_token::Error as PukTokenError};

#[derive(Error, Debug)]
pub enum Error {
    #[error("Generic Error: {0}")]
    Generic(String),

    #[error("IO Error: {0}")]
    IoError(IoError),

    #[error("Unable to set logger: {0}")]
    SetLoggerError(SetLoggerError),

    #[error("Unable to setup log4rs: {0}")]
    Log4RsError(Log4RsError),

    #[error("Service Error: {0}")]
    ServiceError(ServiceError),

    #[error("PUK_TOKEN Error: {0}")]
    PukTokenError(PukTokenError),
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

impl From<ServiceError> for Error {
    fn from(v: ServiceError) -> Self {
        Self::ServiceError(v)
    }
}

impl From<PukTokenError> for Error {
    fn from(v: PukTokenError) -> Self {
        Self::PukTokenError(v)
    }
}
