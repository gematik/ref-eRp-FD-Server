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

#[derive(Clone, PartialEq, Debug)]
pub struct OperationOutcome {
    pub issue: Vec<Issue>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Issue {
    pub severity: Severity,
    pub code: IssueType,
    pub details: Option<String>,
    pub diagnostics: Option<String>,
    pub expression: Vec<String>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Severity {
    Fatal,
    Error,
    Warning,
    Information,
}

#[derive(Clone, PartialEq, Debug)]
pub enum IssueType {
    Invalid,
    InvalidStructure,
    InvalidRequired,
    InvalidValue,
    InvalidInvariant,
    Security,
    SecurityLogin,
    SecurityUnknown,
    SecurityExpired,
    SecurityForbidden,
    SecuritySuppressed,
    Processing,
    ProcessingNotSupported,
    ProcessingDuplicate,
    ProcessingMultipleMatches,
    ProcessingNotFound,
    ProcessingDeleted,
    ProcessingTooLong,
    ProcessingCodeInvalid,
    ProcessingExtension,
    ProcessingTooCostly,
    ProcessingBusinessRule,
    ProcessingConflict,
    Transient,
    TransientLockError,
    TransientNoStore,
    TransientException,
    TransientTimeout,
    TransientIncomplete,
    TransientThrottled,
    Informational,
}
