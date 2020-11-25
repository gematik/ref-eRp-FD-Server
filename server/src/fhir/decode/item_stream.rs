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

use std::fmt::Debug;
use std::mem::take;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{
    future::{Future, LocalBoxFuture},
    stream::Stream,
};

use crate::fhir::{Format, WithFormat};

use super::DecodeError;

#[allow(clippy::type_complexity)]
pub enum ItemStream<'a, D>
where
    D: Decoder<'a>,
{
    Done,
    Idle(D),
    Pending(DecoderFuture<'a, D, D::Error>),
}

#[derive(Debug, PartialEq)]
pub enum Item {
    BeginElement {
        name: String,
    },
    EndElement,
    Field {
        name: String,
        value: String,
        extension: Vec<Item>,
    },
}

pub trait Decoder<'a>: Sized {
    type Error;

    fn next(self) -> DecoderFuture<'a, Self, Self::Error>;
}

pub type DecodeFuture<'a, T, E> = LocalBoxFuture<'a, Result<T, DecodeError<E>>>;
pub type DecoderFuture<'a, T, E> = LocalBoxFuture<'a, Result<Option<(T, Item)>, E>>;

impl<'a, D> Default for ItemStream<'a, D>
where
    D: Decoder<'a>,
{
    fn default() -> Self {
        Self::Done
    }
}

impl<'a, D> WithFormat for ItemStream<'a, D>
where
    D: Decoder<'a> + WithFormat,
{
    fn format(&self) -> Option<Format> {
        match self {
            Self::Done => None,
            Self::Idle(d) => d.format(),
            Self::Pending(_) => None,
        }
    }
}

impl<'a, D> Stream for ItemStream<'a, D>
where
    Self: Unpin,
    D: Decoder<'a>,
{
    type Item = Result<Item, D::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        loop {
            match take(this) {
                ItemStream::Done => return Poll::Ready(None),
                ItemStream::Idle(inner) => *this = ItemStream::Pending(inner.next()),
                ItemStream::Pending(mut fut) => match Pin::new(&mut fut).poll(cx) {
                    Poll::Pending => {
                        *this = ItemStream::Pending(fut);

                        return Poll::Pending;
                    }
                    Poll::Ready(Err(err)) => return Poll::Ready(Some(Err(err))),
                    Poll::Ready(Ok(None)) => return Poll::Ready(None),
                    Poll::Ready(Ok(Some((inner, item)))) => {
                        *this = ItemStream::Idle(inner);

                        return Poll::Ready(Some(Ok(item)));
                    }
                },
            };
        }
    }
}
