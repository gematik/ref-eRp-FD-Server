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

#[cfg(not(any(feature = "interface-patient", feature = "interface-supplier")))]
compile_error!(
    r##"At least one of the following features must be defined:
    - interface-patient
    - interface-supplier"##
);

#[macro_use]
extern crate lazy_static;

#[macro_use]
mod macros;

mod fhir;

pub mod error;
pub mod logging;
pub mod service;
pub mod tsl;
