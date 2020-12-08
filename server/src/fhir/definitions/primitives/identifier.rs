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
    decode::{decode_any, DataStream, DecodeError, DecodeStream, Fields},
    encode::{encode_any, DataStorage, EncodeError, EncodeStream},
};

pub async fn decode_identifier<T, S>(
    stream: &mut DecodeStream<S>,
) -> Result<T, DecodeError<S::Error>>
where
    T: Identifier,
    S: DataStream,
{
    let value = T::decode_identifier(stream).await?;

    Ok(value)
}

pub fn encode_identifier<T, S>(
    value: &T,
    stream: &mut EncodeStream<S>,
) -> Result<(), EncodeError<S::Error>>
where
    T: Identifier,
    S: DataStorage,
{
    value.encode_identifier(stream)?;

    Ok(())
}

pub async fn decode_identifier_reference<T, S>(
    stream: &mut DecodeStream<S>,
) -> Result<T, DecodeError<S::Error>>
where
    T: Identifier,
    S: DataStream,
{
    let mut fields = Fields::new(&["identifier"]);

    stream.element().await?;

    let value = stream.decode(&mut fields, decode_identifier).await?;

    stream.end().await?;

    Ok(value)
}

pub fn encode_identifier_reference<T, S>(
    value: &T,
    stream: &mut EncodeStream<S>,
) -> Result<(), EncodeError<S::Error>>
where
    T: Identifier,
    S: DataStorage,
{
    stream
        .element()?
        .encode("identifier", value, encode_identifier)?
        .end()?;

    Ok(())
}

pub trait IdentifierEx: Sized {
    fn from_parts(value: String) -> Result<Self, String>;

    fn value(&self) -> String;

    fn system() -> Option<&'static str> {
        None
    }
}

#[async_trait(?Send)]
pub trait Identifier: Sized {
    async fn decode_identifier<S>(
        stream: &mut DecodeStream<S>,
    ) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream;

    fn encode_identifier<S>(
        &self,
        stream: &mut EncodeStream<S>,
    ) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage;
}

#[async_trait(?Send)]
impl<T: IdentifierEx> Identifier for T {
    async fn decode_identifier<S>(
        stream: &mut DecodeStream<S>,
    ) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["system", "value"]);

        stream.element().await?;

        let _system = stream.ifixed_opt(&mut fields, Self::system()).await?;
        let value = stream.decode(&mut fields, decode_any).await?;

        stream.end().await?;

        let value = Self::from_parts(value).map_err(|value| DecodeError::InvalidValue {
            value,
            path: stream.path().into(),
        })?;

        Ok(value)
    }

    fn encode_identifier<S>(
        &self,
        stream: &mut EncodeStream<S>,
    ) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode_opt("system", Self::system(), encode_any)?
            .encode("value", self.value(), encode_any)?
            .end()?;

        Ok(())
    }
}
