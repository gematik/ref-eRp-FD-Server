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

use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

pub trait XmlnsType {
    fn xmlns() -> &'static str;
}

pub trait SerializeRoot<'a>: Sized {
    type Inner: XmlnsType + 'a;

    fn from_inner(inner: &'a Self::Inner) -> Self;
}

pub trait DeserializeRoot: Sized {
    type Inner: XmlnsType;

    fn into_inner(self) -> Self::Inner;
}

pub struct Root<X>(X);

impl<'a, X: Serialize + SerializeRoot<'a>> Root<X> {
    pub fn new(inner: &'a X::Inner) -> Self {
        Self(X::from_inner(inner))
    }
}

impl<'a, X: Serialize + SerializeRoot<'a>> Serialize for Root<X> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        #[serde(rename = "xml:placeholder")]
        struct Helper<'x, X> {
            #[serde(alias = "xmlns")]
            #[serde(rename = "attrib=xmlns")]
            xmlns: &'static str,

            #[serde(rename = "flatten-take-name")]
            value: &'x X,
        }

        Helper {
            xmlns: X::Inner::xmlns(),
            value: &self.0,
        }
        .serialize(serializer)
    }
}

impl<X: DeserializeRoot> Root<X> {
    pub fn into_inner(self) -> X::Inner {
        self.0.into_inner()
    }
}

impl<'de, X: Deserialize<'de> + DeserializeRoot> Deserialize<'de> for Root<X> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[serde(rename = "xml:placeholder")]
        struct Helper<X> {
            #[serde(alias = "xmlns")]
            #[serde(rename = "attrib=xmlns")]
            xmlns: String,

            #[serde(rename = "flatten-take-name")]
            value: X,
        }

        let helper = Helper::deserialize(deserializer)?;

        if helper.xmlns != X::Inner::xmlns() {
            return Err(D::Error::custom(format!(
                "Invalid xmlns (actual={}, expected={})!",
                helper.xmlns,
                X::Inner::xmlns()
            )));
        }

        Ok(Root(helper.value))
    }
}
