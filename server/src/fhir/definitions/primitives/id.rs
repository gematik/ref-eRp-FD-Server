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

use std::convert::TryInto;

use async_trait::async_trait;
use resources::primitives::Id;

use crate::fhir::{
    decode::{DataStream, Decode, DecodeError, DecodeStream, Search},
    encode::{DataStorage, Encode, EncodeError, EncodeStream},
};

use super::ReferenceEx;

#[async_trait(?Send)]
impl Decode for Id {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let value = stream.value(Search::Any).await?.unwrap();
        let value = value
            .try_into()
            .map_err(|value| DecodeError::InvalidValue {
                value,
                path: stream.path().into(),
            })?;

        Ok(value)
    }
}

impl Encode for &Id {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream.value((**self).clone())?;

        Ok(())
    }
}

impl ReferenceEx for Id {
    fn from_parts(value: String) -> Result<Self, String> {
        if let Some(Ok(id)) = value.strip_prefix('#').map(TryInto::try_into) {
            return Ok(id);
        }

        Err(value)
    }

    fn reference(&self) -> String {
        format!("#{}", self)
    }
}
