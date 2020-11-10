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

use std::str::Utf8Error;

use thiserror::Error;

use super::super::item::Item;

#[derive(Error, Debug)]
pub enum Error {
    #[error("UTF8 Error: {0:?}!")]
    Utf8Error(Utf8Error),

    #[error("Unexpected Item: {0:?}!")]
    UnexpectedItem(Item),

    #[error("Unexpected EOF!")]
    UnexpectedEof,

    #[error("Expected EOF!")]
    ExpectedEof,

    #[error("Invalid Extension!")]
    InvalidExtension,
}

impl From<Utf8Error> for Error {
    fn from(err: Utf8Error) -> Self {
        Self::Utf8Error(err)
    }
}
