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

use foreign_types::{foreign_type, ForeignType, ForeignTypeRef};

use super::{Error, NodeRef};

foreign_type! {
    pub unsafe type NodeSet<'a>: Send {
        type CType = ffi::xmlNodeSet;
        type PhantomData = &'a NodeRef;

        fn drop = ffi::xmlXPathFreeNodeSet;
    }
}

impl<'a> NodeSet<'a> {
    pub fn from_node(node: &'a NodeRef) -> Result<Self, Error> {
        unsafe {
            let ptr = ffi::xmlXPathNodeSetCreate(node.as_ptr());
            let ptr = Error::check_ptr_mut(ptr, "xmlXPathNodeSetCreate")?;

            let set = NodeSet::from_ptr(ptr);

            Ok(set)
        }
    }
}

impl NodeSetRef<'_> {
    pub fn contains(&self, node: &NodeRef) -> bool {
        unsafe {
            let ptr = self.as_ptr();
            let node = node.as_ptr();

            ffi::xmlXPathNodeSetContains(ptr, node) != 0
        }
    }
}
