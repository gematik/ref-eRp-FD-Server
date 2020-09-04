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

#[cfg(feature = "support-json")]
pub mod json;
#[cfg(feature = "support-xml")]
pub mod xml;

use mime::Mime;

use super::super::header::Accept;
use super::constants::MIME_ANY;
#[cfg(feature = "support-json")]
use super::constants::{MIMES_FHIR_JSON, MIME_FHIR_JSON};
#[cfg(feature = "support-xml")]
use super::constants::{MIMES_FHIR_XML, MIME_FHIR_XML};

#[derive(Clone, Copy, PartialEq)]
pub enum DataType {
    Unknown,
    Any,

    #[cfg(feature = "support-json")]
    Json,

    #[cfg(feature = "support-xml")]
    Xml,
}

impl DataType {
    pub fn from_mime(mime: &Mime) -> Self {
        #[cfg(feature = "support-xml")]
        {
            if compare_mimes(mime, &*MIMES_FHIR_XML) {
                return Self::Xml;
            }
        }

        #[cfg(feature = "support-json")]
        {
            if compare_mimes(mime, &*MIMES_FHIR_JSON) {
                return Self::Json;
            }
        }

        if compare_mime(mime, &*MIME_ANY) {
            return Self::Any;
        }

        Self::Unknown
    }

    pub fn from_accept(accept: Accept) -> Option<Self> {
        let mut accept = accept.0;
        accept.sort_by(|a, b| a.quality.cmp(&b.quality));

        for mime in accept.iter().map(|m| &m.item) {
            let ret = Self::from_mime(mime);
            if ret != Self::Unknown {
                return Some(ret);
            }
        }

        if accept.is_empty() {
            None
        } else {
            Some(Self::Unknown)
        }
    }

    pub fn as_mime(&self) -> &'static Mime {
        match self {
            #[cfg(feature = "support-xml")]
            Self::Xml => &*MIME_FHIR_XML,

            #[cfg(feature = "support-json")]
            Self::Json => &*MIME_FHIR_JSON,

            Self::Any => &*MIME_ANY,
            Self::Unknown => panic!("Unknown data type!"),
        }
    }

    pub fn ignore_any(self) -> Option<Self> {
        if let Self::Any = self {
            None
        } else {
            Some(self)
        }
    }
}

impl Default for DataType {
    fn default() -> Self {
        #[cfg(feature = "support-xml")]
        {
            Self::Xml
        }

        #[cfg(all(feature = "support-json", not(feature = "support-xml")))]
        {
            Self::Json
        }

        #[cfg(all(not(feature = "support-json"), not(feature = "support-xml")))]
        {
            Self::Unknown
        }
    }
}

fn compare_mimes(mime: &Mime, mimes: &[Mime]) -> bool {
    for m in mimes {
        if compare_mime(mime, m) {
            return true;
        }
    }

    false
}

fn compare_mime(a: &Mime, b: &Mime) -> bool {
    a.essence_str() == b.essence_str()
}
