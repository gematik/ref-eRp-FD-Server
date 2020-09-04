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

pub trait ResourceType {
    fn resource_type() -> &'static str;
}

pub trait SerializeRoot<'a>: Sized {
    type Inner: ResourceType + 'a;

    fn from_inner(inner: &'a Self::Inner) -> Self;
}

pub trait DeserializeRoot: Sized {
    type Inner: ResourceType;

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
        struct Helper<'x, X> {
            resource_type: &'static str,

            #[serde(flatten)]
            value: &'x X,
        }

        Helper {
            resource_type: X::Inner::resource_type(),
            value: &self.0,
        }
        .serialize(serializer)
    }
}

impl<X: DeserializeRoot> Root<X> {
    #[allow(dead_code)]
    pub fn into_inner(self) -> X::Inner {
        self.0.into_inner()
    }
}

impl<'de, X: Deserialize<'de> + DeserializeRoot> Deserialize<'de> for Root<X> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Helper<X> {
            resource_type: String,

            #[serde(flatten)]
            value: X,
        }

        let helper = Helper::deserialize(deserializer)?;

        if helper.resource_type != X::Inner::resource_type() {
            return Err(D::Error::custom(format!(
                "Invalid resource type (actual={}, expected={})!",
                helper.resource_type,
                X::Inner::resource_type()
            )));
        }

        Ok(Root(helper.value))
    }
}
