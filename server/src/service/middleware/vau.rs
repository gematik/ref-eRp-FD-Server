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

use crate::service::misc::logging::{log_err, log_req, log_res, RequestTag};

use crate::{
    pki_store::PkiStore,
    service::{
        misc::{AccessToken, AccessTokenError},
        RequestError,
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
        let tag = RequestTag::default();
        req.extensions_mut().insert(tag);

        let req = log_req("Received Request", req, tag);

        let head = req.head();
        let mut parts = head.uri.path().split('/').filter(|s| !s.is_empty());

        let res = match parts.next() {
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
        };

        match res {
            Ok(res) => {
                Ok(res
                    .map_body(|head, body| log_res("Sending Response (Success)", head, body, tag)))
            }
            Err(err) => Err(log_err("Sending Response (Error)", err, tag)),
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

        let request_tag = *outer_service_req.extensions().get::<RequestTag>().unwrap();
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

        let outer_payload = log_err!(
            into_bytes(payload).await,
            "Error while reading VAU payload: {:?}"
        )?;

        let outer_payload = log_err!(
            decrypter.decrypt(outer_payload),
            "Error while decrypting VAU payload: {:?}"
        )?;

        let (decoded, inner_service_req, outer_http_req) = log_err!(
            decode(outer_http_req, &outer_payload),
            "Error while decoding VAU payload: {:?}"
        )?;

        inner_service_req.extensions_mut().insert(request_tag);
        let inner_service_req = log_req("VAU Inner Request", inner_service_req, request_tag);

        let access_token = log_err!(
            extract_access_token(&inner_service_req, Some(&decoded.access_token)),
            "Error while extracting ACCESS_TOKEN: {:?}"
        );

        let inner_http_res = if let Err(err) = access_token {
            let err = RequestError::AccessTokenError(err);
            let err: ActixError = err.with_type_from(&inner_service_req).into();

            err.as_response_error().error_response()
        } else {
            let res = log_err!(
                this.service.call(inner_service_req).await,
                "Error while handling inner request: {:?}"
            );
            match res {
                Ok(res) => res.into(),
                Err(err) => err.as_response_error().error_response(),
            }
        };

        let inner_http_res = inner_http_res
            .map_body(|head, body| log_res("VAU Inner Response", head, body, request_tag));

        let inner_res = log_err!(
            encode(decoded.request_id, inner_http_res).await,
            "Error while encoding VAU payload: {:?}"
        )?;

        let body = log_err!(
            this.encrypter.encrypt(&decoded.response_key, inner_res),
            "Error while encrypting VAU payload: {:?}"
        )?;

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
        let access_token = extract_access_token(&req, None);

        match access_token {
            Ok(()) => (),
            Err(err) => {
                let uri = req.head().uri.path();

                if !URI_WHITELIST.contains(&uri) {
                    log_err!(only_err, &err, "Error while extracting ACCESS_TOKEN: {:?}");

                    return Err(RequestError::AccessTokenError(err)
                        .with_type_from(&req)
                        .into());
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

async fn into_bytes(payload: Payload) -> Result<BytesMut, PayloadError> {
    match payload {
        Payload::None => Err(PayloadError::Incomplete(None)),
        Payload::H1(s) => stream_into_bytes(s).await,
        Payload::H2(s) => stream_into_bytes(s).await,
        Payload::Stream(s) => stream_into_bytes(s).await,
    }
}

async fn stream_into_bytes<S>(mut s: S) -> Result<BytesMut, PayloadError>
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
) -> Result<(), AccessTokenError> {
    let pki_store = req
        .app_data::<Data<PkiStore>>()
        .expect("Shared data 'PkiStore' is missing!");

    let pub_key = pki_store
        .puk_token()
        .as_ref()
        .ok_or(AccessTokenError::NoPukToken)?
        .token_key
        .clone();

    let access_token = req
        .headers()
        .get(&*AUTHORIZATION_HEADER)
        .ok_or(AccessTokenError::Missing)?
        .to_str()
        .map_err(|_| AccessTokenError::InvalidValue)?;

    let access_token = match access_token.strip_prefix("Bearer") {
        Some(access_token) => access_token.trim(),
        None => return Err(AccessTokenError::InvalidValue),
    };

    if let Some(expected_access_token) = expected_access_token {
        if access_token != expected_access_token {
            return Err(AccessTokenError::Mismatch);
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

const URI_WHITELIST: &[&str] = &["/CertList", "/OCSPList", "/Random", "/Health"];
