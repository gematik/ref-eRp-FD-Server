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

use super::{Comperator, Parameter};

impl<T> Parameter for Option<T>
where
    T: Parameter,
{
    type Storage = Option<T::Storage>;

    fn parse(s: &str) -> Result<Self::Storage, String> {
        match s {
            "null" | "NULL" => Ok(None),
            s => Ok(Some(T::parse(s)?)),
        }
    }

    fn compare(&self, comperator: Comperator, param: &Self::Storage) -> bool {
        match comperator {
            Comperator::Equal => match (self, param) {
                (Some(this), Some(param)) => this.compare(Comperator::Equal, param),
                (None, None) => true,
                (_, _) => false,
            },
            Comperator::NotEqual => match (self, param) {
                (Some(this), Some(param)) => this.compare(Comperator::NotEqual, param),
                (Some(_), None) => true,
                (None, Some(_)) => true,
                (None, None) => false,
            },
            Comperator::GreaterThan => match (self, param) {
                (Some(this), Some(param)) => this.compare(Comperator::GreaterThan, param),
                (_, _) => false,
            },
            Comperator::LessThan => match (self, param) {
                (Some(this), Some(param)) => this.compare(Comperator::LessThan, param),
                (_, _) => false,
            },
            Comperator::GreaterEqual => match (self, param) {
                (Some(this), Some(param)) => this.compare(Comperator::GreaterEqual, param),
                (_, _) => false,
            },
            Comperator::LessEqual => match (self, param) {
                (Some(this), Some(param)) => this.compare(Comperator::LessEqual, param),
                (_, _) => false,
            },
            Comperator::StartsAfter => match (self, param) {
                (Some(this), Some(param)) => this.compare(Comperator::StartsAfter, param),
                (_, _) => false,
            },
            Comperator::EndsBefore => match (self, param) {
                (Some(this), Some(param)) => this.compare(Comperator::EndsBefore, param),
                (_, _) => false,
            },
            Comperator::Approximately => match (self, param) {
                (Some(this), Some(param)) => this.compare(Comperator::Approximately, param),
                (_, _) => false,
            },
        }
    }
}
