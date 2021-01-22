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
use resources::primitives::{Id, Instant};

use crate::fhir::{
    decode::{decode_any, DataStream, Decode, DecodeError, DecodeStream, Fields},
    encode::{encode_any, DataStorage, Encode, EncodeError, EncodeStream},
};

#[derive(Debug, Default)]
pub struct Meta {
    pub version_id: Option<Id>,
    pub last_updated: Option<Instant>,
    pub profiles: Vec<String>,
}

#[async_trait(?Send)]
impl Decode for Meta {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["versionId", "lastUpdated", "profile"]);

        stream.root("Meta").await?;

        let version_id = stream.decode_opt(&mut fields, decode_any).await?;
        let last_updated = stream.decode_opt(&mut fields, decode_any).await?;
        let profiles = stream.decode_vec(&mut fields, decode_any).await?;

        stream.end().await?;

        Ok(Meta {
            version_id,
            last_updated,
            profiles,
        })
    }
}

impl Encode for Meta {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode_opt("versionId", &self.version_id, encode_any)?
            .encode_opt("lastUpdated", &self.last_updated, encode_any)?
            .encode_vec("profile", self.profiles, encode_any)?
            .end()?;

        Ok(())
    }
}

impl Encode for &Meta {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode_vec("profile", &self.profiles, encode_any)?
            .end()?;

        Ok(())
    }
}
