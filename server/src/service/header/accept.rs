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
        header::{Header, HeaderMap, HeaderName, HeaderValue, IntoHeaderValue, QualityItem},
        Error,
    },
    FromRequest, HttpMessage, HttpRequest,
};
use futures::future::{err, ok, Ready};
use mime::Mime;

use crate::service::{RequestError, TypedRequestError};

lazy_static! {
    pub static ref ACCEPT: HeaderName = HeaderName::from_lowercase(b"accept").unwrap();
}

pub struct Accept(pub Vec<QualityItem<Mime>>);

impl Accept {
    pub fn from_headers(headers: &HeaderMap) -> Result<Self, ()> {
        let mut result = Vec::<QualityItem<Mime>>::new();

        for header in headers.get_all(Accept::name()) {
            let s = header.to_str().map_err(|_| ())?;
            let items = s.split(',').filter_map(|x| match x.trim() {
                "" => None,
                x => Some(x.parse()),
            });

            for item in items {
                result.push(item.map_err(|_| ())?);
            }
        }

        result.sort_by(|a, b| a.quality.cmp(&b.quality));

        Ok(Accept(result))
    }
}

impl Deref for Accept {
    type Target = Vec<QualityItem<Mime>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Header for Accept {
    #[inline]
    fn name() -> HeaderName {
        ACCEPT.clone()
    }

    #[inline]
    fn parse<T: HttpMessage>(msg: &T) -> Result<Self, ParseError> {
        Accept::from_headers(msg.headers()).map_err(|_| ParseError::Header)
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
    type Error = TypedRequestError;
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        match Accept::from_headers(req.headers()) {
            Ok(accept) => ok(accept),
            Err(()) => {
                err(RequestError::HeaderInvalid(Self::name().to_string()).with_type_from(req))
            }
        }
    }
}
