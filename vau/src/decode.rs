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

use std::pin::Pin;
use std::rc::Rc;
use std::str::from_utf8;
use std::task::{Context, Poll};

use actix_codec::Decoder;
use actix_http::h1::{Codec, Message};
use actix_router::{Path, Url};
use actix_web::{
    dev::{Payload, PayloadStream, ServiceRequest},
    error::{Error as ActixError, PayloadError},
    HttpRequest,
};
use bytes::{Bytes, BytesMut};
use futures::stream::Stream;

use super::{misc::hex_decode, Error};

pub struct Decoded<'a> {
    pub access_token: String,
    pub request_id: &'a str,
    pub response_key: Vec<u8>,
}

pub fn decode(
    req: HttpRequest,
    data: &[u8],
) -> Result<(Decoded, ServiceRequest, HttpRequest), ActixError> {
    let (decoded, data) = Decoded::from_bytes(data)?;
    let mut data = data.into();
    let mut codec = Codec::default();
    let head = match codec.decode(&mut data)? {
        Some(Message::Item(req)) => req,
        _ => return Err(Error::PayloadIncomplete.into()),
    };
    let payload = Payload::<PayloadStream>::Stream(Box::pin(PayloadDecoder {
        data: Some(data),
        codec: Some(codec),
    }));

    let head = if let (head, Payload::None) = head.into_parts() {
        head
    } else {
        return Err(Error::Internal.into());
    };

    let pool = req.0.pool;
    let next = if let Some(mut next) = pool.get_request() {
        let inner = Rc::get_mut(&mut next.0).unwrap();
        inner.path.get_mut().update(&head.uri);
        inner.path.reset();
        inner.head = head;
        inner.payload = payload;
        inner.app_data = req.0.app_data.clone();

        next
    } else {
        let mut res = HttpRequest::new(
            Path::new(Url::new(head.uri.clone())),
            head,
            payload,
            req.0.rmap.clone(),
            req.0.config.clone(),
            req.0.app_data[0].clone(),
            pool,
        );

        Rc::get_mut(&mut res.0).unwrap().app_data = req.0.app_data.clone();

        res
    };

    Ok((decoded, ServiceRequest::new(next), req))
}

impl<'a> Decoded<'a> {
    fn from_bytes(payload: &'a [u8]) -> Result<(Self, &'a [u8]), Error> {
        let mut it = payload.splitn(5, |b| *b == 0x20);

        let version = it.next().ok_or(Error::DecodeError)?;

        let access_token = it.next().ok_or(Error::DecodeError)?;
        let access_token = from_utf8(&access_token)
            .map_err(|_| Error::DecodeError)?
            .into();

        let request_id = it.next().ok_or(Error::DecodeError)?;
        let request_id = from_utf8(&request_id).map_err(|_| Error::DecodeError)?;

        let response_key = it.next().ok_or(Error::DecodeError)?;
        let response_key = from_utf8(&response_key).map_err(|_| Error::DecodeError)?;
        let response_key = hex_decode(&response_key).map_err(|_| Error::DecodeError)?;

        let inner_request = it.next().ok_or(Error::DecodeError)?;

        if version != b"1" {
            return Err(Error::DecodeError);
        }

        Ok((
            Self {
                access_token,
                request_id,
                response_key,
            },
            inner_request,
        ))
    }
}

struct PayloadDecoder {
    data: Option<BytesMut>,
    codec: Option<Codec>,
}

impl PayloadDecoder {
    fn next(&mut self) -> Result<Option<Bytes>, PayloadError> {
        let mut data = self.data.take().ok_or(PayloadError::EncodingCorrupted)?;
        let mut codec = self.codec.take().ok_or(PayloadError::EncodingCorrupted)?;

        let ret = match codec
            .decode(&mut data)
            .map_err(|_| PayloadError::EncodingCorrupted)?
        {
            Some(Message::Item(_)) => Err(PayloadError::EncodingCorrupted),
            Some(Message::Chunk(Some(bytes))) => Ok(Some(bytes)),
            Some(Message::Chunk(None)) => {
                if !data.is_empty() {
                    Err(PayloadError::EncodingCorrupted)
                } else {
                    Ok(None)
                }
            }
            None => Err(PayloadError::Incomplete(None)),
        };

        self.data = Some(data);
        self.codec = Some(codec);

        ret
    }
}

impl Stream for PayloadDecoder {
    type Item = Result<Bytes, PayloadError>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Option<Self::Item>> {
        match self.next() {
            Ok(Some(bytes)) => Poll::Ready(Some(Ok(bytes))),
            Ok(None) => Poll::Ready(None),
            Err(err) => Poll::Ready(Some(Err(err))),
        }
    }
}
