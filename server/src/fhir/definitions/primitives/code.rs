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
    decode::{DataStream, DecodeError, DecodeStream, Search},
    encode::{DataStorage, EncodeError, EncodeStream},
};

pub async fn decode_code<T, S>(stream: &mut DecodeStream<S>) -> Result<T, DecodeError<S::Error>>
where
    T: Code + Send,
    S: DataStream,
{
    let value = T::decode_code(stream).await?;

    Ok(value)
}

pub fn encode_code<T, S>(
    value: &T,
    stream: &mut EncodeStream<S>,
) -> Result<(), EncodeError<S::Error>>
where
    T: Code,
    S: DataStorage,
{
    value.encode_code(stream)
}

pub trait CodeEx: Sized {
    fn from_parts(value: String) -> Result<Self, String>;

    fn code(&self) -> &'static str;
}

#[async_trait(?Send)]
pub trait Code: Sized {
    async fn decode_code<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream;

    fn encode_code<S>(&self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage;
}

#[async_trait(?Send)]
impl<T: CodeEx> Code for T {
    async fn decode_code<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let value = stream.value(Search::Any).await?.unwrap();
        let value = Self::from_parts(value).map_err(|value| DecodeError::InvalidValue {
            value,
            path: stream.path().into(),
        })?;

        Ok(value)
    }

    fn encode_code<S>(&self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream.value(self.code())?;

        Ok(())
    }
}
