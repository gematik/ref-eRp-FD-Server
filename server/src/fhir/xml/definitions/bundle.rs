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

use std::borrow::Cow;

use resources::{
    bundle::{Bundle, Entry, Identifier, Meta, Type},
    primitives::{Id, Instant},
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{
    super::super::constants::XMLNS_BUNDLE,
    misc::{IdentifierDef, MetaDef, Root, SerializeRoot, XmlnsType},
    primitives::{OptionIdDef, OptionInstantDef},
};

pub struct BundleDef;
pub type BundleRoot<'a, T> = Root<BundleCow<'a, T>>;

#[serde(rename = "Bundle")]
#[derive(Serialize)]
pub struct BundleCow<'a, T: Clone + Serialize>(#[serde(with = "BundleDef")] pub Cow<'a, Bundle<T>>);

#[serde(rename = "Bundle")]
#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize)]
struct BundleHelper<T: Serialize> {
    #[serde(with = "OptionIdDef")]
    pub id: Option<Id>,

    pub meta: Option<MetaDef>,

    pub identifier: Option<IdentifierDef>,

    #[serde(alias = "type")]
    #[serde(rename = "value-tag=type")]
    #[serde(with = "TypeDef")]
    pub type_: Type,

    #[serde(with = "OptionInstantDef")]
    pub timestamp: Option<Instant>,

    pub entry: Vec<EntryDef<T>>,
}

#[serde(rename = "Entry")]
#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize)]
struct EntryDef<T: Serialize> {
    #[serde(alias = "fullUrl")]
    #[serde(rename = "value-tag=fullUrl")]
    url: Option<String>,

    resource: T,
}

#[serde(remote = "Type")]
#[serde(rename_all = "kebab-case")]
#[derive(Serialize, Deserialize)]
pub enum TypeDef {
    Document,
    Message,
    Transaction,
    TransactionResponse,
    Batch,
    BatchResponse,
    History,
    Searchset,
    Collection,
}

impl<T> XmlnsType for Bundle<T> {
    fn xmlns() -> &'static str {
        XMLNS_BUNDLE
    }
}

impl<'a, T: Serialize + Clone> SerializeRoot<'a> for BundleCow<'a, T> {
    type Inner = Bundle<T>;

    fn from_inner(inner: &'a Self::Inner) -> Self {
        BundleCow(Cow::Borrowed(inner))
    }
}

impl BundleDef {
    pub fn serialize<T: Serialize + Clone, S: Serializer>(
        bundle: &Bundle<T>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let root: BundleHelper<T> = bundle.into();

        root.serialize(serializer)
    }
}

impl<'de, T: Serialize + Deserialize<'de> + Clone> Deserialize<'de> for BundleCow<'_, T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let root = BundleHelper::deserialize(deserializer)?;

        Ok(BundleCow(Cow::Owned(root.into())))
    }
}

impl<T: Serialize + Clone> Into<BundleHelper<T>> for &Bundle<T> {
    fn into(self) -> BundleHelper<T> {
        BundleHelper {
            id: self.id.clone(),
            meta: self.meta.as_ref().map(Into::into),
            identifier: self.identifier.as_ref().map(Into::into),
            timestamp: self.timestamp.clone(),
            type_: self.type_.clone(),
            entry: self.entries.iter().map(Into::into).collect(),
        }
    }
}

impl Into<IdentifierDef> for &Identifier {
    fn into(self) -> IdentifierDef {
        IdentifierDef {
            system: self.system.clone(),
            value: self.value.clone(),
            ..Default::default()
        }
    }
}

impl Into<MetaDef> for &Meta {
    fn into(self) -> MetaDef {
        MetaDef {
            last_updated: self.last_updated.clone(),
            profile: self
                .profile
                .iter()
                .map(Clone::clone)
                .map(Into::into)
                .collect(),
            ..Default::default()
        }
    }
}

impl<T: Serialize + Clone> Into<EntryDef<T>> for &Entry<T> {
    fn into(self) -> EntryDef<T> {
        EntryDef {
            url: self.url.as_ref().map(Clone::clone),
            resource: self.resource.clone(),
        }
    }
}

impl<'de, T: Serialize + Deserialize<'de> + Clone> Into<Bundle<T>> for BundleHelper<T> {
    fn into(self) -> Bundle<T> {
        Bundle {
            id: self.id,
            meta: self.meta.map(Into::into),
            identifier: self.identifier.map(Into::into),
            timestamp: self.timestamp,
            type_: self.type_,
            entries: self.entry.into_iter().map(Into::into).collect(),
        }
    }
}

impl Into<Meta> for MetaDef {
    fn into(self) -> Meta {
        Meta {
            last_updated: self.last_updated,
            profile: self.profile.into_iter().map(Into::into).collect(),
        }
    }
}

impl Into<Identifier> for IdentifierDef {
    fn into(self) -> Identifier {
        Identifier {
            system: self.system,
            value: self.value,
        }
    }
}

impl<T: Serialize> Into<Entry<T>> for EntryDef<T> {
    fn into(self) -> Entry<T> {
        Entry {
            url: self.url,
            resource: self.resource,
        }
    }
}
