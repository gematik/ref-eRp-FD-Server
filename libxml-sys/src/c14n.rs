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

use super::{xmlChar, xmlDoc, xmlNode, xmlOutputBuffer};

pub type XmlC14NIsVisibleCallback = extern "C" fn(*mut c_void, *mut xmlNode, *mut xmlNode) -> c_int;

#[repr(C)]
pub enum XmlC14NMode {
    XML_C14N_1_0 = 0,
    XML_C14N_EXCLUSIVE_1_0 = 1,
    XML_C14N_1_1 = 2,
}

#[link(name = "xml2")]
extern "C" {
    pub fn xmlC14NExecute(
        doc: *mut xmlDoc,
        is_visible_callback: XmlC14NIsVisibleCallback,
        user_data: *mut c_void,
        mode: c_int,
        inclusive_ns_prefixes: *mut *mut xmlChar,
        with_comments: c_int,
        buf: *mut xmlOutputBuffer,
    ) -> c_int;
}
