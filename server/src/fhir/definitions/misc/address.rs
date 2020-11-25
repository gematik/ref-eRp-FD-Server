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
use resources::misc::Address;

use crate::fhir::{
    decode::{decode_any, DataStream, Decode, DecodeError, DecodeStream, Fields, Search},
    encode::{encode_any, DataStorage, Encode, EncodeError, EncodeStream},
};

/* Decode */

#[async_trait(?Send)]
impl Decode for Address {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut street = None;
        let mut number = None;
        let mut addition = None;
        let mut post_box = None;

        let mut fields = Fields::new(&["type", "line", "city", "postalCode", "country"]);

        stream.element().await?;

        let _type = stream.fixed(&mut fields, "both").await?;

        stream.begin_substream(&mut fields).await?;
        let address = stream.value_extended().await?;
        {
            let mut fields = Fields::new(&["extension"]);
            while stream.begin_substream_vec(&mut fields).await? {
                stream.element().await?;

                let url = stream.value(Search::Exact("url")).await?.unwrap();

                match url.as_str() {
                    x if icase_eq(x, URL_STREET) => {
                        let mut fields = Fields::new(&["valueString"]);

                        street = Some(stream.decode(&mut fields, decode_any).await?);
                    }
                    x if icase_eq(x, URL_NUMBER) => {
                        let mut fields = Fields::new(&["valueString"]);

                        number = Some(stream.decode(&mut fields, decode_any).await?);
                    }
                    x if icase_eq(x, URL_ADDITION) => {
                        let mut fields = Fields::new(&["valueString"]);

                        addition = Some(stream.decode(&mut fields, decode_any).await?);
                    }
                    x if icase_eq(x, URL_POSTBOX) => {
                        let mut fields = Fields::new(&["valueString"]);

                        post_box = Some(stream.decode(&mut fields, decode_any).await?);
                    }
                    _ => (),
                }

                stream.end().await?;
                stream.end_substream().await?;
            }
        }
        stream.end().await?;
        stream.end_substream().await?;

        let city = stream.decode_opt(&mut fields, decode_any).await?;
        let zip_code = stream.decode_opt(&mut fields, decode_any).await?;
        let country = stream.decode_opt(&mut fields, decode_any).await?;

        stream.end().await?;

        Ok(Address {
            address,
            street,
            number,
            addition,
            post_box,
            city,
            zip_code,
            country,
        })
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
            .encode("type", "both", encode_any)?
            .field_name("line")?
            .array()?
            .value_extended(&self.address)?
            .field_name("extension")?
            .array()?;

        if let Some(street) = &self.street {
            stream
                .element()?
                .attrib("url", URL_STREET, encode_any)?
                .encode("valueString", street, encode_any)?
                .end()?;
        }

        if let Some(number) = &self.number {
            stream
                .element()?
                .attrib("url", URL_NUMBER, encode_any)?
                .encode("valueString", number, encode_any)?
                .end()?;
        }

        if let Some(addition) = &self.addition {
            stream
                .element()?
                .attrib("url", URL_NUMBER, encode_any)?
                .encode("valueString", addition, encode_any)?
                .end()?;
        }

        if let Some(post_box) = &self.post_box {
            stream
                .element()?
                .attrib("url", URL_NUMBER, encode_any)?
                .encode("valueString", post_box, encode_any)?
                .end()?;
        }

        stream
            .end()?
            .end()?
            .end()?
            .encode_opt("city", &self.city, encode_any)?
            .encode_opt("postalCode", &self.zip_code, encode_any)?
            .encode_opt("country", &self.country, encode_any)?
            .end()?;

        Ok(())
    }
}

const URL_STREET: &str = "http://hl7.org/fhir/StructureDefinition/iso21090-ADXP-streetName";
const URL_NUMBER: &str = "http://hl7.org/fhir/StructureDefinition/iso21090-ADXP-houseNumber";
const URL_ADDITION: &str =
    "http://hl7.org/fhir/StructureDefinition/iso21090-ADXP-additionalLocator";
const URL_POSTBOX: &str = "http://hl7.org/fhir/StructureDefinition/iso21090-ADXP-postBox";
