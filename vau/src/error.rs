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

use actix_web::{error::ResponseError, http::StatusCode};
use openssl::error::ErrorStack as OpenSslError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("VAU Request without payload!")]
    NoPayload,

    #[error("VAU Request with incomplete payload!")]
    PayloadIncomplete,

    #[error("Unable to decode VAU payload!")]
    DecodeError,

    #[error("Error in OpenSSL Library: {0}")]
    OpenSslError(OpenSslError),

    #[error("Internal Error!")]
    Internal,
}

impl From<OpenSslError> for Error {
    fn from(v: OpenSslError) -> Self {
        Error::OpenSslError(v)
    }
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Error::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::BAD_REQUEST,
        }
    }
}
