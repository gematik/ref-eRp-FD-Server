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

extern crate libxml_sys as ffi;

mod doc;
mod error;
mod io;
mod namespace;
mod node;
mod nodeset;

pub use doc::*;
pub use error::*;
pub use io::*;
pub use namespace::*;
pub use node::*;
pub use nodeset::*;

pub use ffi::xmlAttributeType as AttributeType;
pub use ffi::xmlElementType as ElementType;
pub use ffi::XmlC14NMode as C14nMode;
