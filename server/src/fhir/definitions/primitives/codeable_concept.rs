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

use std::iter::once;

use async_trait::async_trait;

use crate::fhir::{
    decode::{decode_any, DataStream, DecodeError, DecodeStream, Fields},
    encode::{encode_any, DataStorage, EncodeError, EncodeStream},
};

use super::{decode_coding, encode_coding, Coding};

pub async fn decode_codeable_concept<T, S>(
    stream: &mut DecodeStream<S>,
) -> Result<T, DecodeError<S::Error>>
where
    T: CodeableConcept + Send,
    S: DataStream,
{
    let value = T::decode_codeable_concept(stream).await?;

    Ok(value)
}

pub fn encode_codeable_concept<T, S>(
    value: &T,
    stream: &mut EncodeStream<S>,
) -> Result<(), EncodeError<S::Error>>
where
    T: CodeableConcept,
    S: DataStorage,
{
    value.encode_codeable_concept(stream)
}

pub trait CodeableConceptEx {
    type Coding: Coding + Send;

    fn from_parts(coding: Self::Coding, text: Option<String>) -> Self;

    fn coding(&self) -> &Self::Coding;

    fn text(&self) -> &Option<String> {
        &None
    }
}

#[async_trait(?Send)]
pub trait CodeableConcept: Sized {
    async fn decode_codeable_concept<S>(
        stream: &mut DecodeStream<S>,
    ) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream;

    fn encode_codeable_concept<S>(
        &self,
        stream: &mut EncodeStream<S>,
    ) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage;
}

#[async_trait(?Send)]
impl<T: CodeableConceptEx> CodeableConcept for T {
    async fn decode_codeable_concept<S>(
        stream: &mut DecodeStream<S>,
    ) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["coding", "text"]);

        stream.element().await?;

        let code = stream.decode(&mut fields, decode_coding).await?;
        let text = stream.decode_opt(&mut fields, decode_any).await?;

        stream.end().await?;

        let value = Self::from_parts(code, text);

        Ok(value)
    }

    fn encode_codeable_concept<S>(
        &self,
        stream: &mut EncodeStream<S>,
    ) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode_vec("coding", once(self.coding()), encode_coding)?
            .encode_opt("text", self.text(), encode_any)?
            .end()?;

        Ok(())
    }
}

#[async_trait(?Send)]
impl CodeableConcept for String {
    async fn decode_codeable_concept<S>(
        stream: &mut DecodeStream<S>,
    ) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["text"]);

        stream.element().await?;

        let text = stream.decode(&mut fields, decode_any).await?;

        stream.end().await?;

        Ok(text)
    }

    fn encode_codeable_concept<S>(
        &self,
        stream: &mut EncodeStream<S>,
    ) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream.element()?.encode("text", self, encode_any)?.end()?;

        Ok(())
    }
}
