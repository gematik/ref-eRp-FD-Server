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

use actix_codec::Encoder;
use actix_http::{
    h1::{Codec, Message},
    Response,
};
use actix_web::{dev::MessageBody, error::Error as ActixError};
use bytes::{BufMut, BytesMut};
use futures::stream::TryStreamExt;

pub async fn encode(request_id: &str, res: Response) -> Result<BytesMut, ActixError> {
    let (head, mut body) = res.into_parts();

    let mut codec = Codec::default();
    let mut buffer = BytesMut::new();

    codec.encode(Message::Item((head, body.size())), &mut buffer)?;

    while let Some(bytes) = body.try_next().await? {
        codec.encode(Message::Chunk(Some(bytes)), &mut buffer)?;
    }

    codec.encode(Message::Chunk(None), &mut buffer)?;

    let mut ret = BytesMut::new();
    ret.reserve(3 + request_id.len() + buffer.len());
    ret.put_u8(0x31);
    ret.put_u8(0x20);
    ret.put(request_id.as_bytes());
    ret.put_u8(0x20);
    ret.put(buffer);

    Ok(ret)
}
