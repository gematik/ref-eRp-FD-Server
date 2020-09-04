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

use resources::types::Profession;
use std::str::FromStr;

#[derive(Default)]
pub struct IdpClient {}

#[derive(Debug, Clone)]
pub struct IdToken(String);

#[derive(Debug, Clone, PartialEq)]
pub enum Error {}

impl IdpClient {
    pub fn get_profession(&self, _id_token: &IdToken) -> Result<Profession, Error> {
        Ok(Profession::Insured)
    }

    pub fn get_kvnr(&self, _id_token: &IdToken) -> Result<Option<String>, Error> {
        Ok(Some("KVNR0123456789".to_owned()))
    }
}

impl IdToken {
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl FromStr for IdToken {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(IdToken(s.to_owned()))
    }
}
