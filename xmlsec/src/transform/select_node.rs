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

use crate::Error;

use super::{Data, DataType, DataTypes, Transform, TransformBuilder};

/* SelectNode */

pub struct SelectNode {
    uri: Option<String>,
}

impl SelectNode {
    pub fn new(uri: Option<String>) -> Self {
        Self { uri }
    }
}

impl<'a> TransformBuilder<'a> for SelectNode {
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
        Ok(Box::new(SelectNodeTransform {
            uri: self.uri,
            next,
        }))
    }
}

/* SelectNodeTransform */

struct SelectNodeTransform<'a> {
    uri: Option<String>,
    next: Option<Box<dyn Transform + 'a>>,
}

impl<'a> Transform for SelectNodeTransform<'a> {
    fn name(&self) -> &str {
        "select_node_transform"
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

        let uri = &self.uri;
        let node = node
            .xpath(uri.as_deref())?
            .ok_or_else(|| Error::UnableToGetNodeForXPath(uri.clone()))?;

        let data = Data::Xml(node, set);

        next.update(data)
    }

    fn finish(self: Box<Self>) -> Result<(), Error> {
        self.next.ok_or(Error::UnexpectedEndOfChain)?.finish()
    }
}
