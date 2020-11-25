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

use std::mem::{swap, take};

use bytes::{BufMut, Bytes, BytesMut};

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

#[derive(Debug, PartialEq)]
enum State {
    ExpectRoot,
    Barrier,
    Object,
    ArrayAny,
    ArrayValues {
        name: String,
        buffer: BytesMut,
        has_extension: bool,
    },
    PendingField {
        name: String,
    },
    PendingObject,
    PendingArray,
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
            None | Some(State::Barrier) => return Err(Error::ExpectedEof),
            Some(state) => state,
        };

        match item {
            Item::Root { name } => match state {
                State::ExpectRoot => {
                    self.buffer.extend_from_slice(b"{\"resourceType\":\"");
                    encode_escape_str(&mut self.buffer, &name);
                    self.buffer.extend_from_slice(b"\"");

                    self.state.push(State::Object);
                }
                State::PendingField { name: field } => {
                    self.flush_pending();

                    self.buffer.extend_from_slice(b"\"");
                    self.buffer.extend_from_slice(field.as_bytes());
                    self.buffer.extend_from_slice(b"\":{\"resourceType\":\"");
                    encode_escape_str(&mut self.buffer, &name);
                    self.buffer.extend_from_slice(b"\"");

                    self.state.push(State::Object);
                }
                State::PendingArray => {
                    self.flush_pending();

                    self.buffer.extend_from_slice(b"[{\"resourceType\":\"");
                    encode_escape_str(&mut self.buffer, &name);
                    self.buffer.extend_from_slice(b"\"");

                    self.state.push(State::ArrayAny);
                    self.state.push(State::Object);
                }
                State::ArrayAny => {
                    self.buffer.extend_from_slice(b",{\"resourceType\":\"");
                    encode_escape_str(&mut self.buffer, &name);
                    self.buffer.extend_from_slice(b"\"");

                    self.state.push(State::ArrayAny);
                    self.state.push(State::Object);
                }
                _ => return Err(Error::UnexpectedItem(Item::Root { name })),
            },
            Item::Element => match state {
                State::PendingField { name } => {
                    self.state.push(State::PendingField { name });
                    self.state.push(State::PendingObject);
                }
                State::PendingArray => {
                    self.state.push(State::PendingArray);
                    self.state.push(State::PendingObject);
                }
                State::ArrayAny => {
                    self.state.push(State::ArrayAny);
                    self.state.push(State::PendingObject);
                }
                _ => return Err(Error::UnexpectedItem(Item::Element)),
            },
            Item::Array => match state {
                State::PendingField { name } => {
                    self.state.push(State::PendingField { name });
                    self.state.push(State::PendingArray);
                }
                _ => return Err(Error::UnexpectedItem(Item::Array)),
            },
            Item::End => match state {
                State::PendingArray => {
                    self.drop_pending_field();
                }
                State::PendingObject => {
                    self.drop_pending_field();
                }
                State::Object => {
                    self.buffer.extend_from_slice(b"}");
                }
                State::ArrayAny => {
                    self.buffer.extend_from_slice(b"]");
                }
                State::ArrayValues {
                    name,
                    buffer,
                    has_extension,
                } => {
                    self.buffer.extend_from_slice(b"]");

                    if has_extension {
                        self.buffer.extend_from_slice(b",\"_");
                        encode_escape_str(&mut self.buffer, &name);
                        self.buffer.extend_from_slice(b"\":[");
                        self.buffer.extend_from_slice(&buffer);
                        self.buffer.extend_from_slice(b"]");
                    }
                }
                _ => return Err(Error::UnexpectedItem(Item::End)),
            },
            Item::Attrib { name } => match state {
                State::Object => {
                    self.state.push(State::Object);
                    self.state.push(State::PendingField { name });
                }
                State::PendingObject => {
                    self.state.push(State::PendingObject);
                    self.state.push(State::PendingField { name });
                }
                _ => return Err(Error::UnexpectedItem(Item::Attrib { name })),
            },
            Item::Field { name } => match state {
                State::Object => {
                    self.state.push(State::Object);
                    self.state.push(State::PendingField { name });
                }
                State::PendingObject => {
                    self.state.push(State::PendingObject);
                    self.state.push(State::PendingField { name });
                }
                _ => return Err(Error::UnexpectedItem(Item::Field { name })),
            },
            Item::Value { value, extension } => match state {
                State::PendingField { name } => {
                    self.flush_pending();

                    self.buffer.extend_from_slice(b"\"");
                    self.buffer.extend_from_slice(name.as_bytes());
                    self.buffer.extend_from_slice(b"\":");

                    let mut buffer = BytesMut::new();
                    if self.encode_value(&mut buffer, value, extension)? {
                        self.buffer.extend_from_slice(b",\"_");
                        encode_escape_str(&mut self.buffer, &name);
                        self.buffer.extend_from_slice(b"\":");
                        self.buffer.extend_from_slice(&buffer);
                    }
                }
                State::PendingArray => {
                    let mut buffer = BytesMut::new();

                    let name = self.flush_pending();

                    self.buffer.extend_from_slice(b"[");

                    if let Some(name) = name {
                        let has_extension = self.encode_value(&mut buffer, value, extension)?;

                        self.state.push(State::ArrayValues {
                            name,
                            buffer,
                            has_extension,
                        });
                    } else {
                        self.state.push(State::ArrayAny);
                    }
                }
                State::ArrayValues {
                    name,
                    mut buffer,
                    mut has_extension,
                } => {
                    self.buffer.extend_from_slice(b",");

                    has_extension |= self.encode_value(&mut buffer, value, extension)?;

                    self.state.push(State::ArrayValues {
                        name,
                        buffer,
                        has_extension,
                    });
                }
                State::ArrayAny => {
                    self.buffer.extend_from_slice(b",");

                    encode_value(&mut self.buffer, value);

                    self.state.push(State::ArrayAny);
                }
                _ => return Err(Error::UnexpectedItem(Item::Value { value, extension })),
            },
        }

        Ok(true)
    }

    fn drop_pending_field(&mut self) {
        match self.state.pop() {
            None => (),
            Some(State::PendingField { .. }) => (),
            Some(state) => self.state.push(state),
        }
    }

    fn flush_pending(&mut self) -> Option<String> {
        match self.state.pop() {
            Some(State::PendingArray) => {
                let ret = self.flush_pending();

                self.buffer.extend_from_slice(b"[");

                self.state.push(State::ArrayAny);

                ret
            }
            Some(State::PendingObject) => {
                let ret = self.flush_pending();

                self.buffer.extend_from_slice(b"{");

                self.state.push(State::Object);

                ret
            }
            Some(State::PendingField { name }) => {
                self.flush_pending();

                self.buffer.extend_from_slice(b"\"");
                self.buffer.extend_from_slice(name.as_bytes());
                self.buffer.extend_from_slice(b"\":");

                Some(name)
            }
            Some(State::Object) => {
                self.buffer.extend_from_slice(b",");

                self.state.push(State::Object);

                None
            }
            Some(State::ArrayAny) => {
                self.buffer.extend_from_slice(b",");

                self.state.push(State::ArrayAny);

                None
            }
            Some(State::ArrayValues {
                name,
                buffer,
                has_extension,
            }) => {
                self.buffer.extend_from_slice(b",");

                self.state.push(State::ArrayValues {
                    name,
                    buffer,
                    has_extension,
                });

                None
            }
            Some(state) => {
                self.state.push(state);

                None
            }
            None => None,
        }
    }

    fn encode_value(
        &mut self,
        buffer: &mut BytesMut,
        value: Value,
        extension: Vec<Item>,
    ) -> Result<bool, Error> {
        encode_value(&mut self.buffer, value);

        if !buffer.is_empty() {
            buffer.extend_from_slice(b",");
        }

        if extension.is_empty() {
            buffer.extend_from_slice(b"null");

            return Ok(false);
        }

        swap(&mut self.buffer, buffer);

        self.state.push(State::Barrier);
        self.state.push(State::PendingObject);

        for item in extension {
            self.write(Some(item))?;
        }
        self.write(Some(Item::End))?;

        match self.state.pop() {
            Some(State::Barrier) => (),
            Some(_) | None => return Err(Error::InvalidExtension),
        }

        swap(&mut self.buffer, buffer);

        Ok(true)
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
        Some(Format::Json)
    }
}

fn encode_value(buf: &mut BytesMut, value: Value) {
    match value {
        Value::Boolean(true) => buf.extend_from_slice(b"true"),
        Value::Boolean(false) => buf.extend_from_slice(b"false"),
        Value::Signed(i) => buf.extend_from_slice(i.to_string().as_bytes()),
        Value::Unsigned(u) => buf.extend_from_slice(u.to_string().as_bytes()),
        Value::Float(f) => buf.extend_from_slice(f.to_string().as_bytes()),
        Value::String(s) => {
            buf.put_u8(b'"');
            encode_escape_str(buf, &s);
            buf.put_u8(b'"');
        }
        Value::Str(s) => {
            buf.put_u8(b'"');
            encode_escape_str(buf, s);
            buf.put_u8(b'"');
        }
    }
}

fn encode_escape_str(bytes: &mut BytesMut, s: &str) {
    for c in s.chars() {
        match c {
            '"' => {
                bytes.put_u8(b'\\');
                bytes.put_u8(b'"');
            }
            '\\' => {
                bytes.put_u8(b'\\');
                bytes.put_u8(b'\\');
            }
            '\x08' => {
                bytes.put_u8(b'\\');
                bytes.put_u8(b'b');
            }
            '\x0c' => {
                bytes.put_u8(b'\\');
                bytes.put_u8(b'f');
            }
            '\n' => {
                bytes.put_u8(b'\\');
                bytes.put_u8(b'n');
            }
            '\r' => {
                bytes.put_u8(b'\\');
                bytes.put_u8(b'r');
            }
            '\t' => {
                bytes.put_u8(b'\\');
                bytes.put_u8(b't');
            }
            c @ 'ä' | c @ 'ö' | c @ 'ü' => {
                let mut buffer: [u8; 4] = [0; 4];
                let s = c.encode_utf8(&mut buffer);
                bytes.extend_from_slice(s.as_bytes());
            }
            c if c.is_ascii() => bytes.put_u8(c as u8),
            c => {
                let mut buffer: [u16; 4] = [0; 4];
                for c in c.encode_utf16(&mut buffer) {
                    bytes.extend_from_slice(format!("\\u{:04X}", c).as_bytes());
                }
            }
        }
    }
}
