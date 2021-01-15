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

use bytes::Bytes;
use futures::{
    future::{FutureExt, LocalBoxFuture},
    stream::{Stream, StreamExt},
};

use super::{
    super::{
        byte_stream::{ByteStream, Encoder},
        item::Item,
    },
    error::Error,
    writer::Writer,
};

pub struct Json<S> {
    stream: S,
    writer: Writer,
}

impl<S> Json<S>
where
    S: Stream<Item = Item> + Send + Unpin + 'static,
{
    pub fn new(stream: S) -> ByteStream<Self> {
        ByteStream::Idle(Json {
            stream,
            writer: Writer::default(),
        })
    }
}

impl<S> Encoder for Json<S>
where
    S: Stream<Item = Item> + Send + Unpin + 'static,
{
    type Error = Error;

    fn next(mut self) -> LocalBoxFuture<'static, Result<Option<(Self, Bytes)>, Error>> {
        async move {
            let item = self.stream.next().await;
            if self.writer.write(item)? {
                let bytes = self.writer.freeze();

                Ok(Some((self, bytes)))
            } else {
                Ok(None)
            }
        }
        .boxed_local()
    }
}
