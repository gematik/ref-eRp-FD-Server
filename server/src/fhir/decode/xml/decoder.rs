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

use super::{
    super::{
        byte_stream::StreamError,
        item_stream::{Decoder, DecoderFuture, Item, ItemStream},
    },
    error::Error,
    reader::{Element, Reader},
};

pub struct Xml<'a, S, E>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin + 'a,
    E: Display + Debug,
{
    reader: Reader<'a, S, E>,
    state: Vec<State>,
}

#[derive(Debug)]
enum State {
    Root,
    Element {
        is_barrier: bool,
    },
    Attribs {
        is_empty_tag: bool,
        attribs: IntoIter<(String, String)>,
    },
}

type InnerFuture<'a, S, T, E> = LocalBoxFuture<'a, Result<(S, T), Error<E>>>;

impl<'a, S, E> Xml<'a, S, E>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin + 'a,
    E: Display + Debug + 'static,
{
    pub fn new(stream: S) -> ItemStream<'a, Self> {
        ItemStream::Idle(Xml {
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

                let item = match self.state.pop() {
                    None => match element {
                        Element::Text { value } if value.chars().all(char::is_whitespace) => {
                            continue
                        }
                        _ => return Err(Error::ExpectedEof),
                    },
                    Some(State::Root) => match element {
                        Element::Begin { name, attribs } => {
                            match attribs.get("xmlns") {
                                Some("http://hl7.org/fhir") => (),
                                Some(_) => return Err(Error::InvalidXmlns),
                                None => return Err(Error::MissingXmlns),
                            }

                            self.state.push(State::Element { is_barrier: false });

                            Item::BeginElement { name }
                        }
                        Element::Text { value } if value.chars().all(char::is_whitespace) => {
                            self.state.push(State::Root);

                            continue;
                        }
                        element => return Err(Error::UnexpectedElement(element)),
                    },
                    Some(State::Element { is_barrier }) => match element {
                        Element::Begin { name, attribs } => {
                            self.state.push(State::Element { is_barrier });

                            let has_xmlns = match attribs.get("xmlns") {
                                None => false,
                                Some("http://hl7.org/fhir") if attribs.0.len() == 1 => true,
                                _ => return Err(Error::InvalidXmlns),
                            };

                            if attribs.0.len() == 1 && attribs.0[0].0 == "value" {
                                self.state.push(State::Element { is_barrier: true });

                                let value = attribs.0.into_iter().next().unwrap().1;

                                let mut depth = 0;
                                let mut extension = Vec::new();

                                loop {
                                    let (this, item) = self.next_item().await?;
                                    self = this;

                                    match item {
                                        Some(Item::BeginElement { name }) => {
                                            depth += 1;

                                            extension.push(Item::BeginElement { name });
                                        }
                                        Some(Item::EndElement) if depth == 0 => {
                                            return Err(Error::InvalidValueExtension)
                                        }
                                        Some(Item::EndElement) => {
                                            depth -= 1;

                                            extension.push(Item::EndElement);
                                        }
                                        Some(item) => extension.push(item),
                                        None if depth == 0 => break,
                                        None => return Err(StreamError::UnexpectedEof.into()),
                                    }
                                }

                                Item::Field {
                                    name,
                                    value,
                                    extension,
                                }
                            } else {
                                self.state.push(State::Element { is_barrier: false });

                                if !attribs.0.is_empty() && !has_xmlns {
                                    let attribs = attribs.0.into_iter();

                                    self.state.push(State::Attribs {
                                        attribs,
                                        is_empty_tag: false,
                                    });
                                }

                                Item::BeginElement { name }
                            }
                        }
                        Element::Empty { name, attribs } => {
                            self.state.push(State::Element { is_barrier });

                            if attribs.0.len() == 1 && attribs.0[0].0 == "value" {
                                Item::Field {
                                    name,
                                    value: attribs.0.into_iter().next().unwrap().1,
                                    extension: Vec::new(),
                                }
                            } else {
                                let attribs = attribs.0.into_iter();

                                self.state.push(State::Attribs {
                                    attribs,
                                    is_empty_tag: true,
                                });

                                Item::BeginElement { name }
                            }
                        }
                        Element::Text { value } if value.chars().all(char::is_whitespace) => {
                            self.state.push(State::Element { is_barrier });

                            continue;
                        }
                        Element::End => {
                            if is_barrier {
                                return Ok((self, None));
                            } else {
                                Item::EndElement
                            }
                        }
                        element => return Err(Error::UnexpectedElement(element)),
                    },
                    Some(State::Attribs {
                        is_empty_tag,
                        mut attribs,
                    }) => {
                        self.reader.put_back(element);

                        match attribs.next() {
                            Some((name, value)) => {
                                self.state.push(State::Attribs {
                                    is_empty_tag,
                                    attribs,
                                });

                                Item::Field {
                                    name,
                                    value,
                                    extension: Vec::new(),
                                }
                            }
                            None if is_empty_tag => Item::EndElement,
                            None => continue,
                        }
                    }
                };

                return Ok((self, Some(item)));
            }
        }
        .boxed_local()
    }
}

impl<'a, S, E> Decoder<'a> for Xml<'a, S, E>
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
