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

use async_trait::async_trait;

use crate::fhir::{
    decode::{decode_any, DataStream, DecodeError, DecodeStream, Fields},
    encode::{encode_any, DataStorage, EncodeError, EncodeStream},
};

pub async fn decode_binary<T, S>(stream: &mut DecodeStream<S>) -> Result<T, DecodeError<S::Error>>
where
    T: Binary + Send,
    S: DataStream,
{
    let value = T::decode_binary(stream).await?;

    Ok(value)
}

pub fn encode_binary<T, S>(
    value: &T,
    stream: &mut EncodeStream<S>,
) -> Result<(), EncodeError<S::Error>>
where
    T: Binary,
    S: DataStorage,
{
    value.encode_binary(stream)
}

pub trait BinaryEx: Sized {
    fn from_parts(data: String) -> Result<Self, String>;

    fn data(&self) -> String;

    fn content_type() -> Option<&'static str>;
}

#[async_trait(?Send)]
pub trait Binary: Sized {
    async fn decode_binary<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream;

    fn encode_binary<S>(&self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage;
}

#[async_trait(?Send)]
impl<T: BinaryEx + Send> Binary for T {
    async fn decode_binary<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["contentType", "data"]);

        stream.root("Binary").await?;

        let _content_type = stream.fixed_opt(&mut fields, Self::content_type()).await?;
        let data = stream.decode(&mut fields, decode_any).await?;

        stream.end().await?;

        let value = Self::from_parts(data).map_err(|value| DecodeError::InvalidValue {
            value,
            path: stream.path().into(),
        })?;

        Ok(value)
    }

    fn encode_binary<S>(&self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .root("Binary")?
            .encode_opt("contentType", Self::content_type(), encode_any)?
            .encode("data", self.data(), encode_any)?
            .end()?;

        Ok(())
    }
}
