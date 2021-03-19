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

use std::ffi::CStr;

use foreign_types::{foreign_type, ForeignTypeRef};
use libc::*;

use super::{ElementType, Error};

foreign_type! {
    pub unsafe type Namespace: Send {
        type CType = ffi::xmlNs;
        fn drop = ffi::xmlFreeNs;
    }
}

impl NamespaceRef {
    pub fn type_(&self) -> ElementType {
        unsafe { (*self.as_ptr()).type_ }
    }

    pub fn href(&self) -> Result<&str, Error> {
        unsafe {
            let ptr = self.as_ptr();
            let ns = &*ptr;

            let href = ns.href as *const c_char;
            let href = Error::check_ptr(href, "xmlNs.href")?;

            let s = CStr::from_ptr(href).to_str()?;

            Ok(s)
        }
    }

    pub fn prefix(&self) -> Result<&str, Error> {
        unsafe {
            let ptr = self.as_ptr();
            let ns = &*ptr;

            let href = ns.prefix as *const c_char;
            let href = Error::check_ptr(href, "xmlNs.prefix")?;

            let s = CStr::from_ptr(href).to_str()?;

            Ok(s)
        }
    }
}
