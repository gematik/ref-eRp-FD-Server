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

use std::fmt::{Debug, Display};

use bytes::{Buf, Bytes, BytesMut};
use futures::stream::{Stream, StreamExt};
use thiserror::Error;

pub struct ByteStream<S> {
    stream: S,
    buffer: Option<Bytes>,
}

#[derive(Error, Debug)]
pub enum StreamError<E>
where
    E: Display + Debug,
{
    #[error("Stream Error: {0}")]
    Stream(E),

    #[error("Unexpected End of File!")]
    UnexpectedEof,

    #[error("Unexpected Ident!")]
    UnexpectedIdent,

    #[error("Text is to large!")]
    TextTooLarge,
}

impl<S, E> ByteStream<S>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin,
    E: Display + Debug,
{
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            buffer: None,
        }
    }

    pub async fn peek(&mut self) -> Result<Option<u8>, StreamError<E>> {
        Ok(self.buffer().await?.map(|b| b[0]))
    }

    pub async fn take(&mut self) -> Result<Option<u8>, StreamError<E>> {
        Ok(self.buffer().await?.map(|b| b.get_u8()))
    }

    pub async fn take_while<F>(&mut self, mut f: F) -> Result<Option<Bytes>, StreamError<E>>
    where
        F: FnMut(usize, u8) -> bool,
    {
        let mut i = 0;
        let mut ret = BytesMut::new();

        loop {
            let buffer = match self.buffer().await? {
                Some(buffer) => buffer,
                None if ret.is_empty() => return Ok(None),
                None => return Ok(Some(ret.freeze())),
            };

            let mut j = 0;
            while j < buffer.len() && f(i, buffer[j]) {
                i += 1;
                j += 1;
            }

            if ret.len() + j > MAX_TEXT_SIZE {
                return Err(StreamError::TextTooLarge);
            }

            ret.extend_from_slice(&buffer[..j]);
            buffer.advance(j);

            if !buffer.is_empty() {
                return Ok(Some(ret.freeze()));
            }
        }
    }

    pub async fn drop_while<F>(&mut self, mut f: F) -> Result<(), StreamError<E>>
    where
        F: FnMut(usize, u8) -> bool,
    {
        let mut i = 0;

        loop {
            let buffer = match self.buffer().await? {
                Some(buffer) => buffer,
                None => return Ok(()),
            };

            let mut j = 0;
            while j < buffer.len() && f(i, buffer[j]) {
                i += 1;
                j += 1;
            }

            buffer.advance(j);

            if !buffer.is_empty() {
                return Ok(());
            }
        }
    }

    pub async fn drop_whitespaces(&mut self) -> Result<(), StreamError<E>> {
        self.drop_while(|_, v| v == b' ' || v == b'\t' || v == b'\n' || v == b'\r')
            .await
    }

    pub async fn expect(&mut self, mut expect: &[u8]) -> Result<(), StreamError<E>> {
        while !expect.is_empty() {
            let buffer = match self.buffer().await? {
                Some(buffer) => buffer,
                None => return Err(StreamError::UnexpectedEof),
            };

            if expect.len() > buffer.len() {
                if !expect.starts_with(buffer) {
                    return Err(StreamError::UnexpectedIdent);
                }

                buffer.advance(buffer.len());

                expect = &expect[buffer.len()..];
            } else if !buffer.starts_with(expect) {
                return Err(StreamError::UnexpectedIdent);
            } else {
                buffer.advance(expect.len());

                break;
            }
        }

        Ok(())
    }

    pub async fn buffer(&mut self) -> Result<Option<&mut Bytes>, StreamError<E>> {
        let buffer = match self.buffer.take() {
            Some(buffer) if !buffer.is_empty() => buffer,
            _ => loop {
                let buffer = match self.stream.next().await {
                    Some(buffer) => buffer.map_err(StreamError::Stream)?,
                    None => return Ok(None),
                };

                if !buffer.is_empty() {
                    break buffer;
                }
            },
        };

        self.buffer = Some(buffer);

        Ok(self.buffer.as_mut())
    }
}

const MAX_TEXT_SIZE: usize = 1024 * 1024; // 1 MB
