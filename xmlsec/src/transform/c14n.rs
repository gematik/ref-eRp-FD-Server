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

use std::io::{Error as IoError, ErrorKind, Write};

use libxml::{C14nMode, ElementType, NodeRef, NodeVisibility, OutputBuffer};

use crate::{Error, NodeSet, NodeSetLike, NodeSetOps};

use super::{Data, DataType, DataTypes, Transform, TransformBuilder};

/* C14nMethod */

#[allow(non_camel_case_types)]
pub enum C14nMethod {
    C14n_1_0,
    C14n_Exclusive_1_0,
}

/* C14n */

pub struct C14n {
    method: C14nMethod,
}

impl C14n {
    pub fn new(method: C14nMethod) -> Self {
        Self { method }
    }
}

impl<'a> TransformBuilder<'a> for C14n {
    fn input_types(&self) -> DataTypes {
        DataTypes::Xml
    }

    fn output_type(&self) -> Option<DataType> {
        Some(DataType::Binary)
    }

    fn build(
        self: Box<Self>,
        next: Option<Box<dyn Transform + 'a>>,
    ) -> Result<Box<dyn Transform + 'a>, Error> {
        let Self { method } = *self;

        Ok(Box::new(C14nTransform { next, method }))
    }
}

/* C14nTransform */

struct C14nTransform<'a> {
    next: Option<Box<dyn Transform + 'a>>,
    method: C14nMethod,
}

struct Writer<'a>(&'a mut dyn Transform);
struct Visibility<'a> {
    set: &'a dyn NodeSetLike,
    with_comments: bool,
}

impl<'a> Transform for C14nTransform<'a> {
    fn name(&self) -> &str {
        "c14n_transform"
    }

    fn next(&self) -> Option<&dyn Transform> {
        self.next.as_deref()
    }

    fn update(&mut self, data: Data) -> Result<(), Error> {
        let (node, set) = match data {
            Data::Xml(node, set) => (node, set),
            x => return Err(Error::UnexpectedDataType(x.into())),
        };

        let (mode, with_comments) = match self.method {
            C14nMethod::C14n_1_0 => (C14nMode::XML_C14N_1_0, false),
            C14nMethod::C14n_Exclusive_1_0 => (C14nMode::XML_C14N_EXCLUSIVE_1_0, false),
        };

        let set = set.intersect(NodeSet::from_node(node)?);
        let visibility = Visibility {
            set: &set,
            with_comments,
        };

        let next = self.next.as_mut().ok_or(Error::UnexpectedEndOfChain)?;
        let mut writer = Writer(&mut **next);
        let buffer = OutputBuffer::new(&mut writer)?;

        node.doc()?
            .c14n(&visibility, mode, with_comments, None, &buffer)?;

        Ok(())
    }

    fn finish(self: Box<Self>) -> Result<(), Error> {
        self.next.ok_or(Error::UnexpectedEndOfChain)?.finish()
    }
}

impl Write for Writer<'_> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, IoError> {
        match self.0.update(Data::BinaryRaw(buf)) {
            Ok(()) => Ok(buf.len()),
            Err(err) => Err(IoError::new(ErrorKind::Other, err)),
        }
    }

    fn flush(&mut self) -> Result<(), IoError> {
        Ok(())
    }
}

impl Visibility<'_> {
    fn check(&self, node: &NodeRef, parent: Option<&NodeRef>) -> bool {
        if !self.with_comments && node.type_() == ElementType::XML_COMMENT_NODE {
            return false;
        }

        self.set.contains(node, parent)
    }
}

impl NodeVisibility for Visibility<'_> {
    fn is_visible(&self, node: &NodeRef, parent: Option<&NodeRef>) -> bool {
        self.check(node, parent)
    }
}
