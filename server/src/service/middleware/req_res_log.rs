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

use std::pin::Pin;
use std::str::from_utf8;
use std::task::{Context, Poll};

use actix_web::{
    dev::{
        Body, BodySize, MessageBody, Payload, PayloadStream, ResponseBody, Service, ServiceRequest,
        ServiceResponse, Transform,
    },
    error::{Error, PayloadError},
};
use bytes::Bytes;
use futures::{
    future::{ok, Future, FutureExt, Ready},
    stream::{Stream, StreamExt},
};
use log::info;
use rand::random;

pub struct ReqResLogging;

pub struct ReqResLoggingMiddleware<S> {
    service: S,
}

impl<S, B> Transform<S> for ReqResLogging
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = ReqResLoggingMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(ReqResLoggingMiddleware { service })
    }
}

impl<S, B> Service for ReqResLoggingMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<ServiceResponse<B>, Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        let (req, payload) = req.into_parts();
        let head = req.head();
        let tag = random::<usize>();

        info!(target: "req_res_log", "REQ{} - {} {} {:?}", &tag, &head.method, &head.uri, &head.version);
        for (key, value) in head.headers.iter() {
            info!(target: "req_res_log", "REQ{} - {}: {}", &tag, &key, value.to_str().unwrap());
        }

        let payload: Payload<PayloadStream> = match payload {
            Payload::None => Payload::None,
            Payload::H1(s) => Payload::Stream(Box::pin(ReqStream {
                tag,
                stream: Box::pin(s),
            })),
            Payload::H2(s) => Payload::Stream(Box::pin(ReqStream {
                tag,
                stream: Box::pin(s),
            })),
            Payload::Stream(s) => Payload::Stream(Box::pin(ReqStream { tag, stream: s })),
        };

        let req = ServiceRequest::from_parts(req, payload)
            .map_err(|_| ())
            .unwrap();

        Box::pin(self.service.call(req).map(move |res| {
            Ok(res?.map_body(|head, body| {
                info!(target: "req_res_log", "RES{} - {:?} {}", &tag, &head.version, &head.status);
                for (key, value) in head.headers.iter() {
                    info!(target: "req_res_log", "REQ{} - {}: {}", &tag, &key, value.to_str().unwrap());
                }

                match body {
                    ResponseBody::Body(b) => ResponseBody::Other(Body::Message(Box::new(ResBody {
                        tag,
                        body: Box::pin(b),
                    }))),
                    ResponseBody::Other(Body::None) => ResponseBody::Other(Body::None),
                    ResponseBody::Other(Body::Empty) => ResponseBody::Other(Body::Empty),
                    ResponseBody::Other(Body::Bytes(data)) => {
                        if let Ok(data) = from_utf8(&data) {
                            info!(target: "req_res_log", "RES{} - {}", &tag, &data);
                        }

                        ResponseBody::Other(Body::Bytes(data))
                    },
                    ResponseBody::Other(Body::Message(body)) => ResponseBody::Other(Body::Message(Box::new(ResBody {
                        tag,
                        body:  unsafe { Pin::new_unchecked(body) },
                    }))),
                }
            }))
        }))
    }
}

struct ReqStream {
    tag: usize,
    stream: Pin<Box<dyn Stream<Item = Result<Bytes, PayloadError>>>>,
}

impl Stream for ReqStream {
    type Item = Result<Bytes, PayloadError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match this.stream.poll_next_unpin(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(Ok(data))) => {
                if let Ok(data) = from_utf8(&data) {
                    info!(target: "req_res_log", "REQ{} - {}", &this.tag, &data);
                }

                Poll::Ready(Some(Ok(data)))
            }
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err))),
            Poll::Ready(None) => Poll::Ready(None),
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}

struct ResBody {
    tag: usize,
    body: Pin<Box<dyn MessageBody>>,
}

impl MessageBody for ResBody {
    fn size(&self) -> BodySize {
        self.body.size()
    }

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
        let this = self.get_mut();
        match this.body.as_mut().poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(Ok(data))) => {
                if let Ok(data) = from_utf8(&data) {
                    info!(target: "req_res_log", "RES{} - {}", &this.tag, &data);
                }

                Poll::Ready(Some(Ok(data)))
            }
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err))),
            Poll::Ready(None) => Poll::Ready(None),
        }
    }
}
