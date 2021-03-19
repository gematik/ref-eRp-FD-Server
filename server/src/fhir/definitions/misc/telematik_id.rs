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

use super::super::primitives::IdentifierEx;

impl IdentifierEx for TelematikId {
    fn from_parts(value: String) -> Result<Self, String> {
        Ok(Self(value))
    }

    fn value(&self) -> String {
        self.to_string()
    }

    fn system() -> Option<&'static str> {
        Some("https://gematik.de/fhir/NamingSystem/TelematikID")
    }
}
