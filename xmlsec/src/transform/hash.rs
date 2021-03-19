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

use openssl::hash::{Hasher, MessageDigest};

use crate::Error;

use super::{Data, DataType, DataTypes, Transform, TransformBuilder};

/* HashMethod */

#[allow(non_camel_case_types)]
pub enum HashMethod {
    Sha1,
    Sha256,
}

impl Into<MessageDigest> for HashMethod {
    fn into(self) -> MessageDigest {
        match self {
            HashMethod::Sha1 => MessageDigest::sha1(),
            HashMethod::Sha256 => MessageDigest::sha256(),
        }
    }
}

/* Hash */

pub struct Hash {
    method: HashMethod,
}

impl Hash {
    pub fn new(method: HashMethod) -> Self {
        Self { method }
    }
}

impl<'a> TransformBuilder<'a> for Hash {
    fn input_types(&self) -> DataTypes {
        DataTypes::Binary
    }

    fn output_type(&self) -> Option<DataType> {
        Some(DataType::Binary)
    }

    fn build(
        self: Box<Self>,
        next: Option<Box<dyn Transform + 'a>>,
    ) -> Result<Box<dyn Transform + 'a>, Error> {
        let Self { method } = *self;

        let hasher = Hasher::new(method.into())?;

        Ok(Box::new(HashTransform { next, hasher }))
    }
}

/* HashTransform */

struct HashTransform<'a> {
    next: Option<Box<dyn Transform + 'a>>,
    hasher: Hasher,
}

impl<'a> Transform for HashTransform<'a> {
    fn name(&self) -> &str {
        "hash_transform"
    }

    fn next(&self) -> Option<&dyn Transform> {
        self.next.as_deref()
    }

    fn update(&mut self, data: Data) -> Result<(), Error> {
        let data = match &data {
            Data::Binary(data) => data.as_ref(),
            Data::BinaryRaw(data) => data,
            x => return Err(Error::UnexpectedDataType(x.into())),
        };

        self.hasher.update(data)?;

        Ok(())
    }

    fn finish(mut self: Box<Self>) -> Result<(), Error> {
        let data = self.hasher.finish()?;
        let data = Data::BinaryRaw(data.as_ref());

        let mut next = self.next.ok_or(Error::UnexpectedEndOfChain)?;

        next.update(data)?;

        next.finish()
    }
}
