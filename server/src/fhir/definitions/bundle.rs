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
use resources::bundle::{Bundle, Entry, Identifier, Meta, Relation, Type};

use crate::fhir::{
    decode::{decode_any, DataStream, Decode, DecodeError, DecodeStream, Fields, Search},
    encode::{encode_any, DataStorage, Encode, EncodeError, EncodeStream},
};

use super::primitives::{
    decode_code, decode_identifier, encode_code, encode_identifier, CodeEx,
    Identifier as IdentifierTrait,
};

pub trait DecodeBundleResource: Decode {}
pub trait EncodeBundleResource: Encode {}

/* Decode */

#[async_trait(?Send)]
impl<T> Decode for Bundle<T>
where
    T: DecodeBundleResource,
{
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&[
            "id",
            "meta",
            "identifier",
            "type",
            "timestamp",
            "total",
            "link",
            "entry",
        ]);

        stream.root("Bundle").await?;

        let id = stream.decode_opt(&mut fields, decode_any).await?;
        let meta = stream.decode_opt(&mut fields, decode_any).await?;
        let identifier = stream.decode_opt(&mut fields, decode_identifier).await?;
        let type_ = stream.decode(&mut fields, decode_code).await?;
        let timestamp = stream.decode_opt(&mut fields, decode_any).await?;
        let total = stream.decode_opt(&mut fields, decode_any).await?;
        let link = stream.decode_vec(&mut fields, decode_any).await?;
        let entries = stream.decode_vec(&mut fields, decode_any).await?;

        stream.end().await?;

        Ok(Bundle {
            id,
            meta,
            identifier,
            type_,
            timestamp,
            total,
            link,
            entries,
        })
    }
}

#[async_trait(?Send)]
impl Decode for (Relation, String) {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["relation", "url"]);

        stream.element().await?;

        let relation = stream.decode(&mut fields, decode_any).await?;
        let url = stream.decode(&mut fields, decode_any).await?;

        stream.end().await?;

        Ok((relation, url))
    }
}

#[async_trait(?Send)]
impl<T> Decode for Entry<T>
where
    T: DecodeBundleResource,
{
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["fullUrl", "resource"]);

        stream.element().await?;

        let url = stream.decode_opt(&mut fields, decode_any).await?;
        let resource = stream.resource(&mut fields, decode_any).await?;

        stream.end().await?;

        Ok(Entry { url, resource })
    }
}

#[async_trait(?Send)]
impl Decode for Meta {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["lastUpdated", "profile"]);

        stream.root("Meta").await?;

        let last_updated = stream.decode_opt(&mut fields, decode_any).await?;
        let profile = stream.decode_vec(&mut fields, decode_any).await?;

        stream.end().await?;

        Ok(Meta {
            last_updated,
            profile,
        })
    }
}

#[async_trait(?Send)]
impl Decode for Relation {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let value = stream.value(Search::Any).await?.unwrap();

        #[allow(clippy::single_match)]
        match value.as_str() {
            "self" => Ok(Self::Self_),
            "first" => Ok(Self::First),
            "previous" => Ok(Self::Previous),
            "next" => Ok(Self::Next),
            "last" => Ok(Self::Last),
            _ => Err(DecodeError::InvalidValue {
                value,
                path: Default::default(),
            }),
        }
    }
}

/* Encode */

impl<'a, T> Encode for &'a Bundle<T>
where
    T: EncodeBundleResource + Clone + 'a,
{
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .root("Bundle")?
            .encode_opt("id", &self.id, encode_any)?
            .encode_opt("meta", &self.meta, encode_any)?
            .encode_opt("identifier", &self.identifier, encode_identifier)?
            .encode("type", &self.type_, encode_code)?
            .encode_opt("timestamp", &self.timestamp, encode_any)?
            .encode_opt("total", &self.total, encode_any)?
            .encode_vec("link", &self.link, encode_any)?
            .encode_vec("entry", &self.entries, encode_any)?
            .end()?;

        Ok(())
    }
}

impl Encode for &(Relation, String) {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode("relation", &self.0, encode_any)?
            .encode("url", &self.1, encode_any)?
            .end()?;

        Ok(())
    }
}

impl<'a, T> Encode for &'a Entry<T>
where
    T: EncodeBundleResource + Clone + 'a,
{
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode_opt("fullUrl", &self.url, encode_any)?
            .resource("resource", self.resource.clone(), encode_any)?
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
            .root("Meta")?
            .encode_opt("lastUpdated", &self.last_updated, encode_any)?
            .encode_vec("profile", &self.profile, encode_any)?
            .end()?;

        Ok(())
    }
}

impl Encode for &Relation {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let value = match self {
            Relation::Self_ => "self",
            Relation::First => "first",
            Relation::Previous => "previous",
            Relation::Next => "next",
            Relation::Last => "last",
        };

        stream.value(value)?;

        Ok(())
    }
}

/* Misc */

#[async_trait(?Send)]
impl IdentifierTrait for Identifier {
    async fn decode_identifier<S>(
        stream: &mut DecodeStream<S>,
    ) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["system", "value"]);

        stream.element().await?;

        let system = stream.decode_opt(&mut fields, decode_any).await?;
        let value = stream.decode_opt(&mut fields, decode_any).await?;

        stream.end().await?;

        Ok(Identifier { system, value })
    }

    fn encode_identifier<S>(
        &self,
        stream: &mut EncodeStream<S>,
    ) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode_opt("system", &self.system, encode_any)?
            .encode_opt("value", &self.value, encode_any)?
            .end()?;

        Ok(())
    }
}

impl CodeEx for Type {
    fn from_parts(value: String) -> Result<Self, String> {
        match value.as_str() {
            "document" => Ok(Self::Document),
            "message" => Ok(Self::Message),
            "transaction" => Ok(Self::Transaction),
            "transaction-response" => Ok(Self::TransactionResponse),
            "batch" => Ok(Self::Batch),
            "batch-response" => Ok(Self::BatchResponse),
            "history" => Ok(Self::History),
            "searchset" => Ok(Self::Searchset),
            "collection" => Ok(Self::Collection),
            _ => Err(value),
        }
    }

    fn code(&self) -> &'static str {
        match *self {
            Self::Document => "document",
            Self::Message => "message",
            Self::Transaction => "transaction",
            Self::TransactionResponse => "transaction-response",
            Self::Batch => "batch",
            Self::BatchResponse => "batch-response",
            Self::History => "history",
            Self::Searchset => "searchset",
            Self::Collection => "collection",
        }
    }
}
