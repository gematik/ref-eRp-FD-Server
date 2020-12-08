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
use serde::Deserialize;
use serde_urlencoded::from_str;

use crate::service::{RequestError, TypedRequestError};

lazy_static! {
    pub static ref X_ACCESS_CODE: HeaderName = HeaderName::from_lowercase(b"x-accesscode").unwrap();
}

pub struct XAccessCode(pub String);

impl Header for XAccessCode {
    #[inline]
    fn name() -> HeaderName {
        X_ACCESS_CODE.clone()
    }

    #[inline]
    fn parse<T: HttpMessage>(msg: &T) -> Result<Self, ParseError> {
        parse_x_access_code(msg.headers())
            .ok_or(ParseError::Header)?
            .map_err(|_| ParseError::Header)
    }
}

impl IntoHeaderValue for XAccessCode {
    type Error = Error;

    fn try_into(self) -> Result<HeaderValue, Self::Error> {
        self.0.try_into().map_err(Into::into)
    }
}

impl FromRequest for XAccessCode {
    type Error = TypedRequestError;
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        #[derive(Deserialize)]
        struct Args {
            ac: Option<String>,
        }

        match parse_x_access_code(req.headers()) {
            Some(Ok(access_code)) => return ok(access_code),
            Some(Err(())) => {
                return err(
                    RequestError::HeaderInvalid(Self::name().to_string()).with_type_from(req)
                )
            }
            None => (),
        }

        let args = match from_str::<Args>(req.query_string()) {
            Ok(args) => args,
            Err(e) => return err(RequestError::QueryInvalid(e.to_string()).with_type_from(req)),
        };

        match args.ac {
            Some(ac) => ok(XAccessCode(ac)),
            None => err(RequestError::HeaderMissing("X-Access-Code".into()).with_type_from(req)),
        }
    }
}

impl PartialEq<String> for XAccessCode {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }
}

impl PartialEq<XAccessCode> for String {
    fn eq(&self, other: &XAccessCode) -> bool {
        self == &other.0
    }
}

impl Deref for XAccessCode {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn parse_x_access_code(headers: &HeaderMap) -> Option<Result<XAccessCode, ()>> {
    let access_code = match headers.get(XAccessCode::name()) {
        Some(access_code) => access_code,
        None => return None,
    };

    let access_code = match access_code.to_str() {
        Ok(access_code) => access_code.to_owned(),
        Err(_) => return Some(Err(())),
    };

    Some(Ok(XAccessCode(access_code)))
}
