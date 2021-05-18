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
use miscellaneous::str::icase_eq;
use resources::misc::{Family, Name, Prefix};

use crate::fhir::{
    decode::{decode_any, DataStream, Decode, DecodeError, DecodeStream, Fields, Search},
    encode::{encode_any, DataStorage, Encode, EncodeError, EncodeStream},
};

/* Decode */

#[async_trait(?Send)]
impl Decode for Name {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["use", "family", "given", "prefix"]);

        stream.element().await?;

        let _use = stream.fixed(&mut fields, "official").await?;
        let family = stream.decode(&mut fields, decode_any).await?;
        let given = stream.decode(&mut fields, decode_any).await?;
        let prefix = stream.decode_opt(&mut fields, decode_any).await?;

        stream.end().await?;

        Ok(Name {
            given,
            family,
            prefix,
        })
    }
}

#[async_trait(?Send)]
impl Decode for Family {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut prefix = None;
        let mut family = None;
        let mut extension = None;

        let value = stream.value_extended().await?;

        let mut fields = Fields::new(&["extension"]);
        while stream.begin_substream_vec(&mut fields).await? {
            stream.element().await?;

            let url = stream.value(Search::Exact("url")).await?.unwrap();

            match url.as_str() {
                x if icase_eq(x, URL_FAMILY_PREFIX) => {
                    let mut fields = Fields::new(&["valueString"]);

                    prefix = Some(stream.decode(&mut fields, decode_any).await?);
                }
                x if icase_eq(x, URL_FAMILY_FAMILY) => {
                    let mut fields = Fields::new(&["valueString"]);

                    family = Some(stream.decode(&mut fields, decode_any).await?);
                }
                x if icase_eq(x, URL_FAMILY_EXTENSION) => {
                    let mut fields = Fields::new(&["valueString"]);

                    extension = Some(stream.decode(&mut fields, decode_any).await?);
                }
                _ => (),
            }

            stream.end().await?;
            stream.end_substream().await?;
        }

        stream.end().await?;

        Ok(Family {
            value,
            prefix,
            family,
            extension,
        })
    }
}

#[async_trait(?Send)]
impl Decode for Prefix {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut qualifier = false;

        let value = stream.value_extended().await?;

        let mut fields = Fields::new(&["extension"]);
        while stream.begin_substream_vec(&mut fields).await? {
            stream.element().await?;

            let url = stream.value(Search::Exact("url")).await?.unwrap();

            if icase_eq(url, URL_PREFIX_QUALIFIER) {
                let mut fields = Fields::new(&["valueCode"]);

                stream.fixed(&mut fields, "AC").await?;

                qualifier = true;
            }

            stream.end().await?;
            stream.end_substream().await?;
        }

        stream.end().await?;

        Ok(Prefix { value, qualifier })
    }
}

/* Encode */

impl Encode for &Name {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode("use", "official", encode_any)?
            .encode("family", &self.family, encode_any)?
            .encode_vec("given", once(&self.given), encode_any)?
            .encode_vec("prefix", &self.prefix, encode_any)?
            .end()?;

        Ok(())
    }
}

impl Encode for &Family {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .value_extended(&self.value)?
            .field_name("extension")?
            .array()?;

        if let Some(prefix) = &self.prefix {
            stream
                .element()?
                .attrib("url", URL_FAMILY_PREFIX, encode_any)?
                .encode("valueString", prefix, encode_any)?
                .end()?;
        }

        if let Some(family) = &self.family {
            stream
                .element()?
                .attrib("url", URL_FAMILY_FAMILY, encode_any)?
                .encode("valueString", family, encode_any)?
                .end()?;
        }

        if let Some(extension) = &self.extension {
            stream
                .element()?
                .attrib("url", URL_FAMILY_EXTENSION, encode_any)?
                .encode("valueString", extension, encode_any)?
                .end()?;
        }

        stream.end()?.end()?;

        Ok(())
    }
}

impl Encode for &Prefix {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .value_extended(&self.value)?
            .field_name("extension")?
            .array()?;

        if self.qualifier {
            stream
                .element()?
                .attrib("url", URL_PREFIX_QUALIFIER, encode_any)?
                .encode("valueCode", "AC", encode_any)?
                .end()?;
        }

        stream.end()?.end()?;

        Ok(())
    }
}

const URL_FAMILY_PREFIX: &str = "http://hl7.org/fhir/StructureDefinition/humanname-own-prefix";
const URL_FAMILY_FAMILY: &str = "http://hl7.org/fhir/StructureDefinition/humanname-own-name";
const URL_FAMILY_EXTENSION: &str = "http://fhir.de/StructureDefinition/humanname-namenszusatz";

const URL_PREFIX_QUALIFIER: &str = "http://hl7.org/fhir/StructureDefinition/iso21090-EN-qualifier";
