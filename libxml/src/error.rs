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

use std::ffi::CStr;
use std::path::PathBuf;
use std::str::Utf8Error;

use libc::*;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Error ({code}) in {file}:{line} - {msg}")]
    XmlError {
        code: usize,
        file: String,
        line: usize,
        msg: String,
    },

    #[error("Object returned by libxml was null: {0}!")]
    NullPointer(String),

    #[error("Unknown error while transforming to C14N!")]
    C14nFailed,

    #[error("Unable to get document!")]
    UnableToGetDoc,

    #[error("Unable to get node content!")]
    UnableToGetNodeContent,

    #[error("UTF-8 Error: {0}")]
    Utf8Error(Utf8Error),

    #[error("Invalid XPath: {0}!")]
    InvalidXPath(String),

    #[error("Invalid Filepath: {0}!")]
    InvalidFilepath(PathBuf),

    #[error("Unknown Error!")]
    Unknown,
}

impl Error {
    pub fn check_ptr<T, S>(ptr: *const T, context: S) -> Result<*const T, Self>
    where
        S: Into<String>,
    {
        if !ptr.is_null() {
            Ok(ptr)
        } else {
            Err(Self::last_error(Self::NullPointer(context.into())))
        }
    }

    pub fn check_ptr_mut<T, S>(ptr: *mut T, context: S) -> Result<*mut T, Self>
    where
        S: Into<String>,
    {
        if !ptr.is_null() {
            Ok(ptr)
        } else {
            Err(Self::last_error(Self::NullPointer(context.into())))
        }
    }

    pub fn last_error(default: Self) -> Self {
        unsafe {
            let err = ffi::xmlGetLastError();
            if err.is_null() {
                return default;
            }

            let err = &*err;
            let code = err.code as usize;
            let file = cstr_to_string(err.file, "<unknown>");
            let line = err.line as usize;
            let msg = cstr_to_string(err.message, "<unknown>");

            Self::XmlError {
                code,
                file,
                line,
                msg,
            }
        }
    }
}

impl From<Utf8Error> for Error {
    fn from(err: Utf8Error) -> Self {
        Self::Utf8Error(err)
    }
}

fn cstr_to_string(ptr: *const c_char, default: &str) -> String {
    if ptr.is_null() {
        return default.to_owned();
    }

    match unsafe { CStr::from_ptr(ptr) }.to_str() {
        Ok(s) => s.to_owned(),
        Err(_) => default.to_owned(),
    }
}
