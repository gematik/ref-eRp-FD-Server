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

use std::io::Write;

use base64::encode;

use crate::Error;

use super::{Data, DataType, DataTypes, Transform, TransformBuilder};

/* Hash */

pub struct Base64;

impl<'a> TransformBuilder<'a> for Base64 {
    fn input_types(&self) -> DataTypes {
        DataTypes::Binary
    }

    fn output_type(&self) -> Option<DataType> {
        Some(DataType::Base64)
    }

    fn build(
        self: Box<Self>,
        next: Option<Box<dyn Transform + 'a>>,
    ) -> Result<Box<dyn Transform + 'a>, Error> {
        Ok(Box::new(Base64Transform {
            next,
            data: Vec::new(),
        }))
    }
}

/* Base64Transform */

struct Base64Transform<'a> {
    next: Option<Box<dyn Transform + 'a>>,
    data: Vec<u8>,
}

impl<'a> Transform for Base64Transform<'a> {
    fn name(&self) -> &str {
        "base64_transform"
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

        self.data.write_all(data)?;

        Ok(())
    }

    fn finish(self: Box<Self>) -> Result<(), Error> {
        let data = encode(&self.data);
        let data = Data::Base64(data);

        let mut next = self.next.ok_or(Error::UnexpectedEndOfChain)?;

        next.update(data)?;

        next.finish()
    }
}
