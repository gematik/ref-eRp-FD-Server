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

use resources::misc::TelematikId;

use super::{Comperator, Parameter};

impl Parameter for TelematikId {
    type Storage = TelematikId;

    fn parse(s: &str) -> Result<Self::Storage, String> {
        Ok(TelematikId::new(s))
    }

    fn compare(&self, comperator: Comperator, param: &Self::Storage) -> bool {
        match comperator {
            Comperator::Equal => self.0 == param.0,
            Comperator::NotEqual => self.0 != param.0,
            Comperator::GreaterThan => self.0 > param.0,
            Comperator::LessThan => self.0 < param.0,
            Comperator::GreaterEqual => self.0 >= param.0,
            Comperator::LessEqual => self.0 <= param.0,
            _ => false,
        }
    }
}
