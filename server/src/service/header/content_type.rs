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

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::ops::Deref;
use std::str::FromStr;

use actix_web::{
    dev::Payload,
    error::ParseError,
    http::{
        header::{Header, HeaderMap, HeaderName, HeaderValue, IntoHeaderValue},
        Error,
    },
    FromRequest, HttpMessage, HttpRequest,
};
use futures::future::{err, ok, Ready};
use mime::Mime;

use crate::service::{RequestError, TypedRequestError};

lazy_static! {
    pub static ref CONTENT_TYPE: HeaderName = HeaderName::from_lowercase(b"content-type").unwrap();
}

pub struct ContentType(pub Mime);

impl Header for ContentType {
    #[inline]
    fn name() -> HeaderName {
        CONTENT_TYPE.clone()
    }

    #[inline]
    fn parse<T: HttpMessage>(msg: &T) -> Result<Self, ParseError> {
        parse_content_type(msg.headers())
            .ok_or(ParseError::Header)?
            .map_err(|_| ParseError::Header)
    }
}

impl IntoHeaderValue for ContentType {
    type Error = Error;

    fn try_into(self) -> Result<HeaderValue, Self::Error> {
        self.0.to_string().try_into().map_err(Into::into)
    }
}

impl Into<Mime> for ContentType {
    fn into(self) -> Mime {
        self.0
    }
}

impl Deref for ContentType {
    type Target = Mime;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for ContentType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

impl FromRequest for ContentType {
    type Error = TypedRequestError;
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        match parse_content_type(req.headers()) {
            Some(Ok(content_type)) => ok(content_type),
            Some(Err(())) => {
                err(RequestError::HeaderInvalid(Self::name().to_string()).with_type_from(req))
            }
            None => err(RequestError::HeaderMissing(Self::name().to_string()).with_type_from(req)),
        }
    }
}

fn parse_content_type(headers: &HeaderMap) -> Option<Result<ContentType, ()>> {
    let content_type = match headers.get(ContentType::name()) {
        Some(content_type) => content_type,
        None => return None,
    };

    let content_type = match content_type.to_str() {
        Ok(content_type) => content_type,
        Err(_) => return Some(Err(())),
    };

    let content_type = match Mime::from_str(content_type) {
        Ok(content_type) => content_type,
        Err(_) => return Some(Err(())),
    };

    Some(Ok(ContentType(content_type)))
}
