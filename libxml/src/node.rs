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

use std::{
    cmp::{Eq, PartialEq},
    ffi::CStr,
    hash::{Hash, Hasher},
    iter::{DoubleEndedIterator, Iterator},
    ptr::{null, null_mut},
};

use foreign_types::{foreign_type, ForeignTypeRef};
use libc::*;

use super::{DocRef, ElementType, Error, NamespaceRef};

/* Node */

foreign_type! {
    pub unsafe type Node: Send {
        type CType = ffi::xmlNode;
        fn drop = ffi::xmlFreeNode;
    }
}

impl NodeRef {
    pub fn type_(&self) -> ElementType {
        unsafe { (*self.as_ptr()).type_ }
    }

    pub fn name(&self) -> Result<&str, Error> {
        unsafe {
            let ptr = self.as_ptr();
            let node = &*ptr;

            let ptr = node.name as *const c_char;
            let ptr = Error::check_ptr(ptr, "xmlNode.name")?;

            let s = CStr::from_ptr(ptr).to_str()?;

            Ok(s)
        }
    }

    pub fn ns(&self) -> Option<&NamespaceRef> {
        unsafe {
            let ptr = self.as_ptr();
            let node = &*ptr;

            if !node.ns.is_null() {
                return Some(NamespaceRef::from_ptr(node.ns));
            }

            let ns = ffi::xmlSearchNs(node.doc, ptr, null());
            if !ns.is_null() {
                return Some(NamespaceRef::from_ptr(ns));
            }

            None
        }
    }

    pub fn doc(&self) -> Result<&DocRef, Error> {
        unsafe {
            let ptr = self.as_ptr();
            let ptr = (*ptr).doc;
            let ptr = Error::check_ptr_mut(ptr, "xmlNode.doc")?;

            Ok(DocRef::from_ptr(ptr))
        }
    }

    pub fn parent(&self) -> Option<&NodeRef> {
        unsafe {
            let ptr = self.as_ptr();
            let ptr = (*ptr).parent;

            if ptr.is_null() {
                return None;
            }

            Some(NodeRef::from_ptr(ptr))
        }
    }

    pub fn has_parent(&self, node: &NodeRef) -> bool {
        unsafe {
            let p = node.as_ptr();
            let mut ptr = self.as_ptr();

            while !ptr.is_null() {
                if ptr == p {
                    return true;
                }

                ptr = (*ptr).parent;
            }

            false
        }
    }

    pub fn xpath(&self, path: Option<&str>) -> Result<Option<&NodeRef>, Error> {
        match path {
            None | Some("") => Ok(Some(self)),
            Some(x) => unsafe {
                let doc = self.doc()?;
                let doc = doc.as_ptr();

                let xpath = if let Some(x) = x.strip_prefix('#') {
                    format!("//*[@Id=\"{}\"]\0", x)
                } else {
                    format!("{}\0", x)
                };

                let context = ffi::xmlXPathNewContext(doc);
                let context = Error::check_ptr_mut(context, "xmlXPathNewContext")?;

                let obj_ptr = ffi::xmlXPathEvalExpression(xpath.as_bytes().as_ptr(), context);
                let obj_ptr = match Error::check_ptr_mut(obj_ptr, "xmlXPathEvalExpression") {
                    Ok(obj_ptr) => obj_ptr,
                    Err(err) => {
                        ffi::xmlXPathFreeContext(context);

                        return Err(err);
                    }
                };

                let mut ret = None;
                let obj = &*obj_ptr;
                if !obj.nodesetval.is_null() {
                    let nodeset = &*obj.nodesetval;
                    if nodeset.node_nr == 1 {
                        let ptr = nodeset.node_tab;
                        ret = Some(NodeRef::from_ptr(*ptr));
                    }
                }

                ffi::xmlXPathFreeObject(obj_ptr);
                ffi::xmlXPathFreeContext(context);

                Ok(ret)
            },
        }
    }

    pub fn first_child(&self) -> Option<&NodeRef> {
        unsafe {
            let ptr = self.as_ptr();
            let ptr = (*ptr).children;
            if ptr.is_null() {
                None
            } else {
                Some(NodeRef::from_ptr(ptr))
            }
        }
    }

    pub fn first_child_element(&self) -> Option<&NodeRef> {
        unsafe {
            let ptr = ffi::xmlFirstElementChild(self.as_ptr());
            if ptr.is_null() {
                None
            } else {
                Some(NodeRef::from_ptr(ptr))
            }
        }
    }

    pub fn last_child(&self) -> Option<&NodeRef> {
        unsafe {
            let ptr = ffi::xmlGetLastChild(self.as_ptr());
            if ptr.is_null() {
                None
            } else {
                Some(NodeRef::from_ptr(ptr))
            }
        }
    }

    pub fn last_child_element(&self) -> Option<&NodeRef> {
        unsafe {
            let ptr = ffi::xmlLastElementChild(self.as_ptr());
            if ptr.is_null() {
                None
            } else {
                Some(NodeRef::from_ptr(ptr))
            }
        }
    }

    pub fn next_sibling(&self) -> Option<&NodeRef> {
        unsafe {
            let ptr = self.as_ptr();
            let ptr = (*ptr).next;
            if ptr.is_null() {
                None
            } else {
                Some(NodeRef::from_ptr(ptr))
            }
        }
    }

    pub fn next_sibling_element(&self) -> Option<&NodeRef> {
        unsafe {
            let ptr = ffi::xmlNextElementSibling(self.as_ptr());
            if ptr.is_null() {
                None
            } else {
                Some(NodeRef::from_ptr(ptr))
            }
        }
    }

    pub fn prev_sibling(&self) -> Option<&NodeRef> {
        unsafe {
            let ptr = self.as_ptr();
            let ptr = (*ptr).prev;
            if ptr.is_null() {
                None
            } else {
                Some(NodeRef::from_ptr(ptr))
            }
        }
    }

    pub fn prev_sibling_element(&self) -> Option<&NodeRef> {
        unsafe {
            let ptr = ffi::xmlPreviousElementSibling(self.as_ptr());
            if ptr.is_null() {
                None
            } else {
                Some(NodeRef::from_ptr(ptr))
            }
        }
    }

    pub fn next_element(&self) -> Option<&NodeRef> {
        let mut node = Some(self);

        while let Some(n) = node {
            if n.type_() == ElementType::XML_ELEMENT_NODE {
                return Some(n);
            }

            node = n.next_sibling_element();
        }

        None
    }

    pub fn prev_element(&self) -> Option<&NodeRef> {
        let mut node = Some(self);

        while let Some(n) = node {
            if n.type_() == ElementType::XML_ELEMENT_NODE {
                return Some(n);
            }

            node = n.prev_sibling_element();
        }

        None
    }

    pub fn children(&self) -> ChildIter<'_> {
        ChildIter::new(self.first_child())
    }

    pub fn child_elements(&self) -> ChildElementIter<'_> {
        ChildElementIter::new(self.first_child_element())
    }

    pub fn search<'a, F>(&'a self, f: &mut F) -> Option<&NodeRef>
    where
        F: FnMut(&'a NodeRef) -> bool,
    {
        if f(self) {
            return Some(self);
        }

        for child in self.children() {
            if let Some(node) = child.search(f) {
                return Some(node);
            }
        }

        None
    }

    pub fn search_parent<F>(&self, f: F) -> Option<&NodeRef>
    where
        F: Fn(&NodeRef) -> bool + Copy,
    {
        if f(self) {
            return Some(self);
        }

        if let Some(parent) = self.parent() {
            parent.search_parent(f)
        } else {
            None
        }
    }

    pub fn prop(&self, name: &str) -> Result<Option<String>, Error> {
        unsafe {
            let ptr = self.as_ptr();
            let node = &*ptr;

            if node.type_ != ElementType::XML_ELEMENT_NODE {
                return Ok(None);
            }

            let mut prop = node.properties;
            while !prop.is_null() {
                let p = &*prop;
                prop = (*prop).next;

                let s = CStr::from_ptr(p.name as *const c_char).to_string_lossy();

                if s != name {
                    continue;
                }

                if p.type_ != ElementType::XML_ATTRIBUTE_NODE {
                    continue;
                }

                if p.children.is_null() {
                    return Ok(Some(String::new()));
                }

                let c = &*p.children;
                if c.next.is_null()
                    && (c.type_ == ElementType::XML_TEXT_NODE
                        || c.type_ == ElementType::XML_CDATA_SECTION_NODE)
                {
                    return Ok(Some(
                        CStr::from_ptr(c.content as *const c_char)
                            .to_str()?
                            .to_owned(),
                    ));
                }
            }

            Ok(None)
        }
    }

    pub fn content(&self) -> Result<Option<String>, Error> {
        let mut buf = String::new();

        if get_content(&mut buf, self)? {
            Ok(Some(buf))
        } else {
            Ok(None)
        }
    }
}

impl PartialEq for NodeRef {
    fn eq(&self, other: &NodeRef) -> bool {
        self.as_ptr() == other.as_ptr()
    }
}

impl Eq for NodeRef {}

impl Hash for NodeRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ptr().hash(state)
    }
}

/* ChildIter */

pub struct ChildIter<'a> {
    next: Option<&'a NodeRef>,
}

impl<'a> ChildIter<'a> {
    fn new(next: Option<&'a NodeRef>) -> Self {
        Self { next }
    }
}

impl<'a> Iterator for ChildIter<'a> {
    type Item = &'a NodeRef;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.next {
            self.next = next.next_sibling();

            Some(next)
        } else {
            None
        }
    }
}

impl<'a> DoubleEndedIterator for ChildIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.next {
            self.next = next.prev_sibling();

            Some(next)
        } else {
            None
        }
    }
}

/* ChildElementIter */

pub struct ChildElementIter<'a> {
    next: Option<&'a NodeRef>,
}

impl<'a> ChildElementIter<'a> {
    fn new(next: Option<&'a NodeRef>) -> Self {
        Self { next }
    }
}

impl<'a> Iterator for ChildElementIter<'a> {
    type Item = &'a NodeRef;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.next {
            self.next = next.next_sibling_element();

            Some(next)
        } else {
            None
        }
    }
}

impl<'a> DoubleEndedIterator for ChildElementIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.next {
            self.next = next.prev_sibling_element();

            Some(next)
        } else {
            None
        }
    }
}

fn get_content(buf: &mut String, node: &NodeRef) -> Result<bool, Error> {
    unsafe {
        match node.type_() {
            ElementType::XML_DOCUMENT_FRAG_NODE | ElementType::XML_ELEMENT_NODE => {
                let ptr = node.as_ptr();
                let mut tmp = ptr;
                let mut ret = false;

                while !tmp.is_null() {
                    match (*tmp).type_ {
                        ElementType::XML_CDATA_SECTION_NODE | ElementType::XML_TEXT_NODE => {
                            let c = (*tmp).content;
                            if !c.is_null() {
                                ret = true;
                                *buf += str_from_ptr(c as *const c_char)?;
                            }
                        }
                        ElementType::XML_ENTITY_REF_NODE => {
                            ret |= get_content(buf, &NodeRef::from_ptr(tmp))?;
                        }
                        _ => (),
                    }

                    let c = (*tmp).children;
                    if !c.is_null() && (*c).type_ != ElementType::XML_ELEMENT_DECL {
                        tmp = c;
                        continue;
                    }

                    if tmp == ptr {
                        break;
                    }

                    if !(*tmp).next.is_null() {
                        tmp = (*tmp).next;
                        continue;
                    }

                    while !tmp.is_null() {
                        tmp = (*tmp).parent;
                        if tmp.is_null() {
                            break;
                        }

                        if tmp == ptr {
                            tmp = null_mut();
                            break;
                        }

                        if !(*tmp).next.is_null() {
                            tmp = (*tmp).next;
                            break;
                        }
                    }
                }

                Ok(ret)
            }
            ElementType::XML_DOCUMENT_NODE
            | ElementType::XML_DOCB_DOCUMENT_NODE
            | ElementType::XML_HTML_DOCUMENT_NODE => {
                let ptr = node.as_ptr();
                let mut ret = false;
                let mut cur = (*ptr).children;

                while !cur.is_null() {
                    let ptr = cur;
                    let n = &*ptr;
                    cur = n.next;

                    if n.type_ == ElementType::XML_ELEMENT_NODE
                        || n.type_ == ElementType::XML_TEXT_NODE
                        || n.type_ == ElementType::XML_CDATA_SECTION_NODE
                    {
                        ret |= get_content(buf, &NodeRef::from_ptr(ptr))?;
                    }
                }

                Ok(ret)
            }
            ElementType::XML_COMMENT_NODE
            | ElementType::XML_PI_NODE
            | ElementType::XML_CDATA_SECTION_NODE
            | ElementType::XML_TEXT_NODE => {
                let ptr = node.as_ptr();
                let n = &*ptr;

                if n.content.is_null() {
                    Ok(false)
                } else {
                    *buf += str_from_ptr(n.content as *const c_char)?;

                    Ok(true)
                }
            }
            _ => Ok(false),
        }
    }
}

unsafe fn str_from_ptr(s: *const c_char) -> Result<&'static str, Error> {
    Ok(CStr::from_ptr(s).to_str()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::str::FromStr;

    use super::super::Doc;

    #[test]
    fn children_iterator() {
        let doc = test_doc();
        let root = doc.root().unwrap();

        let mut iter = root.children();

        let el = iter.next();
        assert!(el.is_some());
        assert_eq!(el.unwrap().type_(), ElementType::XML_TEXT_NODE);

        let el = iter.next();
        assert!(el.is_some());
        assert_eq!(el.unwrap().type_(), ElementType::XML_ELEMENT_NODE);
        assert_eq!(el.unwrap().name().unwrap(), "element1");

        let el = iter.next();
        assert!(el.is_some());
        assert_eq!(el.unwrap().type_(), ElementType::XML_TEXT_NODE);

        let el = iter.next();
        assert!(el.is_some());
        assert_eq!(el.unwrap().name().unwrap(), "element2");

        let el = iter.next();
        assert!(el.is_some());
        assert_eq!(el.unwrap().type_(), ElementType::XML_TEXT_NODE);

        let el = iter.next();
        assert!(el.is_some());
        assert_eq!(el.unwrap().name().unwrap(), "element3");

        let el = iter.next();
        assert!(el.is_some());
        assert_eq!(el.unwrap().type_(), ElementType::XML_TEXT_NODE);

        let el = iter.next();
        assert!(el.is_none());
    }

    #[test]
    fn child_element_iterator() {
        let doc = test_doc();
        let root = doc.root().unwrap();

        let mut iter = root.child_elements();

        let el = iter.next();
        assert!(el.is_some());
        assert_eq!(el.unwrap().name().unwrap(), "element1");

        let el = iter.next();
        assert!(el.is_some());
        assert_eq!(el.unwrap().name().unwrap(), "element2");

        let el = iter.next();
        assert!(el.is_some());
        assert_eq!(el.unwrap().name().unwrap(), "element3");

        let el = iter.next();
        assert!(el.is_none());
    }

    fn test_doc() -> Doc {
        Doc::from_str(
            r##"
            <root>
                <element1>
                    <child1>
                        <data1>Element1.Child1.Data1</data1>
                        <data2>Element1.Child1.Data2</data2>
                    </child1>
                    <child2>
                        <data1>Element1.Child2.Data1</data1>
                        <data2>Element1.Child2.Data2</data2>
                    </child2>
                </element1>
                <element2>
                    <child1>
                        <data1>Element2.Child1.Data1</data1>
                        <data2>Element2.Child1.Data2</data2>
                    </child1>
                    <child2>
                        <data1>Element2.Child2.Data1</data1>
                        <data2>Element2.Child2.Data2</data2>
                    </child2>
                </element2>
                <element3>
                    <child1>
                        <data1>Element3.Child1.Data1</data1>
                        <data2>Element3.Child1.Data2</data2>
                    </child1>
                    <child2>
                        <data1>Element3.Child2.Data1</data1>
                        <data2>Element3.Child2.Data2</data2>
                    </child2>
                </element3>
            </root>
        "##,
        )
        .unwrap()
    }
}
