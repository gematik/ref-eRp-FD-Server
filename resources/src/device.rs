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

use super::primitives::Id;

#[derive(Clone, PartialEq, Debug)]
pub struct Device {
    pub id: Id,
    pub status: Status,
    pub serial_number: Option<String>,
    pub device_name: DeviceName,
    pub version: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct DeviceName {
    pub name: String,
    pub type_: Type,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Status {
    Active,
    Inactive,
    EnteredInError,
    Unknown,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Type {
    UdiLabelName,
    UserFriendlyName,
    PatientReportedName,
    ManufacturerName,
    ModelName,
    Other,
}
