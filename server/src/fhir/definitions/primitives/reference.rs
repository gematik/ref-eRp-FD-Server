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

pub async fn decode_reference<T, S>(
    stream: &mut DecodeStream<S>,
) -> Result<T, DecodeError<S::Error>>
where
    T: Reference + Send,
    S: DataStream,
{
    let value = T::decode_reference(stream).await?;

    Ok(value)
}

pub fn encode_reference<T, S>(
    value: &T,
    stream: &mut EncodeStream<S>,
) -> Result<(), EncodeError<S::Error>>
where
    T: Reference,
    S: DataStorage,
{
    value.encode_reference(stream)?;

    Ok(())
}

pub trait ReferenceEx: Sized {
    fn from_parts(reference: String) -> Result<Self, String>;

    fn reference(&self) -> String;
}

#[async_trait(?Send)]
pub trait Reference: Sized {
    async fn decode_reference<S>(
        stream: &mut DecodeStream<S>,
    ) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream;

    fn encode_reference<S>(
        &self,
        stream: &mut EncodeStream<S>,
    ) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage;
}

impl ReferenceEx for String {
    fn from_parts(reference: String) -> Result<Self, String> {
        Ok(reference)
    }

    fn reference(&self) -> String {
        self.clone()
    }
}

#[async_trait(?Send)]
impl<T: ReferenceEx> Reference for T {
    async fn decode_reference<S>(
        stream: &mut DecodeStream<S>,
    ) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["reference"]);

        stream.element().await?;

        let reference = stream.decode(&mut fields, decode_any).await?;

        stream.end().await?;

        let value = Self::from_parts(reference).map_err(|value| DecodeError::InvalidValue {
            value,
            path: stream.path().into(),
        })?;

        Ok(value)
    }

    fn encode_reference<S>(&self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode("reference", &self.reference(), encode_any)?
            .end()?;

        Ok(())
    }
}
