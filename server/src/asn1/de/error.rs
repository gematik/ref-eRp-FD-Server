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

use std::fmt::Display;

use asn1_der::error::Asn1DerError;
use serde::de::Error as DeError;
use thiserror::Error;

use super::super::types::ObjectIdentifier;

#[derive(Error, Debug)]
pub enum Error {
    #[error("ASN.1 Error: {0}")]
    Asn1Error(Asn1DerError),

    #[error("String does not contain a single character (offset=0x{0:08X})!")]
    InvalidChar(usize),

    #[error("Invalid Tag: {0}!")]
    InvalidTag(String),

    #[error("DER Object was already consumed!")]
    DerObjectAlreadyConsumed,

    #[error("Enum Variant without Name!")]
    EnumVariantWithoutName,

    #[error("Enum Variant with OID already exists!")]
    EnumVariantWithOidAlreadyExists,

    #[error("Enum Variant with Tag already exists!")]
    EnumVariantWithTagAlreadyExists,

    #[error("Invalid UTF-8 String (offset=0x{0:08X})!")]
    InvalidUtf8String(usize),

    #[error("DER Object is not a valid String object (offset=0x{0:08X})!")]
    InvalidStringObject(usize),

    #[error("DER Object is not a valid Buffer object (offset=0x{0:08X})!")]
    InvalidBufferObject(usize),

    #[error("Unknown Enum Variant (offset=0x{0:08X})!")]
    UnknownEnumVariant(usize),

    #[error("Unknown Enum Variant (offset=0x{0:08X}, oid={1})!")]
    UnknownEnumVariantOid(usize, ObjectIdentifier),

    #[error("Unknown Enum Variant (offset=0x{0:08X}, tag={1})!")]
    UnknownEnumVariantTag(usize, u8),

    #[error("Sequence to small (offset=0x{0:08X})!")]
    SequenceToSmall(usize),

    #[error("Invalid Object Identifier ({0})!")]
    InvalidObjectIdentifier(String),

    #[error("Unexpected Tag (offset=0x{offset:X}, expected={expected:?}, actual={actual}!")]
    UnexpectedTag {
        offset: usize,
        expected: Vec<u8>,
        actual: u8,
    },

    #[error(
        "Unexpected Object Identifier (offset=0x{offset:X}, expected={expected}, actual={actual}!"
    )]
    UnexpectedObjectIdentifier {
        offset: usize,
        expected: ObjectIdentifier,
        actual: ObjectIdentifier,
    },

    #[error("Not Supported: {0}")]
    NotSupported(String),

    #[error("Custom Error: {0}")]
    Custom(String),
}

impl DeError for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Self::Custom(format!("{}", msg))
    }
}

impl From<Asn1DerError> for Error {
    fn from(v: Asn1DerError) -> Self {
        Self::Asn1Error(v)
    }
}
