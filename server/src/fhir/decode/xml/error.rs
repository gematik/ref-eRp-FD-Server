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

use std::fmt::{Debug, Display};
use std::str::Utf8Error;

use thiserror::Error;

use super::{super::byte_stream::StreamError, reader::Element};

#[derive(Error, Debug)]
pub enum Error<E>
where
    E: Display + Debug,
{
    #[error("{0}")]
    Stream(StreamError<E>),

    #[error("UTF-8 Error: {0}")]
    Utf8Error(Utf8Error),

    #[error("Invalid Tag: {0}!")]
    InvalidTag(String),

    #[error("Invalid Close Tag (expected={0}, actual={1})!")]
    InvalidCloseTag(String, String),

    #[error("Unexpected Text: {0}!")]
    UnexpectedText(String),

    #[error("Invalid Value Extension!")]
    InvalidValueExtension,

    #[error("Expected EOF!")]
    ExpectedEof,

    #[error("Missing XMLNS!")]
    MissingXmlns,

    #[error("Invalid XMLNS!")]
    InvalidXmlns,

    #[error("Unexpected Element: {0:?}!")]
    UnexpectedElement(Element),
}

impl<E> From<StreamError<E>> for Error<E>
where
    E: Display + Debug,
{
    fn from(err: StreamError<E>) -> Self {
        Self::Stream(err)
    }
}

impl<E> From<Utf8Error> for Error<E>
where
    E: Display + Debug,
{
    fn from(err: Utf8Error) -> Self {
        Self::Utf8Error(err)
    }
}
