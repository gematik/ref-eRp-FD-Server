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

use super::primitives::{Id, Instant};

#[derive(Clone, PartialEq, Debug)]
pub struct Bundle<T> {
    pub id: Option<Id>,
    pub meta: Option<Meta>,
    pub identifier: Option<Identifier>,
    pub timestamp: Option<Instant>,
    pub total: Option<usize>,
    pub link: Vec<(Relation, String)>,
    pub type_: Type,
    pub entries: Vec<Entry<T>>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Meta {
    pub last_updated: Option<Instant>,
    pub profile: Vec<String>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Identifier {
    pub system: Option<String>,
    pub value: Option<String>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Entry<T> {
    pub url: Option<String>,
    pub resource: T,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Type {
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

#[derive(Clone, PartialEq, Debug)]
pub enum Relation {
    Self_,
    First,
    Previous,
    Next,
    Last,
}

impl<T> Bundle<T> {
    pub fn new(type_: Type) -> Self {
        Self {
            id: None,
            meta: None,
            identifier: None,
            timestamp: None,
            total: None,
            link: Vec::new(),
            type_,
            entries: Vec::new(),
        }
    }
}

impl<T> Entry<T> {
    pub fn new(resource: T) -> Self {
        Self {
            url: None,
            resource,
        }
    }
}
