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

use std::io::Error as IoError;

use base64::DecodeError as Base64Error;
use libxml::Error as XmlError;
use openssl::error::ErrorStack as SslError;
use thiserror::Error;

use crate::{DataType, DataTypes};

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO Error: {0}")]
    IoError(IoError),

    #[error("XML Error: {0}")]
    XmlError(XmlError),

    #[error("SSL Error: {0}")]
    SslError(SslError),

    #[error("SSL Error: {0}")]
    Base64Error(Base64Error),

    #[error("Unable to find suitable key!")]
    NoKey,

    #[error("Unable to find signature node!")]
    SignatureNodeNotFound,

    #[error("Invalid signature node: {0}!")]
    InvalidSignatureNode(String),

    #[error("Transform Chain Builder is empty!")]
    EmptyTransformChainBuilder,

    #[error("Transform Chain has unexpected end!")]
    UnexpectedEndOfChain,

    #[error("Invalid Signature Value!")]
    InvalidSignatureValue,

    #[error("Unexpected Data Type: {0:?}!")]
    UnexpectedDataType(DataType),

    #[error("Invalid Data Type (actual={actual:?}, expected={expected:?})!")]
    InvalidDataType {
        actual: Option<DataType>,
        expected: DataTypes,
    },

    #[error("Invalid Digest Value (actual={actual}, expected={expected})!")]
    InvalidDigistValue { actual: String, expected: String },

    #[error("Unknown Canonization Method: {0}!")]
    UnknownCanonizationMethod(String),

    #[error("Unknown Signature Method: {0}!")]
    UnknownSignatureMethod(String),

    #[error("Unknown Transformation: {0}!")]
    UnknownTransformation(String),

    #[error("Unknown Digest Method: {0}!")]
    UnknownDigestMethod(String),

    #[error("Unable to get node for xpath: {0:?}!")]
    UnableToGetNodeForXPath(Option<String>),
}

impl From<IoError> for Error {
    fn from(err: IoError) -> Self {
        Self::IoError(err)
    }
}

impl From<XmlError> for Error {
    fn from(err: XmlError) -> Self {
        Self::XmlError(err)
    }
}

impl From<SslError> for Error {
    fn from(err: SslError) -> Self {
        Self::SslError(err)
    }
}

impl From<Base64Error> for Error {
    fn from(err: Base64Error) -> Self {
        Self::Base64Error(err)
    }
}
