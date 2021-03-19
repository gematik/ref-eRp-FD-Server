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

#![allow(non_upper_case_globals)]

mod base64;
mod c14n;
mod digest_value;
mod enveloped_signature;
mod hash;
mod select_node;
mod signature_value;

use std::borrow::Borrow;

use bytes::Bytes;
use libxml::NodeRef;

use super::{Error, NodeSetLike};

pub use self::base64::*;
pub use c14n::*;
pub use digest_value::*;
pub use enveloped_signature::*;
pub use hash::*;
pub use select_node::*;
pub use signature_value::*;

pub enum Data<'a> {
    Xml(&'a NodeRef, &'a dyn NodeSetLike),
    Binary(Bytes),
    BinaryRaw(&'a [u8]),
    Base64(String),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum DataType {
    Xml,
    Binary,
    Base64,
}

bitflags! {
    pub struct DataTypes: u32 {
        const Xml = 0b00000001;
        const Binary = 0b00000010;
        const Base64 = 0b00000100;

        const None = 0;
        const Any = Self::Xml.bits | Self::Binary.bits | Self::Base64.bits;
    }
}

impl DataTypes {
    pub fn has(&self, data_type: &DataType) -> bool {
        match data_type {
            DataType::Xml => self.contains(DataTypes::Xml),
            DataType::Binary => self.contains(DataTypes::Binary),
            DataType::Base64 => self.contains(DataTypes::Base64),
        }
    }
}

pub trait Transform {
    fn name(&self) -> &str;
    fn next(&self) -> Option<&dyn Transform>;
    fn update(&mut self, data: Data) -> Result<(), Error>;
    fn finish(self: Box<Self>) -> Result<(), Error>;
}

pub trait TransformBuilder<'a> {
    fn input_types(&self) -> DataTypes;

    fn output_type(&self) -> Option<DataType> {
        None
    }

    fn build(
        self: Box<Self>,
        next: Option<Box<dyn Transform + 'a>>,
    ) -> Result<Box<dyn Transform + 'a>, Error>;
}

#[derive(Default)]
pub struct ChainBuilder<'a> {
    items: Vec<Box<dyn TransformBuilder<'a> + 'a>>,
}

impl<'a> ChainBuilder<'a> {
    pub fn append<T: TransformBuilder<'a> + 'a>(&mut self, builder: T) -> &mut Self {
        self.items.push(Box::new(builder));

        self
    }

    pub fn items(&self) -> &Vec<Box<dyn TransformBuilder<'a> + 'a>> {
        &self.items
    }

    pub fn build(self) -> Result<Box<dyn Transform + 'a>, Error> {
        let mut ret = None;
        let mut last_input_type: Option<DataTypes> = None;

        for item in self.items.into_iter().rev() {
            match (last_input_type, item.output_type()) {
                (Some(input), Some(output)) if !input.has(&output) => {
                    return Err(Error::InvalidDataType {
                        expected: input,
                        actual: Some(output),
                    });
                }
                (Some(_), Some(_)) => (),
                (Some(input), None) => {
                    return Err(Error::InvalidDataType {
                        expected: input,
                        actual: None,
                    });
                }
                (None, Some(_)) => (),
                (None, None) => (),
            }

            last_input_type = Some(item.input_types());

            ret = Some(item.build(ret)?);
        }

        ret.ok_or(Error::EmptyTransformChainBuilder)
    }
}

impl<'a, T> From<T> for DataType
where
    T: Borrow<Data<'a>>,
{
    fn from(data: T) -> DataType {
        match data.borrow() {
            Data::Xml(_, _) => DataType::Xml,
            Data::Binary(_) => DataType::Binary,
            Data::BinaryRaw(_) => DataType::Binary,
            Data::Base64(_) => DataType::Base64,
        }
    }
}
