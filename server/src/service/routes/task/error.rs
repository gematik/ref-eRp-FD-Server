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

use crate::fhir::security::SignedError;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Signed Error: {0}")]
    SignedError(SignedError),

    #[error("Not Found: /Task/{0}!")]
    NotFound(Id),

    #[error("Forbidden: /Task/{0}!")]
    Forbidden(Id),

    #[error("Conflict: /Task/{0}!")]
    Conflict(Id),

    #[error("Gone: /Task/{0}!")]
    Gone(Id),

    #[error("Missing e-Prescription Reference!")]
    EPrescriptionMissing,

    #[error("e-Prescription does not match!")]
    EPrescriptionMismatch,

    #[error("Referenced e-Prescription was not found: {0}!")]
    EPrescriptionNotFound(Id),

    #[error("ePrescription with this ID ({0}) was already registered!")]
    EPrescriptionAlreadyRegistered(Id),

    #[error("Referenced Patient Receipt was not found: {0}!")]
    PatientReceiptNotFound(Id),

    #[error("Referenced Erx Receipt was not found: {0}!")]
    ErxReceiptNotFound(Id),

    #[error("Bundle is missing an KV-Nr.!")]
    KvnrMissing,

    #[error("Bundle contains an invalid KV-Nr.!")]
    KvnrInvalid,

    #[error("Unablt to convert Access Token to Audit Event Agent!")]
    AuditEventAgentInvalid,

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

    #[error("Unable to generate prescription id (try again later)!")]
    GeneratePrescriptionId,
}

impl From<SignedError> for Error {
    fn from(err: SignedError) -> Self {
        Self::SignedError(err)
    }
}
