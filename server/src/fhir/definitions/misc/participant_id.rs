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
use miscellaneous::str::icase_eq;
use resources::misc::{Kvnr, ParticipantId, TelematikId};

use crate::fhir::{
    decode::{decode_any, DataStream, DecodeError, DecodeStream, Fields},
    definitions::primitives::Identifier,
    encode::{encode_any, DataStorage, EncodeError, EncodeStream},
};

#[async_trait(?Send)]
impl Identifier for ParticipantId {
    async fn decode_identifier<S>(
        stream: &mut DecodeStream<S>,
    ) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["system", "value"]);

        stream.element().await?;

        let system: String = stream.decode(&mut fields, decode_any).await?;
        let value: String = stream.decode(&mut fields, decode_any).await?;

        stream.end().await?;

        let value = match system.as_str() {
            x if icase_eq(x, SYSTEM_KVNR) => {
                Self::Kvnr(Kvnr::new(value).map_err(|value| DecodeError::InvalidValue {
                    value,
                    path: stream.path().into(),
                })?)
            }
            x if icase_eq(x, SYSTEM_TELEMATIK_ID) => Self::TelematikId(TelematikId::new(value)),
            x => {
                return Err(DecodeError::Custom {
                    message: format!("Unknown system: {}", x),
                    path: stream.path().into(),
                })
            }
        };

        Ok(value)
    }

    fn encode_identifier<S>(
        &self,
        stream: &mut EncodeStream<S>,
    ) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let system = match self {
            Self::Kvnr(_) => SYSTEM_KVNR,
            Self::TelematikId(_) => SYSTEM_TELEMATIK_ID,
        };

        let value = match self {
            Self::Kvnr(v) => v.to_string(),
            Self::TelematikId(v) => v.to_string(),
        };

        stream
            .element()?
            .encode("system", system, encode_any)?
            .encode("value", value, encode_any)?
            .end()?;

        Ok(())
    }
}

const SYSTEM_KVNR: &str = "http://fhir.de/NamingSystem/gkv/kvid-10";
const SYSTEM_TELEMATIK_ID: &str = "https://gematik.de/fhir/NamingSystem/TelematikID";
