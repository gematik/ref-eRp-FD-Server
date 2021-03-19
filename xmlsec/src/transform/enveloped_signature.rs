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

use libxml::NodeRef;

use crate::{node_matches, Error, NodeSet, NodeSetOps, NAMESPACE_HREF, NODE_SIGNATURE};

use super::{Data, DataType, DataTypes, Transform, TransformBuilder};

/* EnvelopedSignature */

pub struct EnvelopedSignature<'a> {
    node: &'a NodeRef,
}

impl<'a> EnvelopedSignature<'a> {
    pub fn new(node: &'a NodeRef) -> Self {
        Self { node }
    }
}

impl<'a> TransformBuilder<'a> for EnvelopedSignature<'a> {
    fn input_types(&self) -> DataTypes {
        DataTypes::Xml
    }

    fn output_type(&self) -> Option<DataType> {
        Some(DataType::Xml)
    }

    fn build(
        self: Box<Self>,
        next: Option<Box<dyn Transform + 'a>>,
    ) -> Result<Box<dyn Transform + 'a>, Error> {
        let signature_node = self
            .node
            .search_parent(|n| node_matches(n, NODE_SIGNATURE, NAMESPACE_HREF))
            .ok_or(Error::SignatureNodeNotFound)?;
        let nodes = NodeSet::from_node(signature_node)?;

        Ok(Box::new(EnvelopedSignatureTransform { next, nodes }))
    }
}

/* EnvelopedSignatureTransform */

struct EnvelopedSignatureTransform<'a> {
    next: Option<Box<dyn Transform + 'a>>,
    nodes: NodeSet<'a>,
}

impl<'a> Transform for EnvelopedSignatureTransform<'a> {
    fn name(&self) -> &str {
        "enveloped_signature"
    }

    fn next(&self) -> Option<&dyn Transform> {
        self.next.as_deref()
    }

    fn update(&mut self, data: Data) -> Result<(), Error> {
        let next = self.next.as_mut().ok_or(Error::UnexpectedEndOfChain)?;
        let (node, set) = match data {
            Data::Xml(node, set) => (node, set),
            x => return Err(Error::UnexpectedDataType(x.into())),
        };

        let set = set.complement(&self.nodes);
        let data = Data::Xml(node, &set);

        next.update(data)
    }

    fn finish(self: Box<Self>) -> Result<(), Error> {
        self.next.ok_or(Error::UnexpectedEndOfChain)?.finish()
    }
}
