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

use super::{misc::Address, primitives::Id};

#[derive(Clone, Debug, PartialEq)]
pub struct Organization {
    pub id: Id,
    pub name: Option<String>,
    pub identifier: Option<Identifier>,
    pub telecom: Telecom,
    pub address: Option<Address>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Telecom {
    pub phone: String,
    pub fax: Option<String>,
    pub mail: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Identifier {
    IK(String),
    BS(String),
    KZV(String),
}
