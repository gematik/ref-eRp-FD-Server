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

use super::{xmlChar, xmlDoc, xmlNode};

pub enum xmlXPathContext {}

#[repr(C)]
#[derive(Debug)]
pub struct xmlNodeSet {
    pub node_nr: c_int,
    pub node_max: c_int,
    pub node_tab: *mut *mut xmlNode,
}

#[repr(C)]
#[derive(Debug)]
pub struct xmlXPathObject {
    pub type_: xmlXPathObjectType,
    pub nodesetval: *mut xmlNodeSet,
    pub boolval: c_int,
    pub floatval: f64,
    pub stringval: *mut xmlChar,
    pub user: *mut c_void,
    pub index: c_int,
    pub user2: *mut c_void,
    pub index2: c_int,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum xmlXPathObjectType {
    XPATH_UNDEFINED = 0,
    XPATH_NODESET = 1,
    XPATH_BOOLEAN = 2,
    XPATH_NUMBER = 3,
    XPATH_STRING = 4,
    XPATH_POINT = 5,
    XPATH_RANGE = 6,
    XPATH_LOCATIONSET = 7,
    XPATH_USERS = 8,
    XPATH_XSLT_TREE = 9,
}

#[link(name = "xml2")]
extern "C" {
    pub fn xmlXPathNewContext(doc: *mut xmlDoc) -> *mut xmlXPathContext;
    pub fn xmlXPathFreeContext(context: *mut xmlXPathContext);
    pub fn xmlXPathFreeObject(obj: *mut xmlXPathObject);

    pub fn xmlXPathEvalExpression(
        xpath: *const xmlChar,
        context: *mut xmlXPathContext,
    ) -> *mut xmlXPathObject;

    pub fn xmlXPathNodeSetCreate(val: *mut xmlNode) -> *mut xmlNodeSet;
    pub fn xmlXPathFreeNodeSet(obj: *mut xmlNodeSet);
    pub fn xmlXPathNodeSetContains(set: *mut xmlNodeSet, val: *mut xmlNode) -> c_int;
}
