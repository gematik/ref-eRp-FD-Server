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

use std::mem::take;
use std::pin::Pin;
use std::str::{from_utf8, Utf8Error};
use std::task::{Context, Poll};

use bytes::{Bytes, BytesMut};
use futures::{
    future::LocalBoxFuture,
    stream::{Stream, TryStreamExt},
};

pub enum ByteStream<E>
where
    E: Encoder,
{
    Done,
    Idle(E),
    Pending(EncoderFuture<E, E::Error>),
}

pub trait Encoder: Sized {
    type Error: From<Utf8Error>;

    fn next(self) -> EncoderFuture<Self, Self::Error>;
}

type EncoderFuture<T, E> = LocalBoxFuture<'static, Result<Option<(T, Bytes)>, E>>;

impl<E> ByteStream<E>
where
    Self: Unpin,
    E: Encoder,
{
    #[allow(dead_code)]
    pub async fn into_bytes(mut self) -> Result<Bytes, E::Error> {
        let mut bytes = BytesMut::new();

        while let Some(b) = self.try_next().await? {
            bytes.extend(b);
        }

        Ok(bytes.freeze())
    }

    #[allow(dead_code)]
    pub async fn into_string(self) -> Result<String, E::Error> {
        let bytes = self.into_bytes().await?;
        let s = from_utf8(&bytes)?;

        Ok(s.into())
    }
}

impl<E> Default for ByteStream<E>
where
    E: Encoder,
{
    fn default() -> Self {
        Self::Done
    }
}

impl<E> Stream for ByteStream<E>
where
    Self: Unpin,
    E: Encoder,
{
    type Item = Result<Bytes, E::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        loop {
            match take(this) {
                ByteStream::Done => return Poll::Ready(None),
                ByteStream::Idle(inner) => *this = ByteStream::Pending(inner.next()),
                ByteStream::Pending(mut fut) => match fut.as_mut().poll(cx) {
                    Poll::Pending => *this = ByteStream::Pending(fut),
                    Poll::Ready(Err(err)) => return Poll::Ready(Some(Err(err))),
                    Poll::Ready(Ok(None)) => return Poll::Ready(None),
                    Poll::Ready(Ok(Some((inner, bytes)))) => {
                        *this = ByteStream::Idle(inner);

                        return Poll::Ready(Some(Ok(bytes)));
                    }
                },
            };
        }
    }
}
