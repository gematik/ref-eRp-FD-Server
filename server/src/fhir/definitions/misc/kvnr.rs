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

use std::convert::TryInto;

use async_trait::async_trait;
use resources::misc::Kvnr;

use crate::fhir::{
    decode::{DataStream, Decode, DecodeError, DecodeStream, Search},
    encode::{DataStorage, Encode, EncodeError, EncodeStream},
};

use super::super::primitives::IdentifierEx;

#[async_trait(?Send)]
impl Decode for Kvnr {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let value = stream.value(Search::Any).await?.unwrap();
        let value = Kvnr::new(value).map_err(|value| DecodeError::InvalidValue {
            value,
            path: stream.path().into(),
        })?;

        Ok(value)
    }
}

impl Encode for &Kvnr {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let value: String = self.clone().into();

        stream.value(value)?;

        Ok(())
    }
}

impl IdentifierEx for Kvnr {
    fn from_parts(value: String) -> Result<Self, String> {
        value.try_into()
    }

    fn value(&self) -> String {
        self.to_string()
    }

    fn system() -> Option<&'static str> {
        Some("http://fhir.de/NamingSystem/gkv/kvid-10")
    }
}
