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

use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::ops::Deref;
use std::str::{from_utf8_unchecked, Utf8Error};

use base64::encode;
use chrono::Utc;
use miscellaneous::jwt::{sign, Error as JwsError};
use openssl::{
    cms::{CMSOptions, CmsContentInfo},
    error::ErrorStack as OpenSslError,
    pkey::{PKey, Private},
    x509::X509,
};
use resources::{Signature, SignatureFormat, SignatureType, WithSignature};
use serde_json::Error as SerdeJsonError;
use thiserror::Error;

use crate::fhir::encode::{Encode, EncodeError, JsonEncode, JsonError, XmlEncode, XmlError};

use super::canonize_json;

pub struct Signed<T>(T);

impl<T> Debug for Signed<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_tuple("Signed").field(&self.0).finish()
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("UTF-8 Error: {0}")]
    Utf8Error(Utf8Error),

    #[error("JSON Error: {0}")]
    JsonError(SerdeJsonError),

    #[error("JSON Encode Error: {0}")]
    JsonEncodeError(EncodeError<JsonError>),

    #[error("XML Encode Error: {0}")]
    XmlEncodeError(EncodeError<XmlError>),

    #[error("JWS Error: {0}")]
    JwsError(JwsError),

    #[error("OpenSSL Error: {0}")]
    OpenSslError(OpenSslError),
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
        let json = unsafe { from_utf8_unchecked(&json) };
        canonize_json(json, &mut buf)?;

        let data = sign::<()>(sig_key.clone(), Some(sig_cert), None, &buf, true)?;
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

    pub fn sign_cades(
        &'e mut self,
        type_: SignatureType,
        who: String,
        sig_key: &PKey<Private>,
        sig_cert: &X509,
    ) -> Result<(), Error> {
        let data = &self.0 as *const T;

        #[cfg(openssl300)]
        lazy_static! {
            static ref FLAGS: CMSOptions = CMSOptions::BINARY | CMSOptions::CADES;
        }

        #[cfg(not(openssl300))]
        lazy_static! {
            static ref FLAGS: CMSOptions = CMSOptions::BINARY;
        }

        // Without the unsafe block, data would be borrowed for 'e until the end of the function.
        // If it's still borrowed, we could not update the signatures. So we use an unsafe block
        // here to avoid the borrowing. No worries, the code is still safe, because the 'xml'
        // method returns no references.
        let xml = unsafe { (&*data).xml()? };
        let cms = CmsContentInfo::sign(Some(sig_cert), Some(sig_key), None, Some(&xml), *FLAGS)?;

        let data = cms.to_der()?;
        let data = encode(&data);

        let signatures = self.0.signatures_mut();
        signatures.retain(|sig| sig.type_ != type_ && sig.format != Some(SignatureFormat::Xml));
        signatures.push(Signature {
            type_,
            who,
            when: Utc::now().into(),
            data,
            format: Some(SignatureFormat::Xml),
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

impl From<EncodeError<XmlError>> for Error {
    fn from(err: EncodeError<XmlError>) -> Self {
        Self::XmlEncodeError(err)
    }
}

impl From<JwsError> for Error {
    fn from(err: JwsError) -> Self {
        Self::JwsError(err)
    }
}

impl From<OpenSslError> for Error {
    fn from(err: OpenSslError) -> Self {
        Self::OpenSslError(err)
    }
}
