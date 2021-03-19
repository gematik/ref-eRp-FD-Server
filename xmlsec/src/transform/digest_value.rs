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

use base64::encode;

use crate::Error;

use super::{Data, DataTypes, Transform, TransformBuilder};

/* DigestValue */

pub struct DigestValue {
    data: Data<'static>,
}

impl DigestValue {
    pub fn new(data: Data<'static>) -> Self {
        Self { data }
    }
}

impl<'a> TransformBuilder<'a> for DigestValue {
    fn input_types(&self) -> DataTypes {
        DataTypes::Binary | DataTypes::Base64
    }

    fn build(
        self: Box<Self>,
        next: Option<Box<dyn Transform + 'a>>,
    ) -> Result<Box<dyn Transform + 'a>, Error> {
        let Self { data } = *self;

        Ok(Box::new(DigestValueTransform {
            next,
            actual: None,
            expected: data,
        }))
    }
}

/* DigestValueTransform */

enum Buffer {
    Base64(String),
    Binary(Vec<u8>),
}

struct DigestValueTransform<'a> {
    next: Option<Box<dyn Transform + 'a>>,
    actual: Option<Buffer>,
    expected: Data<'static>,
}

impl<'a> Transform for DigestValueTransform<'a> {
    fn name(&self) -> &str {
        "expect_data_transform"
    }

    fn next(&self) -> Option<&dyn Transform> {
        self.next.as_deref()
    }

    fn update(&mut self, data: Data) -> Result<(), Error> {
        self.actual = Some(match (data, self.actual.take()) {
            (Data::Base64(data), None) => Buffer::Base64(data),
            (Data::Binary(data), None) => Buffer::Binary(data.to_vec()),
            (Data::BinaryRaw(data), None) => Buffer::Binary(data.into()),

            (Data::Base64(data), Some(Buffer::Base64(buffer))) => Buffer::Base64(buffer + &data),
            (Data::Binary(data), Some(Buffer::Binary(mut buffer))) => {
                buffer.extend_from_slice(&data);

                Buffer::Binary(buffer)
            }
            (Data::BinaryRaw(data), Some(Buffer::Binary(mut buffer))) => {
                buffer.extend_from_slice(&data);

                Buffer::Binary(buffer)
            }

            (x, _) => return Err(Error::UnexpectedDataType(x.into())),
        });

        Ok(())
    }

    fn finish(self: Box<Self>) -> Result<(), Error> {
        let expected = match self.expected {
            Data::Base64(v) => v,
            Data::Binary(v) => encode(v),
            Data::BinaryRaw(v) => encode(v),
            x => return Err(Error::UnexpectedDataType(x.into())),
        };

        let actual = match self.actual {
            None => String::new(),
            Some(Buffer::Base64(v)) => v,
            Some(Buffer::Binary(v)) => encode(&v),
        };

        if actual == expected {
            Ok(())
        } else {
            Err(Error::InvalidDigistValue { actual, expected })
        }
    }
}
