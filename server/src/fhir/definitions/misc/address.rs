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
use resources::misc::{Address, AddressType, Line, LineExtension};

use crate::fhir::{
    decode::{decode_any, DataStream, Decode, DecodeError, DecodeStream, Fields, Search},
    encode::{encode_any, DataStorage, Encode, EncodeError, EncodeStream},
};

use super::super::primitives::{decode_code, encode_code, CodeEx};

/* Decode */

#[async_trait(?Send)]
impl Decode for Address {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["type", "line", "city", "postalCode", "country"]);

        stream.element().await?;

        let type_ = stream.decode(&mut fields, decode_code).await?;
        let lines = stream.decode_vec(&mut fields, decode_any).await?;
        let city = stream.decode_opt(&mut fields, decode_any).await?;
        let zip_code = stream.decode_opt(&mut fields, decode_any).await?;
        let country = stream.decode_opt(&mut fields, decode_any).await?;

        stream.end().await?;

        Ok(Address {
            type_,
            lines,
            city,
            zip_code,
            country,
        })
    }
}

#[async_trait(?Send)]
impl Decode for Line {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["extension"]);

        let value = stream.value_extended().await?;
        let extensions = stream.decode_vec(&mut fields, decode_any).await?;
        stream.end().await?;

        Ok(Line { value, extensions })
    }
}

#[async_trait(?Send)]
impl Decode for LineExtension {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        stream.element().await?;

        let url = stream.value(Search::Exact("url")).await?.unwrap();

        let ext = match url.as_str() {
            x if icase_eq(x, URL_STREET) => {
                let mut fields = Fields::new(&["valueString"]);

                LineExtension::Street(stream.decode(&mut fields, decode_any).await?)
            }
            x if icase_eq(x, URL_NUMBER) => {
                let mut fields = Fields::new(&["valueString"]);

                LineExtension::Number(stream.decode(&mut fields, decode_any).await?)
            }
            x if icase_eq(x, URL_ADDITION) => {
                let mut fields = Fields::new(&["valueString"]);

                LineExtension::Addition(stream.decode(&mut fields, decode_any).await?)
            }
            x if icase_eq(x, URL_POSTBOX) => {
                let mut fields = Fields::new(&["valueString"]);

                LineExtension::Postbox(stream.decode(&mut fields, decode_any).await?)
            }
            x => {
                return Err(DecodeError::Custom {
                    message: format!("Unknown Address Line Extension: {}", &x),
                    path: stream.path().into(),
                })
            }
        };

        stream.end().await?;

        Ok(ext)
    }
}

/* Encode */

impl Encode for &Address {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode("type", &self.type_, encode_code)?
            .encode_vec("line", &self.lines, encode_any)?
            .encode_opt("city", &self.city, encode_any)?
            .encode_opt("postalCode", &self.zip_code, encode_any)?
            .encode_opt("country", &self.country, encode_any)?
            .end()?;

        Ok(())
    }
}

impl Encode for &Line {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .value_extended(&self.value)?
            .encode_vec("extension", &self.extensions, encode_any)?
            .end()?;

        Ok(())
    }
}

impl Encode for &LineExtension {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        match self {
            LineExtension::Street(s) => {
                stream
                    .element()?
                    .attrib("url", URL_STREET, encode_any)?
                    .encode("valueString", s, encode_any)?
                    .end()?;
            }
            LineExtension::Number(s) => {
                stream
                    .element()?
                    .attrib("url", URL_NUMBER, encode_any)?
                    .encode("valueString", s, encode_any)?
                    .end()?;
            }
            LineExtension::Addition(s) => {
                stream
                    .element()?
                    .attrib("url", URL_ADDITION, encode_any)?
                    .encode("valueString", s, encode_any)?
                    .end()?;
            }
            LineExtension::Postbox(s) => {
                stream
                    .element()?
                    .attrib("url", URL_POSTBOX, encode_any)?
                    .encode("valueString", s, encode_any)?
                    .end()?;
            }
        }

        Ok(())
    }
}

/* Misc */

impl CodeEx for AddressType {
    fn from_parts(value: String) -> Result<Self, String> {
        match value.as_str() {
            "both" => Ok(Self::Both),
            "postal" => Ok(Self::Postal),
            _ => Err(value),
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Both => "both",
            Self::Postal => "postal",
        }
    }
}

const URL_STREET: &str = "http://hl7.org/fhir/StructureDefinition/iso21090-ADXP-streetName";
const URL_NUMBER: &str = "http://hl7.org/fhir/StructureDefinition/iso21090-ADXP-houseNumber";
const URL_ADDITION: &str =
    "http://hl7.org/fhir/StructureDefinition/iso21090-ADXP-additionalLocator";
const URL_POSTBOX: &str = "http://hl7.org/fhir/StructureDefinition/iso21090-ADXP-postBox";
