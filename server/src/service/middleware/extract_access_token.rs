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

use std::rc::Rc;
use std::task::{Context, Poll};

use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    error::Error,
    http::header::HeaderName,
    HttpRequest,
};
use chrono::Utc;
use futures::future::{ok, ready, Either, Ready};

use crate::service::{
    misc::{AccessToken, AccessTokenError},
    PukToken, RequestError,
};

pub struct ExtractAccessToken;

pub struct ExtractAccessTokenMiddleware<S> {
    service: S,
}

impl<S> Transform<S> for ExtractAccessToken
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse, Error = Error>,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse;
    type Error = Error;
    type InitError = ();
    type Transform = ExtractAccessTokenMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(ExtractAccessTokenMiddleware { service })
    }
}

impl<S> Service for ExtractAccessTokenMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse, Error = Error>,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse;
    type Error = Error;
    type Future = Either<S::Future, Ready<Result<Self::Response, Error>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        let (req, payload) = req.into_parts();

        match parse_authorization(&req) {
            Ok(()) => (),
            Err(err) => return Either::Right(ready(Err(err.into()))),
        };

        let req = ServiceRequest::from_parts(req, payload)
            .map_err(|_| ())
            .unwrap();

        Either::Left(self.service.call(req))
    }
}

pub fn extract_access_token(req: &HttpRequest) -> Result<&str, RequestError> {
    let access_token = req
        .headers()
        .get(&*AUTHORIZATION)
        .ok_or_else(|| AccessTokenError::Missing)?
        .to_str()
        .map_err(|_| AccessTokenError::InvalidValue)?;

    if !access_token.starts_with("Bearer ") {
        return Err(AccessTokenError::InvalidValue.into());
    }

    Ok(&access_token[7..])
}

fn parse_authorization(req: &HttpRequest) -> Result<(), RequestError> {
    let puk_token = req
        .app_data::<PukToken>()
        .ok_or_else(|| RequestError::internal("Shared data 'PukToken' is missing!"))?;

    let pub_key = puk_token
        .load()
        .as_ref()
        .ok_or_else(|| AccessTokenError::NoPukToken)?
        .public_key
        .clone();

    let access_token = extract_access_token(req)?;
    let access_token = AccessToken::verify(access_token, pub_key, Utc::now())?;

    req.extensions_mut().insert(Rc::new(access_token));

    Ok(())
}

lazy_static! {
    pub static ref AUTHORIZATION: HeaderName =
        HeaderName::from_lowercase(b"authorization").unwrap();
}
