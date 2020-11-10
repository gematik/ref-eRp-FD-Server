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

use std::borrow::Cow;

use async_trait::async_trait;

use crate::fhir::{
    decode::{decode_any, DataStream, Decode, DecodeError, DecodeStream},
    encode::{encode_any, DataStorage, Encode, EncodeError, EncodeStream},
};

/* Decode */

#[async_trait(?Send)]
impl<T> Decode for Cow<'static, T>
where
    T: Decode + Clone + 'static,
{
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {

        let value = decode_any(stream).await?;

        Ok(Cow::Owned(value))
    }
}

/* Encode */

impl<'a, T> Encode for &'a Cow<'a, T>
where
    T: Clone,
    &'a T: Encode
{
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        encode_any(self.as_ref(), stream)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use futures::stream::once;

    use crate::fhir::decode::Item as DecodeItem;

    #[tokio::test]
    async fn test_decode() {
        let stream = once(DecodeItem::Field {
            name: "fuu".into(),
            value: "bar".into(),
            extension: Vec::new(),
        });
        let mut stream = DecodeStream::new(stream);

        let decoded: Cow<'static, str> = stream.decode(&mut Fields::Any, decode_any).await.unwrap();
    }

    #[tokio::test]
    async fn test_encode() {
    }
}
