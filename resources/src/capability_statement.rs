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

use super::primitives::DateTime;

#[derive(Clone, Debug, PartialEq)]
pub struct CapabilityStatement {
    pub name: String,
    pub title: String,
    pub status: Status,
    pub date: DateTime,
    pub software: Software,
    pub description: String,
    pub fhir_version: FhirVersion,
    pub format: Vec<Format>,
    pub rest: Vec<Rest>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Software {
    pub name: String,
    pub version: String,
    pub release_date: DateTime,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Rest {
    pub mode: Mode,
    pub resource: Vec<Resource>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Resource {
    pub type_: Type,
    pub profile: String,
    pub supported_profiles: Vec<String>,
    pub operation: Vec<Operation>,
    pub interaction: Vec<Interaction>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Operation {
    pub name: String,
    pub definition: String,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FhirVersion {
    V4_0_0,
    V4_0_1,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Status {
    Draft,
    Active,
    Retired,
    Unknown,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Format {
    XML,
    JSON,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    Client,
    Server,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Type {
    Task,
    Operation,
    Communication,
    MedicationDispense,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Interaction {
    Read,
    Vread,
    Update,
    Patch,
    Delete,
    HistoryInstance,
    HistoryType,
    Create,
    SearchTyp,
}
