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

use base64::decode;
use openssl::{
    hash::MessageDigest,
    pkey::{HasPublic, PKeyRef},
    rsa::Padding,
    sign::RsaPssSaltlen,
    sign::Verifier,
};

use crate::Error;

use super::{Data, DataTypes, Transform, TransformBuilder};

pub enum SignatureMethod {
    RsaSha1,
    RsaMgfSha256,
}

/* SignatureValue */

pub struct SignatureValue<'a, T>
where
    T: HasPublic,
{
    key: &'a PKeyRef<T>,
    method: SignatureMethod,
    signature: Data<'a>,
}

impl<'a, T> SignatureValue<'a, T>
where
    T: HasPublic,
{
    pub fn new(key: &'a PKeyRef<T>, method: SignatureMethod, signature: Data<'a>) -> Self {
        Self {
            key,
            method,
            signature,
        }
    }
}

impl<'a, T> TransformBuilder<'a> for SignatureValue<'a, T>
where
    T: HasPublic,
{
    fn input_types(&self) -> DataTypes {
        DataTypes::Binary
    }

    fn build(
        self: Box<Self>,
        next: Option<Box<dyn Transform + 'a>>,
    ) -> Result<Box<dyn Transform + 'a>, Error> {
        let Self {
            key,
            method,
            signature,
        } = *self;

        let verifier = match method {
            SignatureMethod::RsaSha1 => Verifier::new(MessageDigest::sha1(), key)?,
            SignatureMethod::RsaMgfSha256 => {
                let mut verifier = Verifier::new(MessageDigest::sha256(), key)?;
                verifier.set_rsa_padding(Padding::PKCS1_PSS)?;
                verifier.set_rsa_mgf1_md(MessageDigest::sha256())?;
                verifier.set_rsa_pss_saltlen(RsaPssSaltlen::MAXIMUM_LENGTH)?;

                verifier
            }
        };

        Ok(Box::new(SignatureValueTransform {
            next,
            verifier,
            signature,
        }))
    }
}

/* SignatureValueTransform */

struct SignatureValueTransform<'a> {
    next: Option<Box<dyn Transform + 'a>>,
    verifier: Verifier<'a>,
    signature: Data<'a>,
}

impl<'a> Transform for SignatureValueTransform<'a> {
    fn name(&self) -> &str {
        "expect_data_transform"
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

        self.verifier.update(&data)?;

        Ok(())
    }

    fn finish(self: Box<Self>) -> Result<(), Error> {
        #[allow(unused_assignments)]
        let mut buf = Vec::new();

        let signature = match &self.signature {
            Data::Binary(data) => data.as_ref(),
            Data::BinaryRaw(data) => data,
            Data::Base64(data) => {
                buf = decode(data)?;

                &buf
            }
            x => return Err(Error::UnexpectedDataType(x.into())),
        };

        let is_valid = self.verifier.verify(signature)?;

        if is_valid {
            Ok(())
        } else {
            Err(Error::InvalidSignatureValue)
        }
    }
}
