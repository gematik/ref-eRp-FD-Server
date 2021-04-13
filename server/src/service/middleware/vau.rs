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

use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use actix_web::{
    dev::{Payload, Service, ServiceRequest, ServiceResponse, Transform},
    error::{Error as ActixError, PayloadError},
    http::{
        header::{ContentType, HeaderName, IntoHeaderValue},
        Method,
    },
    web::Data,
    HttpMessage, HttpResponse,
};
use bytes::{Bytes, BytesMut};
use chrono::Utc;
use futures::{
    future::{ok, Future, FutureExt, Ready},
    stream::{Stream, TryStreamExt},
};
use vau::{decode, encode, Decrypter, Encrypter, Error as VauError, UserPseudonymGenerator};

use crate::{
    pki_store::PkiStore,
    service::{
        misc::{AccessToken, AccessTokenError},
        RequestError, TypedRequestResult,
    },
};

pub struct Vau;

pub struct VauMiddleware<S> {
    handle: Handle<S>,
}

struct Handle<S>(Rc<RefCell<Inner<S>>>);

struct Inner<S> {
    service: S,
    decrypter: Option<Decrypter>,
    encrypter: Encrypter,
    user_pseudonym_generator: UserPseudonymGenerator,
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
        let handle = Handle::new(service);

        ok(VauMiddleware { handle })
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
    fn new(service: S) -> Self {
        Self(Rc::new(RefCell::new(Inner {
            service,
            decrypter: None,
            encrypter: Encrypter::default(),
            user_pseudonym_generator: USER_PSEUDONYM_GENERATOR.clone(),
        })))
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
                    let np = {
                        let this = self.0.borrow_mut();

                        if !this.user_pseudonym_generator.verify(np).await {
                            this.user_pseudonym_generator.generate().await?
                        } else {
                            np.to_owned()
                        }
                    };

                    self.handle_vau_request(req, np).await
                }
                None => Ok(not_found(req)),
            },
            Some("VAUCertificate") => {
                if parts.next().is_some() {
                    return Ok(not_found(req));
                }

                self.handle_vau_cert_request(req).await
            }
            Some("VAUCertificateOCSPResponse") => {
                if parts.next().is_some() {
                    return Ok(not_found(req));
                }

                self.handle_vau_ocsp_request(req).await
            }
            _ => self.handle_normal_request(req).await,
        }
    }

    async fn handle_vau_request(
        self,
        outer_service_req: ServiceRequest,
        np: String,
    ) -> Result<S::Response, ActixError> {
        if outer_service_req.head().method != Method::POST {
            return Ok(method_not_allowed(outer_service_req));
        }

        let (outer_http_req, payload) = outer_service_req.into_parts();
        let content_type = outer_http_req.content_type();
        if content_type != "application/octet-stream" {
            return Err(RequestError::ContentTypeNotSupported
                .with_type_from(&outer_http_req)
                .into());
        }

        let mut this = self.0.borrow_mut();

        if this.decrypter.is_none() {
            let pki_store = outer_http_req
                .app_data::<Data<PkiStore>>()
                .expect("Shared data 'PkiStore' is missing!");

            this.decrypter = Some(Decrypter::new(pki_store.enc_key().to_owned())?);
        }

        let decrypter = this.decrypter.as_mut().unwrap();
        let outer_payload = into_bytes(payload).await?;
        let outer_payload = decrypter.decrypt(outer_payload)?;

        let (decoded, inner_service_req, outer_http_req) = decode(outer_http_req, &outer_payload)?;

        extract_access_token(&inner_service_req, Some(&decoded.access_token))
            .map_err(|_| RequestError::InvalidAccessToken)
            .err_with_type_from(&inner_service_req)?;

        let inner_http_res = match this.service.call(inner_service_req).await {
            Ok(res) => res.into(),
            Err(err) => err.as_response_error().error_response(),
        };

        let inner_res = encode(decoded.request_id, inner_http_res).await?;

        let body = this.encrypter.encrypt(&decoded.response_key, inner_res)?;

        let res = HttpResponse::Ok()
            .content_type(ContentType::octet_stream().try_into()?)
            .header("Userpseudonym", np)
            .body(body);
        let res = ServiceResponse::new(outer_http_req, res);

        Ok(res)
    }

    async fn handle_vau_cert_request(self, req: ServiceRequest) -> Result<S::Response, ActixError> {
        if req.head().method != Method::GET {
            return Ok(method_not_allowed(req));
        }

        let (req, _payload) = req.into_parts();
        let pki_store = req
            .app_data::<Data<PkiStore>>()
            .expect("Shared data 'PkiStore' is missing!");
        let cert = pki_store
            .enc_cert()
            .to_der()
            .map_err(VauError::OpenSslError)?;

        let res = HttpResponse::Ok()
            .content_type(ContentType::octet_stream().try_into()?)
            .body(cert);
        let res = ServiceResponse::new(req, res);

        Ok(res)
    }

    async fn handle_vau_ocsp_request(self, req: ServiceRequest) -> Result<S::Response, ActixError> {
        if req.head().method != Method::GET {
            return Ok(method_not_allowed(req));
        }

        let (req, _payload) = req.into_parts();
        let pki_store = req
            .app_data::<Data<PkiStore>>()
            .expect("Shared data 'PkiStore' is missing!");

        let ocsp_vau = pki_store.ocsp_vau();
        let res = match &*ocsp_vau {
            Some(ocsp_vau) => {
                let body = ocsp_vau.to_der().map_err(VauError::OpenSslError)?;

                HttpResponse::Ok()
                    .content_type(ContentType::octet_stream().try_into()?)
                    .body(body)
            }
            None => HttpResponse::NotFound().finish(),
        };

        drop(ocsp_vau);

        let res = ServiceResponse::new(req, res);

        Ok(res)
    }

    #[cfg(feature = "vau-compat")]
    async fn handle_normal_request(self, req: ServiceRequest) -> Result<S::Response, ActixError> {
        match extract_access_token(&req, None) {
            Ok(()) => (),
            Err(err) => {
                let uri = req.head().uri.path();

                if !URI_WHITELIST.contains(&uri) {
                    return Err(err.with_type_from(&req).into());
                }
            }
        }

        self.0.borrow_mut().service.call(req).await
    }

    #[cfg(not(feature = "vau-compat"))]
    async fn handle_normal_request(self, req: ServiceRequest) -> Result<S::Response, ActixError> {
        let uri = req.head().uri.path();

        if URI_WHITELIST.contains(&uri) {
            self.0.borrow_mut().service.call(req).await
        } else {
            Ok(not_found(req))
        }
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

fn extract_access_token(
    req: &ServiceRequest,
    expected_access_token: Option<&str>,
) -> Result<(), RequestError> {
    let pki_store = req
        .app_data::<Data<PkiStore>>()
        .expect("Shared data 'PkiStore' is missing!");

    let pub_key = pki_store
        .puk_token()
        .as_ref()
        .ok_or(AccessTokenError::NoPukToken)?
        .public_key
        .clone();

    let access_token = req
        .headers()
        .get(&*AUTHORIZATION_HEADER)
        .ok_or(AccessTokenError::Missing)?
        .to_str()
        .map_err(|_| AccessTokenError::InvalidValue)?;

    if !access_token.starts_with("Bearer ") {
        return Err(AccessTokenError::InvalidValue.into());
    }

    let access_token = &access_token[7..];

    if let Some(expected_access_token) = expected_access_token {
        if access_token != expected_access_token {
            return Err(RequestError::AccessTokenError(AccessTokenError::Mismatch));
        }
    }

    let access_token = AccessToken::verify(access_token, pub_key, Utc::now())?;

    req.extensions_mut().insert(Rc::new(access_token));

    Ok(())
}

lazy_static! {
    static ref USER_PSEUDONYM_GENERATOR: UserPseudonymGenerator = UserPseudonymGenerator::default();
    static ref AUTHORIZATION_HEADER: HeaderName =
        HeaderName::from_lowercase(b"authorization").unwrap();
}

const URI_WHITELIST: &[&str] = &["/CertList", "/OCSPList", "/Random"];
