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

#[macro_export]
macro_rules! make_option_def {
    ( $type:ident, $def:tt, $opt:ident ) => {
        make_option_def!($type, $def, $opt, "Helper");
    };
    ( $type:ident, $def:tt, $opt:ident, $rename:tt ) => {
        pub struct $opt;

        impl $opt {
            pub fn serialize<S: Serializer>(
                value: &Option<$type>,
                serializer: S,
            ) -> Result<S::Ok, S::Error> {
                #[derive(Serialize)]
                #[serde(rename = $rename)]
                struct Helper<'a>(#[serde(with = $def)] &'a $type);

                value.as_ref().map(Helper).serialize(serializer)
            }

            pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<$type>, D::Error>
            where
                D: Deserializer<'de>,
            {
                #[derive(Deserialize)]
                #[serde(rename = $rename)]
                struct Helper(#[serde(with = $def)] $type);

                let helper = Option::deserialize(deserializer)?;

                Ok(helper.map(|Helper(value)| value))
            }
        }
    };
}

#[macro_export]
macro_rules! make_vec_def {
    ( $type:ident, $def:tt, $opt:ident ) => {
        make_vec_def!($type, $def, $opt, "Helper");
    };
    ( $type:ident, $def:tt, $opt:ident, $rename:tt ) => {
        pub struct $opt;

        impl $opt {
            pub fn serialize<S: Serializer>(
                value: &[$type],
                serializer: S,
            ) -> Result<S::Ok, S::Error> {
                #[derive(Serialize)]
                #[serde(rename = $rename)]
                struct Helper<'a>(#[serde(with = $def)] &'a $type);

                value
                    .iter()
                    .map(Helper)
                    .collect::<Vec<Helper>>()
                    .serialize(serializer)
            }

            pub fn deserialize<'de, D: Deserializer<'de>>(
                deserializer: D,
            ) -> Result<Vec<$type>, D::Error> {
                #[derive(Deserialize)]
                #[serde(rename = $rename)]
                struct Helper(#[serde(with = $def)] $type);

                let helper = Vec::deserialize(deserializer)?;

                Ok(helper.into_iter().map(|Helper(value)| value).collect())
            }
        }
    };
}
