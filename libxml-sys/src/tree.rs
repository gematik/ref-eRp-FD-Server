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

use super::xmlChar;

pub enum xmlDoc {}

#[repr(C)]
#[derive(Debug)]
pub struct xmlNode {
    pub _private: *mut c_void,
    pub type_: xmlElementType,
    pub name: *const xmlChar,
    pub children: *mut xmlNode,
    pub last: *mut xmlNode,
    pub parent: *mut xmlNode,
    pub next: *mut xmlNode,
    pub prev: *mut xmlNode,
    pub doc: *mut xmlDoc,
    pub ns: *mut xmlNs,
    pub content: *mut xmlChar,
    pub properties: *mut xmlAttr,
    pub ns_def: *mut xmlNs,
    pub psvi: *mut c_void,
    pub line: c_ushort,
    pub extra: c_ushort,
}

#[repr(C)]
#[derive(Debug)]
pub struct xmlAttr {
    pub _private: *mut c_void,
    pub type_: xmlElementType,
    pub name: *const xmlChar,
    pub children: *mut xmlNode,
    pub last: *mut xmlNode,
    pub parent: *mut xmlNode,
    pub next: *mut xmlAttr,
    pub prev: *mut xmlAttr,
    pub doc: *mut xmlDoc,
    pub ns: *mut xmlNs,
    pub atype: xmlAttributeType,
    pub psvi: *mut c_void,
}

#[repr(C)]
#[derive(Debug)]
pub struct xmlNs {
    pub next: *mut xmlNs,
    pub type_: xmlNsType,
    pub href: *const xmlChar,
    pub prefix: *const xmlChar,
    pub _private: *mut c_void,
    pub context: *mut xmlDoc,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum xmlElementType {
    XML_ELEMENT_NODE = 1,
    XML_ATTRIBUTE_NODE = 2,
    XML_TEXT_NODE = 3,
    XML_CDATA_SECTION_NODE = 4,
    XML_ENTITY_REF_NODE = 5,
    XML_ENTITY_NODE = 6,
    XML_PI_NODE = 7,
    XML_COMMENT_NODE = 8,
    XML_DOCUMENT_NODE = 9,
    XML_DOCUMENT_TYPE_NODE = 10,
    XML_DOCUMENT_FRAG_NODE = 11,
    XML_NOTATION_NODE = 12,
    XML_HTML_DOCUMENT_NODE = 13,
    XML_DTD_NODE = 14,
    XML_ELEMENT_DECL = 15,
    XML_ATTRIBUTE_DECL = 16,
    XML_ENTITY_DECL = 17,
    XML_NAMESPACE_DECL = 18,
    XML_XINCLUDE_START = 19,
    XML_XINCLUDE_END = 20,
    XML_DOCB_DOCUMENT_NODE = 21,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum xmlAttributeType {
    XML_ATTRIBUTE_CDATA = 1,
    XML_ATTRIBUTE_ID = 2,
    XML_ATTRIBUTE_IDREF = 3,
    XML_ATTRIBUTE_IDREFS = 4,
    XML_ATTRIBUTE_ENTITY = 5,
    XML_ATTRIBUTE_ENTITIES = 6,
    XML_ATTRIBUTE_NMTOKEN = 7,
    XML_ATTRIBUTE_NMTOKENS = 8,
    XML_ATTRIBUTE_ENUMERATION = 9,
    XML_ATTRIBUTE_NOTATION = 10,
}

pub type xmlNsType = xmlElementType;

#[link(name = "xml2")]
extern "C" {
    /* doc */
    pub fn xmlFreeDoc(doc: *mut xmlDoc);
    pub fn xmlFreeNode(node: *mut xmlNode);
    pub fn xmlDocGetRootElement(doc: *mut xmlDoc) -> *mut xmlNode;
    pub fn xmlSearchNs(doc: *mut xmlDoc, node: *mut xmlNode, ns: *const xmlChar) -> *mut xmlNs;
    pub fn xmlGetNsList(doc: *const xmlDoc, node: *const xmlNode) -> *mut xmlNs;

    /* node */
    pub fn xmlChildElementCount(parent: *mut xmlNode) -> c_ulong;
    pub fn xmlNextElementSibling(node: *mut xmlNode) -> *mut xmlNode;
    pub fn xmlPreviousElementSibling(node: *mut xmlNode) -> *mut xmlNode;
    pub fn xmlFirstElementChild(parent: *mut xmlNode) -> *mut xmlNode;
    pub fn xmlLastElementChild(parent: *mut xmlNode) -> *mut xmlNode;
    pub fn xmlGetLastChild(parent: *mut xmlNode) -> *mut xmlNode;
    pub fn xmlNodeGetContent(node: *mut xmlNode) -> *mut xmlChar;

    /* ns */
    pub fn xmlFreeNs(ns: *mut xmlNs);

    /* attr */
    pub fn xmlGetProp(node: *mut xmlNode, name: *const xmlChar) -> *mut xmlChar;
}
