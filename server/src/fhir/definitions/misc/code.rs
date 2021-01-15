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
use resources::misc::Code;

use crate::fhir::{
    decode::{decode_any, DataStream, DecodeError, DecodeStream, Fields},
    encode::{encode_any, DataStorage, EncodeError, EncodeStream},
};

use super::super::primitives::{CodeableConceptEx, Coding};

#[async_trait(?Send)]
impl Coding for Code {
    async fn decode_coding<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["system", "code"]);

        stream.element().await?;

        let system = stream.decode(&mut fields, decode_any).await?;
        let code = stream.decode(&mut fields, decode_any).await?;

        stream.end().await?;

        Ok(Code { system, code })
    }

    fn encode_coding<S>(&self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode("system", &self.system, encode_any)?
            .encode("code", &self.code, encode_any)?
            .end()?;

        Ok(())
    }
}

impl CodeableConceptEx for Code {
    type Coding = Self;

    fn from_parts(coding: Self::Coding, _text: Option<String>) -> Self {
        coding
    }

    fn coding(&self) -> &Self::Coding {
        &self
    }
}
