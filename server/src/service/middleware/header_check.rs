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

use std::task::{Context, Poll};

use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    error::Error,
    HttpMessage, HttpResponse,
};
use encoding_rs::UTF_8;
use futures::future::{ok, Either, Ready};

pub struct HeaderCheck;

pub struct HeaderCheckMiddleware<S> {
    service: S,
}

impl<S> Transform<S> for HeaderCheck
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse, Error = Error>,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse;
    type Error = Error;
    type InitError = ();
    type Transform = HeaderCheckMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(HeaderCheckMiddleware { service })
    }
}

impl<S> Service for HeaderCheckMiddleware<S>
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
        match req.encoding() {
            Ok(encoding) if encoding == UTF_8 => (),
            _ => {
                let (req, _payload) = req.into_parts();

                return Either::Right(ok(ServiceResponse::new(
                    req,
                    HttpResponse::NotImplemented().body("Unsupported encoding!"),
                )));
            }
        }

        for header in UNSUPPORTED_HEADERS {
            if req.headers().contains_key(*header) {
                let (req, _payload) = req.into_parts();

                return Either::Right(ok(ServiceResponse::new(
                    req,
                    HttpResponse::NotImplemented().body(format!("Unsupported header: {}!", header)),
                )));
            }
        }

        Either::Left(self.service.call(req))
    }
}

const UNSUPPORTED_HEADERS: &[&str] = &[
    "Content-Language",
    "Content-Location",
    "Content-MD5",
    "Content-Range",
];
