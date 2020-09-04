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

use std::str::FromStr;

use actix_web::{
    dev::Payload,
    error::ParseError,
    http::{
        header::{Header, HeaderMap, HeaderName, HeaderValue, IntoHeaderValue, QualityItem},
        Error,
    },
    FromRequest, HttpMessage, HttpRequest,
};
use futures::future::{err, ok, Ready};
use mime::Mime;

use super::{error::Error as ServiceError, idp_client::IdToken};

lazy_static! {
    pub static ref ACCEPT: HeaderName = HeaderName::from_lowercase(b"accept").unwrap();
    pub static ref CONTENT_TYPE: HeaderName = HeaderName::from_lowercase(b"content-type").unwrap();
    pub static ref X_ACCESS_CODE: HeaderName = HeaderName::from_lowercase(b"x-accesscode").unwrap();
    pub static ref AUTHORIZATION: HeaderName =
        HeaderName::from_lowercase(b"authorization").unwrap();
}

pub struct Accept(pub Vec<QualityItem<Mime>>);
pub struct ContentType(pub Mime);
pub struct XAccessCode(pub String);
pub struct Authorization(pub IdToken);

impl Header for Accept {
    #[inline]
    fn name() -> HeaderName {
        ACCEPT.clone()
    }

    #[inline]
    fn parse<T: HttpMessage>(msg: &T) -> Result<Self, ParseError> {
        parse_accept(msg.headers()).map_err(|_| ParseError::Header)
    }
}

impl IntoHeaderValue for Accept {
    type Error = Error;

    fn try_into(self) -> Result<HeaderValue, Self::Error> {
        self.0
            .into_iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(", ")
            .try_into()
            .map_err(Into::into)
    }
}

impl FromRequest for Accept {
    type Error = ServiceError;
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        match parse_accept(req.headers()) {
            Ok(accept) => ok(accept),
            Err(()) => err(ServiceError::InvalidHeader(Self::name())),
        }
    }
}

impl Header for ContentType {
    #[inline]
    fn name() -> HeaderName {
        CONTENT_TYPE.clone()
    }

    #[inline]
    fn parse<T: HttpMessage>(msg: &T) -> Result<Self, ParseError> {
        parse_content_type(msg.headers())
            .ok_or_else(|| ParseError::Header)?
            .map_err(|_| ParseError::Header)
    }
}

impl IntoHeaderValue for ContentType {
    type Error = Error;

    fn try_into(self) -> Result<HeaderValue, Self::Error> {
        self.0.to_string().try_into().map_err(Into::into)
    }
}

impl FromRequest for ContentType {
    type Error = ServiceError;
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        match parse_content_type(req.headers()) {
            None => err(ServiceError::ExpectHeader(Self::name())),
            Some(Err(())) => err(ServiceError::InvalidHeader(Self::name())),
            Some(Ok(content_type)) => ok(content_type),
        }
    }
}

impl Header for XAccessCode {
    #[inline]
    fn name() -> HeaderName {
        X_ACCESS_CODE.clone()
    }

    #[inline]
    fn parse<T: HttpMessage>(msg: &T) -> Result<Self, ParseError> {
        parse_x_access_code(msg.headers())
            .ok_or_else(|| ParseError::Header)?
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
    type Error = ServiceError;
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        match parse_x_access_code(req.headers()) {
            None => err(ServiceError::ExpectHeader(Self::name())),
            Some(Err(())) => err(ServiceError::InvalidHeader(Self::name())),
            Some(Ok(access_code)) => ok(access_code),
        }
    }
}

impl Header for Authorization {
    #[inline]
    fn name() -> HeaderName {
        AUTHORIZATION.clone()
    }

    #[inline]
    fn parse<T: HttpMessage>(msg: &T) -> Result<Self, ParseError> {
        parse_authorization(msg.headers())
            .ok_or_else(|| ParseError::Header)?
            .map_err(|_| ParseError::Header)
    }
}

impl IntoHeaderValue for Authorization {
    type Error = Error;

    fn try_into(self) -> Result<HeaderValue, Self::Error> {
        self.0.into_inner().try_into().map_err(Into::into)
    }
}

impl FromRequest for Authorization {
    type Error = ServiceError;
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        match parse_authorization(req.headers()) {
            None => err(ServiceError::ExpectHeader(Self::name())),
            Some(Err(())) => err(ServiceError::InvalidHeader(Self::name())),
            Some(Ok(authorization)) => ok(authorization),
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

fn parse_accept(headers: &HeaderMap) -> Result<Accept, ()> {
    let mut result = Vec::new();

    for header in headers.get_all(Accept::name()) {
        let s = header.to_str().map_err(|_| ())?;
        result.extend(
            s.split(',')
                .filter_map(|x| match x.trim() {
                    "" => None,
                    y => Some(y),
                })
                .filter_map(|x| x.trim().parse().ok()),
        )
    }

    Ok(Accept(result))
}

fn parse_authorization(headers: &HeaderMap) -> Option<Result<Authorization, ()>> {
    let id_token = match headers.get(Authorization::name()) {
        Some(id_token) => id_token,
        None => return None,
    };

    let id_token = match id_token.to_str() {
        Ok(id_token) => id_token.to_owned(),
        Err(_) => return Some(Err(())),
    };

    let id_token = match id_token.parse() {
        Ok(id_token) => id_token,
        Err(_) => return Some(Err(())),
    };

    Some(Ok(Authorization(id_token)))
}
