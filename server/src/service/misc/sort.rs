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

use std::cmp::Ordering;
use std::str::FromStr;

use serde::de::{Deserialize, Deserializer, Error};

pub struct Sort<T> {
    parameters: Vec<Parameter<T>>,
}

pub enum Parameter<T> {
    Ascending(T),
    Descending(T),
}

impl<T> Sort<T> {
    pub fn cmp<F>(&self, f: F) -> Ordering
    where
        F: Fn(&T) -> Ordering,
    {
        for p in &self.parameters {
            let o = match p {
                Parameter::Ascending(p) => f(p),
                Parameter::Descending(p) => f(p).reverse(),
            };

            if o != Ordering::Equal {
                return o;
            }
        }

        Ordering::Equal
    }
}

impl<'de, T> Deserialize<'de> for Sort<T>
where
    T: FromStr,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let parameters = s
            .split(',')
            .map(|p| {
                let s = if p.starts_with('-') { &p[1..] } else { p };
                let s = s.parse().map_err(|_| {
                    D::Error::custom(format!("Unable to deserialize sort parameter: {}", s))
                })?;

                if p.starts_with('-') {
                    Ok(Parameter::Descending(s))
                } else {
                    Ok(Parameter::Ascending(s))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        if parameters.is_empty() {
            return Err(D::Error::custom("Empty sort parameters!"));
        }

        Ok(Sort { parameters })
    }
}
