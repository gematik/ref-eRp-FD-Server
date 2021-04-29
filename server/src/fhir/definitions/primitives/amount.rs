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

use crate::fhir::{
    decode::{decode_any, DataStream, DecodeError, DecodeStream, Fields},
    encode::{encode_any, DataStorage, EncodeError, EncodeStream},
};

pub async fn decode_amount<T, S>(stream: &mut DecodeStream<S>) -> Result<T, DecodeError<S::Error>>
where
    T: Amount + Send,
    S: DataStream,
{
    let value = T::decode_amount(stream).await?;

    Ok(value)
}

pub fn encode_amount<T, S>(
    value: &T,
    stream: &mut EncodeStream<S>,
) -> Result<(), EncodeError<S::Error>>
where
    T: Amount,
    S: DataStorage,
{
    value.encode_amount(stream)
}

pub trait AmountEx {
    fn from_parts(numerator: usize, denominator: usize, unit: String, code: Option<String>)
        -> Self;

    fn unit(&self) -> &String;

    fn numerator(&self) -> usize;

    fn code(&self) -> &Option<String> {
        &None
    }

    fn system() -> AmountSystem {
        AmountSystem::None
    }

    fn denominator(&self) -> usize {
        1
    }
}

#[allow(dead_code)]
pub enum AmountSystem {
    None,
    Fixed(&'static str),
    OptionalFixed(&'static str),
}

#[async_trait(?Send)]
pub trait Amount: Sized {
    async fn decode_amount<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream;

    fn encode_amount<S>(&self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage;
}

#[async_trait(?Send)]
impl<T: AmountEx> Amount for T {
    async fn decode_amount<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["numerator", "denominator"]);
        let mut fields_numerator = Fields::new(&["value", "unit", "system", "code"]);
        let mut fields_denominator = Fields::new(&["value"]);

        stream.root("Amount").await?;

        stream.begin_substream(&mut fields).await?;
        stream.element().await?;

        let numerator = stream.decode(&mut fields_numerator, decode_any).await?;
        let unit = stream.decode(&mut fields_numerator, decode_any).await?;
        match Self::system() {
            AmountSystem::None => {
                fields_numerator.next();
            }
            AmountSystem::Fixed(s) => stream.ifixed(&mut fields_numerator, s).await?,
            AmountSystem::OptionalFixed(expected) => {
                let actual = stream
                    .decode_opt::<Option<String>, _>(&mut fields_numerator, decode_any)
                    .await?;
                if let Some(actual) = actual {
                    if actual != expected {
                        return Err(DecodeError::InvalidFixedValue {
                            actual: Some(actual).into(),
                            expected: Some(expected.to_owned()).into(),
                            path: stream.path().into(),
                        });
                    }
                }
            }
        }
        let code = stream.decode_opt(&mut fields_numerator, decode_any).await?;

        stream.end().await?;
        stream.end_substream().await?;

        stream.begin_substream(&mut fields).await?;
        stream.element().await?;

        let denominator = stream.decode(&mut fields_denominator, decode_any).await?;

        stream.end().await?;
        stream.end_substream().await?;

        stream.end().await?;

        let value = Self::from_parts(numerator, denominator, unit, code);

        Ok(value)
    }

    fn encode_amount<S>(&self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .field_name("numerator")?
            .element()?
            .encode("value", self.numerator(), encode_any)?
            .encode("unit", self.unit(), encode_any)?;

        match Self::system() {
            AmountSystem::None => (),
            AmountSystem::Fixed(s) => {
                stream.encode("system", s, encode_any)?;
            }
            AmountSystem::OptionalFixed(s) => {
                stream.encode("system", s, encode_any)?;
            }
        }

        stream
            .encode_opt("code", self.code(), encode_any)?
            .end()?
            .field_name("denominator")?
            .element()?
            .encode("value", self.denominator(), encode_any)?
            .end()?
            .end()?;

        Ok(())
    }
}
