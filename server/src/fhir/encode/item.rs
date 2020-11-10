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

use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::stream::Stream;

use super::encode_stream::DataStorage;

#[derive(Debug, Default)]
pub struct ItemStream(VecDeque<Item>);

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum Item {
    Root { name: String },
    Element,
    Array,
    End,
    Attrib { name: String },
    Field { name: String },
    Value { value: Value, extension: Vec<Item> },
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Boolean(bool),
    Signed(isize),
    Unsigned(usize),
    Float(f64),
    String(String),
    Str(&'static str),
}

impl Stream for ItemStream
where
    Self: Unpin,
{
    type Item = Item;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(self.0.pop_front())
    }
}

impl DataStorage for &mut ItemStream {
    type Error = String;

    fn put_item(&mut self, item: Item) -> Result<(), Self::Error> {
        self.0.push_back(item);

        Ok(())
    }
}

impl From<&'static str> for Value {
    fn from(v: &str) -> Value {
        Self::String(v.into())
    }
}

impl From<String> for Value {
    fn from(v: String) -> Value {
        Self::String(v)
    }
}

impl From<&String> for Value {
    fn from(v: &String) -> Value {
        Self::String(v.clone())
    }
}

impl From<isize> for Value {
    fn from(v: isize) -> Value {
        Self::Signed(v)
    }
}

impl From<&isize> for Value {
    fn from(v: &isize) -> Value {
        Self::Signed(*v)
    }
}

impl From<usize> for Value {
    fn from(v: usize) -> Value {
        Self::Unsigned(v)
    }
}

impl From<&usize> for Value {
    fn from(v: &usize) -> Value {
        Self::Unsigned(*v)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Value {
        Self::Float(v)
    }
}

impl From<&f64> for Value {
    fn from(v: &f64) -> Value {
        Self::Float(*v)
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Value {
        Self::Boolean(v)
    }
}

impl From<&bool> for Value {
    fn from(v: &bool) -> Value {
        Self::Boolean(*v)
    }
}
