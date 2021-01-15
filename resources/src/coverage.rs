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

use super::{
    misc::Code,
    primitives::{DateTime, Id},
};

#[derive(Clone, PartialEq, Debug)]
pub struct Coverage {
    pub id: Id,
    pub extension: Extension,
    pub type_: Code,
    pub beneficiary: String,
    pub period_end: Option<DateTime>,
    pub payor: Payor,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Extension {
    pub special_group: Option<Code>,
    pub dmp_mark: Option<Code>,
    pub insured_type: Option<Code>,
    pub wop: Option<Code>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Payor {
    pub display: String,
    pub value: Option<String>,
    pub alternative_id: Option<String>,
}
