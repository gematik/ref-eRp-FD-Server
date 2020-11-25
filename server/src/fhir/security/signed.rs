/*
 * Copyright (c) 2020 gematik GmbH
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
use std::str::{from_utf8, Utf8Error};

use chrono::Utc;
use miscellaneous::jwt::{sign, Error as JwsError};
use openssl::{
    pkey::{PKey, Private},
    x509::X509,
};
use resources::{Signature, SignatureFormat, SignatureType, WithSignature};
use serde_json::Error as SerdeJsonError;
use thiserror::Error;

use crate::fhir::encode::{Encode, EncodeError, JsonEncode, JsonError};

use super::canonize_json;

pub struct Signed<T>(T);

#[derive(Error, Debug)]
pub enum Error {
    #[error("UTF-8 Error: {0}")]
    Utf8Error(Utf8Error),

    #[error("JSON Error: {0}")]
    JsonError(SerdeJsonError),

    #[error("JSON Encode Error: {0}")]
    JsonEncodeError(EncodeError<JsonError>),

    #[error("JWS Error: {0}")]
    JwsError(JwsError),
}

impl<T> Signed<T> {
    pub fn new(data: T) -> Self {
        Self(data)
    }
}

impl<'e, T> Signed<T>
where
    T: WithSignature + 'e,
    &'e T: Encode,
{
    pub fn sign_json(
        &'e mut self,
        type_: SignatureType,
        who: String,
        sig_key: &PKey<Private>,
        sig_cert: &X509,
    ) -> Result<(), Error> {
        let mut buf = Vec::<u8>::new();

        let data = &self.0 as *const T;

        // Without the unsafe block, data would be borrowed for 'e until the end of the function.
        // If it's still borrowed, we could not update the signatures. So we use an unsafe block
        // here to avoid the borrowing. No worries, the code is still safe, because the 'json'
        // method returns no references.
        let json = unsafe { (&*data).json()? };
        let json = from_utf8(&json)?;
        canonize_json(json, &mut buf)?;

        let data = sign(&buf, sig_key.clone(), Some(sig_cert), true)?;
        let signatures = self.0.signatures_mut();
        signatures.retain(|sig| sig.type_ != type_ && sig.format != Some(SignatureFormat::Json));
        signatures.push(Signature {
            type_,
            who,
            when: Utc::now().into(),
            data,
            format: Some(SignatureFormat::Json),
        });

        Ok(())
    }
}

impl<T> Deref for Signed<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Utf8Error> for Error {
    fn from(err: Utf8Error) -> Self {
        Self::Utf8Error(err)
    }
}

impl From<SerdeJsonError> for Error {
    fn from(err: SerdeJsonError) -> Self {
        Self::JsonError(err)
    }
}

impl From<EncodeError<JsonError>> for Error {
    fn from(err: EncodeError<JsonError>) -> Self {
        Self::JsonEncodeError(err)
    }
}

impl From<JwsError> for Error {
    fn from(err: JwsError) -> Self {
        Self::JwsError(err)
    }
}
