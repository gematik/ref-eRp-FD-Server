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

use std::ffi::CString;
use std::ptr::null_mut;
use std::{path::Path, str::FromStr};

use foreign_types::{foreign_type, ForeignType, ForeignTypeRef};

use super::{C14nMode, Error, NodeRef, OutputBuffer};

foreign_type! {
    pub unsafe type Doc: Send {
        type CType = ffi::xmlDoc;
        fn drop = ffi::xmlFreeDoc;
    }
}

pub trait NodeVisibility {
    fn is_visible(&self, node: &NodeRef, parent: Option<&NodeRef>) -> bool;
}

impl Doc {
    pub fn from_file<P: AsRef<Path>>(filename: P) -> Result<Doc, Error> {
        unsafe {
            let filename = filename.as_ref();

            let buf = filename
                .to_str()
                .ok_or_else(|| Error::InvalidFilepath(filename.to_owned()))?;
            let buf = CString::new(buf).map_err(|_| Error::InvalidFilepath(filename.to_owned()))?;

            let ptr = ffi::xmlParseFile(buf.as_ptr());
            let ptr = Error::check_ptr_mut(ptr, "xmlParseFile")?;

            let doc = Doc::from_ptr(ptr);

            Ok(doc)
        }
    }
}

impl FromStr for Doc {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        unsafe {
            let bytes = s.as_bytes();
            let buf = bytes.as_ptr() as *mut c_char;
            let len = bytes.len() as c_int;

            let ptr = ffi::xmlParseMemory(buf, len);
            let ptr = Error::check_ptr_mut(ptr, "xmlParseMemory")?;

            let doc = Doc::from_ptr(ptr);

            Ok(doc)
        }
    }
}

impl DocRef {
    pub fn root(&self) -> Result<&NodeRef, Error> {
        unsafe {
            let ptr = ffi::xmlDocGetRootElement(self.as_ptr());
            let ptr = Error::check_ptr_mut(ptr, "xmlDocGetRootElement")?;

            let node = NodeRef::from_ptr(ptr);

            Ok(node)
        }
    }

    pub fn c14n<V>(
        &self,
        node_visibility: &V,
        mode: C14nMode,
        with_comments: bool,
        namespaces: Option<&[&str]>,
        output: &OutputBuffer<'_>,
    ) -> Result<(), Error>
    where
        V: NodeVisibility,
    {
        unsafe {
            let doc = self.as_ptr();

            let context = WrappedNodeVisibility(node_visibility);
            let context = &context as *const _ as *mut c_void;

            let mode = mode as c_int;

            let with_comments = if with_comments { 1 } else { 0 };

            let output = output.as_ptr();

            let ret = match namespaces {
                Some(namespaces) => {
                    let namespaces = namespaces.iter().map(|ns| ns.as_ptr()).collect::<Vec<_>>();
                    let namespaces = namespaces.as_ptr() as *mut *mut ffi::xmlChar;

                    ffi::xmlC14NExecute(
                        doc,
                        c14n_node_is_visible,
                        context,
                        mode,
                        namespaces,
                        with_comments,
                        output,
                    )
                }
                None => ffi::xmlC14NExecute(
                    doc,
                    c14n_node_is_visible,
                    context,
                    mode,
                    null_mut(),
                    with_comments,
                    output,
                ),
            };

            if ret >= 0 {
                Ok(())
            } else {
                Err(Error::last_error(Error::C14nFailed))
            }
        }
    }
}

struct WrappedNodeVisibility<'a>(&'a dyn NodeVisibility);

extern "C" fn c14n_node_is_visible(
    context: *mut c_void,
    node: *mut ffi::xmlNode,
    parent: *mut ffi::xmlNode,
) -> c_int {
    unsafe {
        if node.is_null() {
            return 0;
        }

        let node = NodeRef::from_ptr(node);
        let parent = if !parent.is_null() {
            Some(NodeRef::from_ptr(parent))
        } else {
            None
        };

        let context = &*(context as *const WrappedNodeVisibility<'static>);
        if context.0.is_visible(&node, parent) {
            1
        } else {
            0
        }
    }
}
