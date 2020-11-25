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
use miscellaneous::str::icase_eq;
use resources::misc::InsuranceId;

use crate::fhir::{
    decode::{decode_any, DataStream, DecodeError, DecodeStream, Fields},
    definitions::primitives::Identifier,
    encode::{encode_any, DataStorage, EncodeError, EncodeStream},
};

#[async_trait(?Send)]
impl Identifier for InsuranceId {
    async fn decode_identifier<S>(
        stream: &mut DecodeStream<S>,
    ) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["system", "value"]);

        stream.element().await?;

        let system = stream.decode::<String, _>(&mut fields, decode_any).await?;
        let value = stream.decode(&mut fields, decode_any).await?;

        stream.end().await?;

        match system.as_str() {
            x if icase_eq(x, SYSTEM_IKNR) => Ok(InsuranceId::Iknr(value)),
            _ => Err(DecodeError::InvalidValue {
                value: system,
                path: stream.path().into(),
            }),
        }
    }

    fn encode_identifier<S>(
        &self,
        stream: &mut EncodeStream<S>,
    ) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let (system, value) = match self {
            Self::Iknr(s) => (SYSTEM_IKNR, s.clone()),
        };

        stream
            .element()?
            .encode("system", system, encode_any)?
            .encode("value", value, encode_any)?
            .end()?;

        Ok(())
    }
}

const SYSTEM_IKNR: &str = "http://fhir.de/NamingSystem/arge-ik/iknr";
