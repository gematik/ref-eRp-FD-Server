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

use std::fmt::{Display, Formatter, Result as FmtResult};
#[cfg(feature = "req-res-log")]
use std::pin::Pin;
#[cfg(feature = "req-res-log")]
use std::task::{Context, Poll};

#[cfg(feature = "req-res-log")]
use actix_web::{
    dev::{Body, BodySize, Payload, PayloadStream},
    error::PayloadError,
};
use actix_web::{
    dev::{MessageBody, ResponseBody, ResponseHead, ServiceRequest},
    error::Error,
};
#[cfg(feature = "req-res-log")]
use bytes::Bytes;
#[cfg(feature = "req-res-log")]
use futures::{
    future::Future,
    stream::{Stream, StreamExt},
};
#[cfg(feature = "req-res-log")]
use log::info;
use rand::random;
#[cfg(feature = "req-res-log")]
use tokio::task::spawn_local;

#[derive(Clone, Copy)]
pub struct RequestTag(usize);

impl Default for RequestTag {
    fn default() -> Self {
        Self(random())
    }
}

impl Display for RequestTag {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:016X}", &self.0)
    }
}

#[cfg(feature = "req-res-log")]
pub fn log_req(msg: &str, req: ServiceRequest, tag: RequestTag) -> ServiceRequest {
    let (req, payload) = req.into_parts();
    let head = req.head();

    info!(target: "req_res_log", "{} - {}", &tag, msg);
    info!(target: "req_res_log", "{} - {} {} {:?}", &tag, &head.method, &head.uri, &head.version);
    for (key, value) in head.headers.iter() {
        info!(target: "req_res_log", "{} - {}: {}", &tag, &key, value.to_str().unwrap());
    }

    let payload: Payload<PayloadStream> = match payload {
        Payload::None => Payload::None,
        Payload::H1(s) => Payload::Stream(Box::pin(ReqStream::new(tag, Box::pin(s)))),
        Payload::H2(s) => Payload::Stream(Box::pin(ReqStream::new(tag, Box::pin(s)))),
        Payload::Stream(s) => Payload::Stream(Box::pin(ReqStream::new(tag, s))),
    };

    ServiceRequest::from_parts(req, payload)
        .map_err(|_| ())
        .unwrap()
}

#[cfg(not(feature = "req-res-log"))]
pub fn log_req(msg: &str, req: ServiceRequest, _tag: RequestTag) -> ServiceRequest {
    req
}

#[cfg(feature = "req-res-log")]
pub fn log_res<B1, B2>(
    msg: &str,
    head: &mut ResponseHead,
    body: ResponseBody<B1>,
    tag: RequestTag,
) -> ResponseBody<B2>
where
    B1: MessageBody + 'static,
    B2: MessageBody + 'static,
{
    info!(target: "req_res_log", "{} - {}", &tag, msg);
    info!(target: "req_res_log", "{} - {:?} {}", &tag, &head.version, &head.status);
    for (key, value) in head.headers.iter() {
        info!(target: "req_res_log", "{} - {}: {}", &tag, &key, value.to_str().unwrap());
    }

    match body {
        ResponseBody::Body(b) => {
            ResponseBody::Other(Body::Message(Box::new(ResBody::new(tag, Box::pin(b)))))
        }
        ResponseBody::Other(Body::None) => ResponseBody::Other(Body::None),
        ResponseBody::Other(Body::Empty) => ResponseBody::Other(Body::Empty),
        ResponseBody::Other(Body::Bytes(data)) => {
            LogWriter::new(format!("{}", tag)).write(&data).finish();

            ResponseBody::Other(Body::Bytes(data))
        }
        ResponseBody::Other(Body::Message(body)) => {
            ResponseBody::Other(Body::Message(Box::new(ResBody::new(tag, unsafe {
                Pin::new_unchecked(body)
            }))))
        }
    }
}

#[cfg(not(feature = "req-res-log"))]
pub fn log_res<B>(
    _msg: &str,
    _head: &mut ResponseHead,
    body: ResponseBody<B>,
    _tag: RequestTag,
) -> ResponseBody<B>
where
    B: MessageBody + 'static,
{
    body
}

#[cfg(feature = "req-res-log")]
pub fn log_err(msg: &str, err: Error, tag: RequestTag) -> Error {
    let res = err.as_response_error().error_response();
    let head = res.head();

    info!(target: "req_res_log", "{} - {}", &tag, msg);
    info!(target: "req_res_log", "{} - {:?} {}", &tag, &head.version, &head.status);
    for (key, value) in head.headers.iter() {
        info!(target: "req_res_log", "{} - {}: {}", &tag, &key, value.to_str().unwrap());
    }

    let (_, body) = res.into_parts();
    match body {
        ResponseBody::Other(Body::None) => (),
        ResponseBody::Other(Body::Empty) => (),
        ResponseBody::Other(Body::Bytes(data)) => {
            LogWriter::new(format!("{}", tag)).write(&data).finish();
        }
        ResponseBody::Body(b) => {
            spawn_local(ResBody::new(tag, Box::pin(b)));
        }
        ResponseBody::Other(Body::Message(body)) => {
            spawn_local(ResBody::new(tag, unsafe { Pin::new_unchecked(body) }));
        }
    }

    err
}

#[cfg(not(feature = "req-res-log"))]
pub fn log_err(_msg: &str, err: Error, _tag: RequestTag) -> Error {
    err
}

#[cfg(feature = "req-res-log")]
struct ReqStream {
    stream: Pin<Box<dyn Stream<Item = Result<Bytes, PayloadError>>>>,
    log_writer: LogWriter,
}

#[cfg(feature = "req-res-log")]
impl ReqStream {
    fn new(
        tag: RequestTag,
        stream: Pin<Box<dyn Stream<Item = Result<Bytes, PayloadError>>>>,
    ) -> Self {
        Self {
            stream,
            log_writer: LogWriter::new(format!("{}", tag)),
        }
    }
}

#[cfg(feature = "req-res-log")]
impl Stream for ReqStream {
    type Item = Result<Bytes, PayloadError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match this.stream.poll_next_unpin(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(Ok(data))) => {
                this.log_writer.write(&data);

                Poll::Ready(Some(Ok(data)))
            }
            Poll::Ready(Some(Err(err))) => {
                this.log_writer.finish();

                Poll::Ready(Some(Err(err)))
            }
            Poll::Ready(None) => {
                this.log_writer.finish();

                Poll::Ready(None)
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}

#[cfg(feature = "req-res-log")]
struct ResBody {
    tag: RequestTag,
    body: Pin<Box<dyn MessageBody>>,
    log_writer: LogWriter,
}

#[cfg(feature = "req-res-log")]
impl ResBody {
    fn new(tag: RequestTag, body: Pin<Box<dyn MessageBody>>) -> Self {
        let log_writer = LogWriter::new(format!("{}", &tag));

        Self {
            tag,
            body,
            log_writer,
        }
    }
}

#[cfg(feature = "req-res-log")]
impl MessageBody for ResBody {
    fn size(&self) -> BodySize {
        self.body.size()
    }

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, Error>>> {
        let this = self.get_mut();
        match this.body.as_mut().poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(Ok(data))) => {
                this.log_writer.write(&data);

                Poll::Ready(Some(Ok(data)))
            }
            Poll::Ready(Some(Err(err))) => {
                this.log_writer.finish();

                Poll::Ready(Some(Err(err)))
            }
            Poll::Ready(None) => {
                this.log_writer.finish();

                Poll::Ready(None)
            }
        }
    }
}

#[cfg(feature = "req-res-log")]
impl Future for ResBody {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        loop {
            match this.body.as_mut().poll_next(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Some(Ok(data))) => {
                    this.log_writer.write(&data);
                }
                Poll::Ready(Some(Err(err))) => {
                    this.log_writer.finish();

                    info!(target: "req_res_log", "{} - Error in body: {}", &this.tag, &err);

                    return Poll::Ready(());
                }
                Poll::Ready(None) => {
                    this.log_writer.finish();

                    return Poll::Ready(());
                }
            }
        }
    }
}

#[cfg(feature = "req-res-log")]
struct LogWriter {
    prefix: String,
    offset: usize,
    buffer: Vec<u8>,
}

#[cfg(feature = "req-res-log")]
impl LogWriter {
    fn new(prefix: String) -> Self {
        Self {
            prefix,
            offset: 0,
            buffer: Vec::new(),
        }
    }

    fn write(&mut self, mut data: &[u8]) -> &mut Self {
        while self.buffer.len() + data.len() >= 16 {
            let (hex, ascii) = format_line(self.buffer.iter().chain(data).take(16));
            log_line(self.offset, &self.prefix, &hex, &ascii);

            let skip = 16 - self.buffer.len();
            data = &data[skip..];

            self.offset += 16;
            self.buffer.clear();
        }

        self.buffer.extend_from_slice(data);

        self
    }

    fn finish(&mut self) {
        if !self.buffer.is_empty() {
            let (hex, ascii) = format_line(self.buffer.iter());
            self.buffer.clear();

            log_line(self.offset, &self.prefix, &hex, &ascii);
        }
    }
}

#[cfg(feature = "req-res-log")]
impl Drop for LogWriter {
    fn drop(&mut self) {
        self.finish();
    }
}

#[cfg(feature = "req-res-log")]
fn format_line<'a, I>(iter: I) -> (String, String)
where
    I: Iterator<Item = &'a u8>,
{
    let mut hex = String::new();
    let mut ascii = String::new();

    for (i, b) in iter.enumerate() {
        let space = if i == 8 { " " } else { "" };

        let c = *b as char;
        let c = if c.is_ascii()
            && (c == ' ' || c.is_ascii_alphanumeric() || c.is_ascii_punctuation())
        {
            c
        } else {
            '.'
        };

        hex = format!("{} {}{:02X}", hex, space, b);
        ascii = format!("{}{}{}", ascii, space, c);
    }

    (hex, ascii)
}

#[cfg(feature = "req-res-log")]
fn log_line(offset: usize, prefix: &str, hex: &str, ascii: &str) {
    info!(target: "req_res_log", "{} - {:08X} |{:49} | {}", prefix, offset, hex, ascii);
}
