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

use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use actix_web::{
    dev::{Payload, Service, ServiceRequest, ServiceResponse, Transform},
    error::{Error as ActixError, PayloadError},
    http::{
        header::{ContentType, IntoHeaderValue},
        Method,
    },
    HttpMessage, HttpResponse,
};
use bytes::{Bytes, BytesMut};
use futures::{
    future::{err, ok, Future, FutureExt, Ready},
    stream::{Stream, TryStreamExt},
};
use openssl::{ec::EcKey, pkey::Private, x509::X509};
use vau::{
    decode, encode, Decrypter, Encrypter, Error as VauError, PriorityFuture, UserPseudonymGenerator,
};

use crate::service::Error;

pub struct Vau {
    pkey: EcKey<Private>,
    cert: X509,
}

pub struct VauMiddleware<S> {
    handle: Handle<S>,
}

struct Handle<S>(Rc<RefCell<Inner<S>>>);

struct Inner<S> {
    service: S,
    cert: X509,
    decrypter: Decrypter,
    encrypter: Encrypter,
    user_pseudonym_generator: UserPseudonymGenerator,
}

impl Vau {
    pub fn new(pkey: EcKey<Private>, cert: X509) -> Result<Self, Error> {
        Ok(Vau { pkey, cert })
    }
}

impl<S> Transform<S> for Vau
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse, Error = ActixError> + 'static,
    S::Future: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse;
    type Error = ActixError;
    type InitError = ();
    type Transform = VauMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        match Handle::new(service, self.pkey.clone(), self.cert.clone()) {
            Ok(handle) => ok(VauMiddleware { handle }),
            Err(_) => err(()),
        }
    }
}

impl<S> Service for VauMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse, Error = ActixError> + 'static,
    S::Future: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse;
    type Error = ActixError;
    type Future = Pin<Box<dyn Future<Output = Result<ServiceResponse, ActixError>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), ActixError>> {
        self.handle.poll_ready(cx)
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        self.handle.clone().handle_request(req).boxed_local()
    }
}

impl<S> Handle<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse, Error = ActixError> + 'static,
    S::Future: 'static,
{
    fn new(service: S, pkey: EcKey<Private>, cert: X509) -> Result<Self, ActixError> {
        Ok(Self(Rc::new(RefCell::new(Inner {
            service,
            cert,
            decrypter: Decrypter::new(pkey)?,
            encrypter: Encrypter::default(),
            user_pseudonym_generator: USER_PSEUDONYM_GENERATOR.clone(),
        }))))
    }

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), ActixError>> {
        self.0.borrow_mut().service.poll_ready(cx)
    }

    async fn handle_request(self, req: ServiceRequest) -> Result<S::Response, ActixError> {
        let head = req.head();
        let mut parts = head.uri.path().split('/').filter(|s| !s.is_empty());

        match parts.next() {
            Some("VAU") => match parts.next() {
                Some(np) => {
                    let (np, is_valid) = {
                        let this = self.0.borrow_mut();
                        if !this.user_pseudonym_generator.verify(np).await {
                            (this.user_pseudonym_generator.generate().await?, false)
                        } else {
                            (np.to_owned(), true)
                        }
                    };

                    let fut = self.handle_vau_request(req, np);
                    let fut = PriorityFuture::new(fut, is_valid);

                    fut.await
                }
                None => Ok(not_found(req)),
            },
            Some("VAUCertificate") => {
                if parts.next().is_some() {
                    return Ok(not_found(req));
                }

                self.handle_vau_cert_request(req).await
            }
            _ => self.handle_normal_request(req).await,
        }
    }

    async fn handle_vau_request(
        self,
        req: ServiceRequest,
        np: String,
    ) -> Result<S::Response, ActixError> {
        if req.head().method != Method::POST {
            return Ok(method_not_allowed(req));
        }

        let (req, payload) = req.into_parts();
        if req.content_type() != "application/octet-stream" {
            return Ok(ServiceResponse::new(
                req,
                HttpResponse::BadRequest().finish(),
            ));
        }

        let mut this = self.0.borrow_mut();

        let payload = into_bytes(payload).await?;
        let payload = this.decrypter.decrypt(payload)?;

        let (decoded, next, req) = decode(req, &payload)?;

        let inner_res = this.service.call(next).await?;
        let inner_res = encode(decoded.request_id, inner_res).await?;

        let body = this.encrypter.encrypt(&decoded.response_key, inner_res)?;

        let res = HttpResponse::Ok()
            .content_type(ContentType::octet_stream().try_into()?)
            .header("Userpseudonym", np)
            .body(body);
        let res = ServiceResponse::new(req, res);

        Ok(res)
    }

    async fn handle_vau_cert_request(self, req: ServiceRequest) -> Result<S::Response, ActixError> {
        if req.head().method != Method::GET {
            return Ok(method_not_allowed(req));
        }

        let (req, _payload) = req.into_parts();
        let body = self.0.borrow().cert.to_der().map_err(Error::from)?;
        let res = HttpResponse::Ok()
            .content_type(ContentType::octet_stream().try_into()?)
            .body(body);
        let res = ServiceResponse::new(req, res);

        Ok(res)
    }

    #[cfg(feature = "vau-compat")]
    async fn handle_normal_request(self, req: ServiceRequest) -> Result<S::Response, ActixError> {
        self.0.borrow_mut().service.call(req).await
    }

    #[cfg(not(feature = "vau-compat"))]
    async fn handle_normal_request(self, req: ServiceRequest) -> Result<S::Response, ActixError> {
        Ok(not_found(req))
    }
}

impl<S> Clone for Handle<S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

async fn into_bytes(payload: Payload) -> Result<BytesMut, ActixError> {
    match payload {
        Payload::None => Err(VauError::NoPayload.into()),
        Payload::H1(s) => stream_into_bytes(s).await,
        Payload::H2(s) => stream_into_bytes(s).await,
        Payload::Stream(s) => stream_into_bytes(s).await,
    }
}

async fn stream_into_bytes<S>(mut s: S) -> Result<BytesMut, ActixError>
where
    S: Stream<Item = Result<Bytes, PayloadError>> + Unpin,
{
    let mut bytes = BytesMut::new();

    while let Some(payload) = s.try_next().await? {
        bytes.extend_from_slice(&payload);
    }

    Ok(bytes)
}

fn not_found(req: ServiceRequest) -> ServiceResponse {
    let (req, _) = req.into_parts();
    let res = HttpResponse::NotFound().finish();

    ServiceResponse::new(req, res)
}

fn method_not_allowed(req: ServiceRequest) -> ServiceResponse {
    let (req, _) = req.into_parts();
    let res = HttpResponse::MethodNotAllowed().finish();

    ServiceResponse::new(req, res)
}

lazy_static! {
    static ref USER_PSEUDONYM_GENERATOR: UserPseudonymGenerator = UserPseudonymGenerator::default();
}
