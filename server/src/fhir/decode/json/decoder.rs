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

use std::fmt::{Debug, Display};
use std::vec::IntoIter;

use bytes::Bytes;
use futures::{
    future::{FutureExt, LocalBoxFuture},
    stream::Stream,
};

use crate::fhir::{Format, WithFormat};

use super::{
    super::{
        byte_stream::StreamError,
        item_stream::{Decoder, DecoderFuture, Item, ItemStream},
    },
    error::Error,
};

use super::reader::{Element, Reader};

pub struct Json<'a, S, E>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin + 'a,
    E: Display + Debug,
{
    reader: Reader<'a, S, E>,
    state: Vec<State>,
}

#[derive(Debug)]
enum State {
    /// A Root Object is expected
    Root,

    /// Current element is a resource object
    Resource,

    /// Current element is an object
    Object { is_barrier: bool },

    /// Current element is an array
    Array { name: String },

    /// Current element is an array of values
    /// This state is used to collect all values in the array before reading it's extensions
    ValueArray { name: String, values: Vec<String> },

    /// Current element is a field inside an object
    Field { name: String },

    /// Last element was a key-value-pair read from an object
    /// This state is used to gain extensions for key-value-pairs
    Value { name: String, value: String },

    /// Last element was an array of values read from an object
    /// This state is used to gain extensions for the value array
    Values { name: String, values: Vec<String> },

    /// List of parsed items to return
    Items { items: IntoIter<Item> },
}

type InnerFuture<'a, S, T, E> = LocalBoxFuture<'a, Result<(S, T), Error<E>>>;

impl<'a, S, E> Json<'a, S, E>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin + 'a,
    E: Display + Debug + 'static,
{
    pub fn new(stream: S) -> ItemStream<'a, Self> {
        ItemStream::Idle(Self {
            reader: Reader::new(stream),
            state: vec![State::Root],
        })
    }

    fn next_item(mut self) -> InnerFuture<'a, Self, Option<Item>, E> {
        async move {
            loop {
                let element = match self.reader.next().await {
                    Ok(Some(element)) => element,
                    Ok(None) if self.state.is_empty() => return Ok((self, None)),
                    Ok(None) => return Err(StreamError::UnexpectedEof.into()),
                    Err(err) => return Err(err),
                };

                let state = match self.state.pop() {
                    Some(state) => state,
                    None => return Err(Error::ExpectedEoF),
                };

                let item = match state {
                    State::Root => match element {
                        Element::BeginObject => {
                            match self.reader.next().await? {
                                Some(Element::Field(n)) if n == "resourceType" => (),
                                None => return Err(StreamError::UnexpectedEof.into()),
                                _ => return Err(Error::ExpectedResourceType),
                            }

                            let name = match self.reader.next().await? {
                                Some(Element::Value(Some(name))) => name,
                                None => return Err(StreamError::UnexpectedEof.into()),
                                _ => return Err(Error::ExpectedResourceType),
                            };

                            self.state.push(State::Object { is_barrier: false });

                            Item::BeginElement { name }
                        }
                        Element::EndObject
                        | Element::BeginArray
                        | Element::EndArray
                        | Element::Field(_)
                        | Element::Value(_) => return Err(Error::UnexpectedElement),
                    },
                    State::Resource => {
                        self.reader.put_back(element);

                        Item::EndElement
                    }
                    State::Object { is_barrier } => match element {
                        Element::EndObject => {
                            if is_barrier {
                                return Ok((self, None));
                            } else {
                                Item::EndElement
                            }
                        }
                        Element::Field(name) => {
                            self.state.push(State::Object { is_barrier });
                            self.state.push(State::Field { name });

                            continue;
                        }
                        Element::BeginObject
                        | Element::BeginArray
                        | Element::EndArray
                        | Element::Value(_) => return Err(Error::UnexpectedElement),
                    },
                    State::Array { name } => match element {
                        Element::BeginObject => {
                            self.state.push(State::Array { name: name.clone() });

                            self.check_for_resource().await?;

                            Item::BeginElement { name }
                        }
                        Element::EndArray => continue,
                        Element::Value(Some(value)) => {
                            self.state.push(State::ValueArray {
                                name,
                                values: vec![value],
                            });

                            continue;
                        }
                        Element::EndObject
                        | Element::BeginArray
                        | Element::Field(_)
                        | Element::Value(None) => return Err(Error::UnexpectedElement),
                    },
                    State::ValueArray { name, mut values } => match element {
                        Element::Value(Some(value)) => {
                            values.push(value);

                            self.state.push(State::ValueArray { name, values });

                            continue;
                        }
                        Element::EndArray => {
                            self.state.push(State::Values { name, values });

                            continue;
                        }
                        Element::BeginObject
                        | Element::EndObject
                        | Element::BeginArray
                        | Element::Field(_)
                        | Element::Value(None) => return Err(Error::UnexpectedElement),
                    },
                    State::Field { name } => match element {
                        Element::BeginObject => {
                            self.check_for_resource().await?;

                            Item::BeginElement { name }
                        }
                        Element::BeginArray => {
                            self.state.push(State::Array { name });

                            continue;
                        }
                        Element::Value(Some(value)) => {
                            self.state.push(State::Value { name, value });

                            continue;
                        }
                        Element::EndObject
                        | Element::EndArray
                        | Element::Field(_)
                        | Element::Value(None) => return Err(Error::UnexpectedElement),
                    },
                    State::Value { name, value } => match element {
                        Element::Field(field) if field.starts_with('_') && name == field[1..] => {
                            match self.reader.next().await? {
                                Some(Element::BeginObject) => (),
                                None => return Err(StreamError::UnexpectedEof.into()),
                                _ => return Err(Error::UnexpectedElement),
                            }

                            self.state.push(State::Object { is_barrier: true });

                            let (next, extension) = self.extract().await?;

                            return Ok((
                                next,
                                Some(Item::Field {
                                    name,
                                    value,
                                    extension,
                                }),
                            ));
                        }
                        element => {
                            self.reader.put_back(element);

                            Item::Field {
                                name,
                                value,
                                extension: Vec::new(),
                            }
                        }
                    },
                    State::Values { name, values } => match element {
                        Element::Field(field) if field.starts_with('_') && name == field[1..] => {
                            match self.reader.next().await? {
                                Some(Element::BeginArray) => (),
                                None => return Err(StreamError::UnexpectedEof.into()),
                                _ => return Err(Error::UnexpectedElement),
                            }

                            let mut items = Vec::new();
                            for value in values {
                                let name = name.clone();
                                let extension = match self.reader.next().await? {
                                    Some(Element::Value(None)) => Vec::new(),
                                    Some(Element::BeginObject) => {
                                        self.state.push(State::Object { is_barrier: true });

                                        let (next, extension) = self.extract().await?;

                                        self = next;

                                        extension
                                    }
                                    None => return Err(StreamError::UnexpectedEof.into()),
                                    _ => return Err(Error::UnexpectedElement),
                                };

                                items.push(Item::Field {
                                    name,
                                    value,
                                    extension,
                                });
                            }
                            let items = items.into_iter();

                            match self.reader.next().await? {
                                Some(Element::EndArray) => (),
                                None => return Err(StreamError::UnexpectedEof.into()),
                                _ => return Err(Error::UnexpectedElement),
                            }

                            self.state.push(State::Items { items });

                            continue;
                        }
                        element => {
                            self.reader.put_back(element);

                            let items = values
                                .into_iter()
                                .map(|value| Item::Field {
                                    name: name.clone(),
                                    value,
                                    extension: Vec::new(),
                                })
                                .collect::<Vec<_>>()
                                .into_iter();

                            self.state.push(State::Items { items });

                            continue;
                        }
                    },
                    State::Items { mut items } => {
                        self.reader.put_back(element);

                        let item = match items.next() {
                            Some(item) => item,
                            None => continue,
                        };

                        self.state.push(State::Items { items });

                        item
                    }
                };

                return Ok((self, Some(item)));
            }
        }
        .boxed_local()
    }

    fn extract(mut self) -> InnerFuture<'a, Self, Vec<Item>, E> {
        async move {
            let mut vec = Vec::new();

            loop {
                let (next, item) = self.next_item().await?;

                if let Some(item) = item {
                    self = next;

                    vec.push(item);
                } else {
                    return Ok((next, vec));
                }
            }
        }
        .boxed_local()
    }

    async fn check_for_resource(&mut self) -> Result<(), Error<E>> {
        let is_resource = match self.reader.next().await? {
            Some(Element::Field(n)) if n == "resourceType" => true,
            Some(element) => {
                self.reader.put_back(element);

                false
            }
            None => return Err(StreamError::UnexpectedEof.into()),
        };

        if is_resource {
            let name = match self.reader.next().await? {
                Some(Element::Value(Some(res))) => res,
                Some(_) => return Err(Error::UnexpectedElement),
                None => return Err(StreamError::UnexpectedEof.into()),
            };

            let items = vec![Item::BeginElement { name }].into_iter();

            self.state.push(State::Resource);
            self.state.push(State::Object { is_barrier: false });
            self.state.push(State::Items { items });
        } else {
            self.state.push(State::Object { is_barrier: false });
        }

        Ok(())
    }
}

impl<'a, S, E> Decoder<'a> for Json<'a, S, E>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin + 'a,
    E: Display + Debug + 'static,
{
    type Error = Error<E>;

    fn next(self) -> DecoderFuture<'a, Self, Self::Error> {
        async move {
            let (next, item) = self.next_item().await?;

            if let Some(item) = item {
                Ok(Some((next, item)))
            } else {
                Ok(None)
            }
        }
        .boxed_local()
    }
}

impl<'a, S, E> WithFormat for Json<'a, S, E>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin + 'a,
    E: Display + Debug,
{
    fn format(&self) -> Option<Format> {
        Some(Format::Json)
    }
}
