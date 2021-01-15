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

use resources::primitives::Id;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Not Found: {0}!")]
    NotFound(Id),

    #[error("Forbidden: {0}!")]
    Forbidden(Id),

    #[error("Conflict: {0}!")]
    Conflict(Id),

    #[error("Gone: {0}!")]
    Gone(Id),

    #[error("Missing e-Prescription Reference!")]
    EPrescriptionMissing,

    #[error("e-Prescription does not match!")]
    EPrescriptionMismatch,

    #[error("Referenced e-Prescription was not found: {0}!")]
    EPrescriptionNotFound(Id),

    #[error("ePrescription with this ID ({0}) was already registered!")]
    EPrescriptionAlreadyRegistered(Id),

    #[error("Bundle is missing an KV-Nr.!")]
    KvnrMissing,

    #[error("Bundle contains an invalid KV-Nr.!")]
    KvnrInvalid,

    #[error("Task is missing the Subject!")]
    SubjectMissing,

    #[error("Task Subject does not match!")]
    SubjectMismatch,

    #[error("Task Performer does not match!")]
    PerformerMismatch,

    #[error("Accept Timestamp is missing!")]
    AcceptTimestampMissing,

    #[error("Invalid Status!")]
    InvalidStatus,

    #[error("Invalid URL: {0}!")]
    InvalidUrl(String),
}
