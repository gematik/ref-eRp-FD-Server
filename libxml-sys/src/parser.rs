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

use libc::*;

use super::xmlDoc;

#[link(name = "xml2")]
extern "C" {
    pub fn xmlParseFile(filename: *const c_char) -> *mut xmlDoc;
    pub fn xmlParseMemory(buffer: *mut c_char, size: c_int) -> *mut xmlDoc;
}
