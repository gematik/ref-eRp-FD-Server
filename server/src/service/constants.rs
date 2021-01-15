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

use mime::Mime;

lazy_static! {
    pub static ref MIME_ANY: Mime = "*/*".parse().unwrap();
    pub static ref MIME_FHIR_XML: Mime = "application/fhir+xml".parse().unwrap();
    pub static ref MIME_FHIR_JSON: Mime = "application/fhir+json".parse().unwrap();
    pub static ref MIMES_FHIR_XML: [Mime; 3] = [
        "text/xml".parse().unwrap(),
        "application/xml".parse().unwrap(),
        "application/fhir+xml".parse().unwrap(),
    ];
    pub static ref MIMES_FHIR_JSON: [Mime; 2] = [
        "application/json".parse().unwrap(),
        "application/fhir+json".parse().unwrap(),
    ];
}
