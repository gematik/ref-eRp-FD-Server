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

use async_trait::async_trait;

use crate::fhir::{
    decode::{DataStream, DecodeError, DecodeStream, Fields},
    encode::{encode_any, DataStorage, EncodeError, EncodeStream},
};

use super::{decode_code, encode_code, Code};

pub async fn decode_coding<T, S>(stream: &mut DecodeStream<S>) -> Result<T, DecodeError<S::Error>>
where
    T: Coding + Send,
    S: DataStream,
{
    let value = T::decode_coding(stream).await?;

    Ok(value)
}

pub fn encode_coding<T, S>(
    value: &T,
    stream: &mut EncodeStream<S>,
) -> Result<(), EncodeError<S::Error>>
where
    T: Coding,
    S: DataStorage,
{
    value.encode_coding(stream)
}

pub trait CodingEx: Sized {
    type Code: Code + Send;

    fn from_parts(code: Self::Code) -> Self;

    fn code(&self) -> &Self::Code;

    fn display(&self) -> Option<&'static str> {
        None
    }

    fn system() -> Option<&'static str> {
        None
    }
}

#[async_trait(?Send)]
pub trait Coding: Sized {
    async fn decode_coding<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream;

    fn encode_coding<S>(&self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage;
}

#[async_trait(?Send)]
impl<T: CodingEx> Coding for T {
    async fn decode_coding<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["system", "code"]);

        stream.element().await?;

        let _system = stream.fixed_opt(&mut fields, Self::system()).await?;
        let code = stream.decode(&mut fields, decode_code).await?;

        stream.end().await?;

        let value = Self::from_parts(code);

        Ok(value)
    }

    fn encode_coding<S>(&self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode_opt("system", Self::system(), encode_any)?
            .encode("code", self.code(), encode_code)?
            .encode_opt("display", self.display(), encode_any)?
            .end()?;

        Ok(())
    }
}
