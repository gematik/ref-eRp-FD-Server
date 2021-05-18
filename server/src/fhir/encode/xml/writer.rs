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

use std::mem::take;

use bytes::{Bytes, BytesMut};

use crate::fhir::Format;

use super::{
    super::{
        encode_stream::DataStorage,
        item::{Item, Value},
    },
    error::Error,
};

pub struct Writer {
    state: Vec<State>,
    buffer: BytesMut,
}

#[derive(Debug)]
enum State {
    ExpectRoot,
    Array { name: String },
    Tag { name: String, is_resource: bool },
    Field { name: String },
    Pending { name: String },
    Attrib { name: String },
}

impl Writer {
    pub fn freeze(&mut self) -> Bytes {
        take(&mut self.buffer).freeze()
    }

    pub fn write(&mut self, item: Option<Item>) -> Result<bool, Error> {
        let item = match item {
            None if self.state.is_empty() => return Ok(false),
            None => return Err(Error::UnexpectedEof),
            Some(item) => item,
        };

        let state = match self.state.pop() {
            None => return Err(Error::ExpectedEof),
            Some(state) => state,
        };

        match item {
            Item::Root { name } => match state {
                State::ExpectRoot => {
                    self.buffer.extend_from_slice(b"<");
                    self.buffer.extend_from_slice(name.as_bytes());
                    self.buffer
                        .extend_from_slice(b" xmlns=\"http://hl7.org/fhir\">");

                    self.state.push(State::Tag {
                        name,
                        is_resource: false,
                    });
                }
                State::Array { name: tag } => {
                    self.buffer.extend_from_slice(b"<");
                    self.buffer.extend_from_slice(tag.as_bytes());
                    self.buffer.extend_from_slice(b"><");
                    self.buffer.extend_from_slice(name.as_bytes());
                    self.buffer.extend_from_slice(b">");

                    self.state.push(State::Array { name: tag.clone() });
                    self.state.push(State::Tag {
                        name: tag,
                        is_resource: false,
                    });
                    self.state.push(State::Tag {
                        name,
                        is_resource: true,
                    });
                }
                State::Pending { name: tag } => {
                    self.buffer.extend_from_slice(b"><");
                    self.buffer.extend_from_slice(name.as_bytes());
                    self.buffer.extend_from_slice(b">");

                    self.state.push(State::Tag {
                        name: tag,
                        is_resource: false,
                    });
                    self.state.push(State::Tag {
                        name,
                        is_resource: true,
                    });
                }
                State::Field { name: tag } => {
                    self.buffer.extend_from_slice(b"<");
                    self.buffer.extend_from_slice(tag.as_bytes());
                    self.buffer.extend_from_slice(b"><");
                    self.buffer.extend_from_slice(name.as_bytes());
                    self.buffer.extend_from_slice(b">");

                    self.state.push(State::Tag {
                        name: tag,
                        is_resource: false,
                    });
                    self.state.push(State::Tag {
                        name,
                        is_resource: true,
                    });
                }
                _ => return Err(Error::UnexpectedItem(Item::Root { name })),
            },
            Item::Element => match state {
                State::Field { name } => {
                    self.buffer.extend_from_slice(b"<");
                    self.buffer.extend_from_slice(name.as_bytes());

                    self.state.push(State::Pending { name });
                }
                State::Array { name } => {
                    self.buffer.extend_from_slice(b"<");
                    self.buffer.extend_from_slice(name.as_bytes());

                    self.state.push(State::Array { name: name.clone() });
                    self.state.push(State::Pending { name });
                }
                _ => return Err(Error::UnexpectedItem(Item::Element)),
            },
            Item::Array => match state {
                State::Field { name } => {
                    self.state.push(State::Array { name });
                }
                _ => return Err(Error::UnexpectedItem(Item::Array)),
            },
            Item::End => match state {
                State::Tag { name, is_resource } => {
                    self.buffer.extend_from_slice(b"</");
                    self.buffer.extend_from_slice(name.as_bytes());
                    self.buffer.extend_from_slice(b">");

                    if is_resource {
                        self.write(Some(Item::End))?;
                    }
                }
                State::Array { .. } => (),
                _ => return Err(Error::UnexpectedItem(Item::End)),
            },
            Item::Attrib { name } => match state {
                State::Pending { name: tag } => {
                    self.state.push(State::Pending { name: tag });
                    self.state.push(State::Attrib { name });
                }
                _ => return Err(Error::UnexpectedItem(Item::Attrib { name })),
            },
            Item::Field { name } => match state {
                State::Tag {
                    name: tag,
                    is_resource,
                } => {
                    self.state.push(State::Tag {
                        name: tag,
                        is_resource,
                    });
                    self.state.push(State::Field { name });
                }
                State::Pending { name: tag } => {
                    self.buffer.extend_from_slice(b">");

                    self.state.push(State::Tag {
                        name: tag,
                        is_resource: false,
                    });
                    self.state.push(State::Field { name });
                }
                _ => return Err(Error::UnexpectedItem(Item::Field { name })),
            },
            Item::Value { value, extension } => match state {
                State::Field { name } => {
                    self.encode_value(name, value, extension)?;
                }
                State::Attrib { name } => {
                    let value = encode_value(value);

                    self.buffer.extend_from_slice(b" ");
                    self.buffer.extend_from_slice(name.as_bytes());
                    self.buffer.extend_from_slice(b"=\"");
                    self.buffer.extend_from_slice(value.as_bytes());
                    self.buffer.extend_from_slice(b"\"");
                }
                State::Array { name } => {
                    self.encode_value(name.clone(), value, extension)?;

                    self.state.push(State::Array { name });
                }
                State::Pending { name: tag } => {
                    self.buffer.extend_from_slice(b">");

                    self.state.push(State::Tag {
                        name: tag,
                        is_resource: false,
                    });

                    let value = encode_value(value);
                    self.buffer.extend_from_slice(value.as_bytes());
                }
                _ => return Err(Error::UnexpectedItem(Item::Value { value, extension })),
            },
        }

        Ok(true)
    }

    fn encode_value(
        &mut self,
        name: String,
        value: Value,
        extension: Vec<Item>,
    ) -> Result<(), Error> {
        let value = encode_value(value);

        self.buffer.extend_from_slice(b"<");
        self.buffer.extend_from_slice(name.as_bytes());
        self.buffer.extend_from_slice(b" value=\"");
        self.buffer.extend_from_slice(value.as_bytes());

        if extension.is_empty() {
            self.buffer.extend_from_slice(b"\"/>");
        } else {
            self.buffer.extend_from_slice(b"\">");

            self.state.push(State::Tag {
                name,
                is_resource: false,
            });

            for item in extension {
                self.write(Some(item))?;
            }

            let name = match self.state.pop() {
                Some(State::Tag { name, .. }) => name,
                Some(_) | None => return Err(Error::InvalidExtension),
            };

            self.buffer.extend_from_slice(b"</");
            self.buffer.extend_from_slice(name.as_bytes());
            self.buffer.extend_from_slice(b">");
        }

        Ok(())
    }
}

impl Default for Writer {
    fn default() -> Self {
        Self {
            state: vec![State::ExpectRoot],
            buffer: BytesMut::new(),
        }
    }
}

impl DataStorage for &mut Writer {
    type Error = Error;

    fn put_item(&mut self, item: Item) -> Result<(), Self::Error> {
        self.write(Some(item))?;

        Ok(())
    }

    fn format(&self) -> Option<Format> {
        Some(Format::Xml)
    }
}

fn encode_value(value: Value) -> String {
    match value {
        Value::Boolean(v) => v.to_string(),
        Value::Signed(v) => v.to_string(),
        Value::Unsigned(v) => v.to_string(),
        Value::Float(v) => v.to_string(),
        Value::String(v) => escape_str(&v),
        Value::Str(v) => v.into(),
    }
}

fn escape_str(s: &str) -> String {
    let mut ret = String::new();

    for c in s.chars() {
        match c {
            '<' => ret += "&lt;",
            '>' => ret += "&gt;",
            '"' => ret += "&quot;",
            '\'' => ret += "&apos;",
            '&' => ret += "&amp;",
            '\n' => ret += "&#xA;",
            '\r' => ret += "&#xD;",
            c => ret.push(c),
        }
    }

    ret
}
