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

use std::ops::Deref;

use libxml::{NodeRef, NodeSet as XmlNodeSet};

use super::Error;

pub trait NodeSetLike {
    fn contains(&self, node: &NodeRef, parent: Option<&NodeRef>) -> bool;
}

impl<'a, T> NodeSetLike for &'a T
where
    T: NodeSetLike + ?Sized,
{
    fn contains(&self, node: &NodeRef, parent: Option<&NodeRef>) -> bool {
        (*self).contains(node, parent)
    }
}

impl<T> NodeSetLike for Box<T>
where
    T: NodeSetLike + ?Sized,
{
    fn contains(&self, node: &NodeRef, parent: Option<&NodeRef>) -> bool {
        self.deref().contains(node, parent)
    }
}

/* NodeSetOp */

pub trait NodeSetOps: NodeSetLike + Sized {
    fn invert(self) -> NodeSetInvert<Self> {
        NodeSetInvert::new(self)
    }

    fn union<T>(self, other: T) -> NodeSetUnion<Self, T> {
        NodeSetUnion::new(self, other)
    }

    fn intersect<T>(self, other: T) -> NodeSetIntersect<Self, T> {
        NodeSetIntersect::new(self, other)
    }

    fn complement<T>(self, other: T) -> NodeSetComplement<Self, T> {
        NodeSetComplement::new(self, other)
    }
}

impl<T> NodeSetOps for T where T: NodeSetLike {}

/* NodeSet */

pub struct NodeSet<'a> {
    node_set: XmlNodeSet<'a>,
}

impl<'a> NodeSet<'a> {
    pub fn from_node(node: &'a NodeRef) -> Result<Self, Error> {
        let node_set = XmlNodeSet::from_node(node)?;

        Ok(Self { node_set })
    }

    fn check(&self, node: &NodeRef, parent: Option<&NodeRef>) -> bool {
        if self.node_set.contains(node) {
            return true;
        }

        if let Some(parent) = parent {
            return self.check(parent, parent.parent());
        }

        false
    }
}

impl NodeSetLike for NodeSet<'_> {
    fn contains(&self, node: &NodeRef, parent: Option<&NodeRef>) -> bool {
        self.check(node, parent)
    }
}

/* NodeSetAll */

pub struct NodeSetAll;

impl NodeSetLike for NodeSetAll {
    fn contains(&self, _node: &NodeRef, _parent: Option<&NodeRef>) -> bool {
        true
    }
}

/* NodeSetNone */

pub struct NodeSetNone;

impl NodeSetLike for NodeSetNone {
    fn contains(&self, _node: &NodeRef, _parent: Option<&NodeRef>) -> bool {
        false
    }
}

/* NodeSetInvert */

pub struct NodeSetInvert<T> {
    inner: T,
}

impl<T> NodeSetInvert<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T> NodeSetLike for NodeSetInvert<T>
where
    T: NodeSetLike,
{
    fn contains(&self, node: &NodeRef, parent: Option<&NodeRef>) -> bool {
        !self.inner.contains(node, parent)
    }
}

/* NodeSetUnion */

pub struct NodeSetUnion<A, B> {
    a: A,
    b: B,
}

impl<A, B> NodeSetUnion<A, B> {
    pub fn new(a: A, b: B) -> Self {
        Self { a, b }
    }
}

impl<A, B> NodeSetLike for NodeSetUnion<A, B>
where
    A: NodeSetLike,
    B: NodeSetLike,
{
    fn contains(&self, node: &NodeRef, parent: Option<&NodeRef>) -> bool {
        self.a.contains(node, parent) || self.b.contains(node, parent)
    }
}

/* NodeSetIntersect */

pub struct NodeSetIntersect<A, B> {
    a: A,
    b: B,
}

impl<A, B> NodeSetIntersect<A, B> {
    pub fn new(a: A, b: B) -> Self {
        Self { a, b }
    }
}

impl<A, B> NodeSetLike for NodeSetIntersect<A, B>
where
    A: NodeSetLike,
    B: NodeSetLike,
{
    fn contains(&self, node: &NodeRef, parent: Option<&NodeRef>) -> bool {
        self.a.contains(node, parent) && self.b.contains(node, parent)
    }
}

/* NodeSetComplement */

pub struct NodeSetComplement<A, B> {
    a: A,
    b: B,
}

impl<A, B> NodeSetComplement<A, B> {
    pub fn new(a: A, b: B) -> Self {
        Self { a, b }
    }
}

impl<A, B> NodeSetLike for NodeSetComplement<A, B>
where
    A: NodeSetLike,
    B: NodeSetLike,
{
    fn contains(&self, node: &NodeRef, parent: Option<&NodeRef>) -> bool {
        if self.b.contains(node, parent) {
            false
        } else {
            self.a.contains(node, parent)
        }
    }
}
