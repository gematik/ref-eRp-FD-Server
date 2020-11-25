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

use crate::primitives::Instant;

pub trait WithSignature {
    fn signatures(&self) -> &Vec<Signature>;

    fn signatures_mut(&mut self) -> &mut Vec<Signature>;
}

#[derive(Clone, PartialEq, Debug)]
pub struct Signature {
    pub type_: Type,
    pub when: Instant,
    pub who: String,
    pub data: String,
    pub format: Option<Format>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Type {
    AuthorsSignature,
    CoauthorsSignature,
    CoParticipantsSignature,
    TranscriptionistRecorderSignature,
    VerificationSignature,
    ValidationSignature,
    ConsentSignature,
    SignatureWitnessSignature,
    EventWitnessSignature,
    IdentityWitnessSignature,
    ConsentWitnessSignature,
    InterpreterSignature,
    ReviewSignature,
    SourceSignature,
    AddendumSignature,
    ModificationSignature,
    AdministrativeSignature,
    TimestampSignature,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Format {
    Json,
    Unknown,
}
