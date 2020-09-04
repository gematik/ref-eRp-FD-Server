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

use std::io::{Error, Write};

use serde_json::ser::{CharEscape, Formatter as JsonFormatter};

pub struct Formatter<F: JsonFormatter> {
    inner: F,
    is_first: bool,
    pending_entries: Vec<Entry>,
}

#[derive(Debug)]
enum Entry {
    Dropped {
        is_first: bool,
    },
    Pending {
        buffer: Vec<u8>,
        is_first: bool,
        child_count: usize,
    },
}

impl<F: JsonFormatter> Formatter<F> {
    pub fn new(inner: F) -> Self {
        Self {
            inner,
            is_first: false,
            pending_entries: Vec::new(),
        }
    }

    fn write_data<W: Write + ?Sized>(&mut self, writer: &mut W, data: &[u8]) -> Result<(), Error> {
        match self.pending_entries.last_mut() {
            Some(Entry::Pending {
                buffer,
                child_count,
                ..
            }) => {
                buffer.write_all(data)?;
                *child_count += 1;
            }
            Some(Entry::Dropped { .. }) => (),
            None => {
                writer.write_all(data)?;
            }
        }

        Ok(())
    }

    fn drop_entry(&mut self) {
        if let Some(pending_entry) = self.pending_entries.last_mut() {
            let is_first = if let Entry::Pending { is_first, .. } = pending_entry {
                *is_first
            } else {
                false
            };

            *pending_entry = Entry::Dropped { is_first };
        }
    }
}

macro_rules! forward {
    ( $func:ident => ( $($name:ident: $type:ty),*) ) => {
        fn $func<W: Write + ?Sized>(
            &mut self,
            writer: &mut W,
            $($name: $type),*
        ) -> Result<(), Error>
        {
            match self.pending_entries.last_mut() {
                Some(Entry::Pending { buffer, .. }) => self.inner.$func(buffer, $($name),*),
                Some(Entry::Dropped { .. }) => Ok(()),
                None => self.inner.$func(writer, $($name),*)
            }
        }
    };
}

impl<F: JsonFormatter> JsonFormatter for Formatter<F> {
    forward!(write_bool => (value: bool));

    forward!(write_i8 => (value: i8));
    forward!(write_i16 => (value: i16));
    forward!(write_i32 => (value: i32));
    forward!(write_i64 => (value: i64));

    forward!(write_u8 => (value: u8));
    forward!(write_u16 => (value: u16));
    forward!(write_u32 => (value: u32));
    forward!(write_u64 => (value: u64));

    forward!(write_f32 => (value: f32));
    forward!(write_f64 => (value: f64));

    forward!(write_number_str => (value: &str));
    forward!(begin_string => ());
    forward!(end_string => ());
    forward!(write_string_fragment => (fragment: &str));
    forward!(write_char_escape => (char_escape: CharEscape));

    forward!(end_object_key => ());
    forward!(begin_object_value => ());

    forward!(write_raw_fragment => (fragment: &str));

    fn write_null<W: Write + ?Sized>(&mut self, _writer: &mut W) -> Result<(), Error> {
        self.drop_entry();

        Ok(())
    }

    fn begin_array<W: Write + ?Sized>(&mut self, _writer: &mut W) -> Result<(), Error> {
        let mut buffer = Vec::new();

        self.inner.begin_array(&mut buffer)?;

        self.pending_entries.push(Entry::Pending {
            buffer,
            is_first: false,
            child_count: 0,
        });

        Ok(())
    }

    fn end_array<W: Write + ?Sized>(&mut self, writer: &mut W) -> Result<(), Error> {
        let data = match self.pending_entries.pop() {
            Some(Entry::Pending {
                mut buffer,
                child_count,
                ..
            }) => {
                if child_count == 0 {
                    self.drop_entry();

                    return Ok(());
                }

                self.is_first = false;
                self.inner.end_array(&mut buffer)?;

                buffer
            }
            Some(Entry::Dropped { is_first }) => {
                self.is_first = is_first;

                return Ok(());
            }
            None => return Ok(()),
        };

        self.write_data(writer, &data)?;

        Ok(())
    }

    fn begin_array_value<W: Write + ?Sized>(
        &mut self,
        _writer: &mut W,
        first: bool,
    ) -> Result<(), Error> {
        let mut buffer = Vec::new();

        let is_first = first || self.is_first;

        self.inner.begin_array_value(&mut buffer, is_first)?;

        self.pending_entries.push(Entry::Pending {
            buffer,
            is_first,
            child_count: 0,
        });

        Ok(())
    }

    fn end_array_value<W: Write + ?Sized>(&mut self, writer: &mut W) -> Result<(), Error> {
        let data = match self.pending_entries.pop() {
            Some(Entry::Pending { mut buffer, .. }) => {
                self.is_first = false;
                self.inner.end_array_value(&mut buffer)?;

                buffer
            }
            Some(Entry::Dropped { is_first }) => {
                self.is_first = is_first;

                return Ok(());
            }
            None => return Ok(()),
        };

        self.write_data(writer, &data)?;

        Ok(())
    }

    fn begin_object<W: Write + ?Sized>(&mut self, _writer: &mut W) -> Result<(), Error> {
        let mut buffer = Vec::new();

        self.inner.begin_object(&mut buffer)?;

        self.pending_entries.push(Entry::Pending {
            buffer,
            is_first: false,
            child_count: 0,
        });

        Ok(())
    }

    fn end_object<W: Write + ?Sized>(&mut self, writer: &mut W) -> Result<(), Error> {
        let data = match self.pending_entries.pop() {
            Some(Entry::Pending {
                mut buffer,
                child_count,
                ..
            }) => {
                if child_count == 0 {
                    self.drop_entry();

                    return Ok(());
                }

                self.is_first = false;
                self.inner.end_object(&mut buffer)?;

                buffer
            }
            Some(Entry::Dropped { is_first }) => {
                self.is_first = is_first;

                return Ok(());
            }
            None => return Ok(()),
        };

        self.write_data(writer, &data)?;

        Ok(())
    }

    fn begin_object_key<W: Write + ?Sized>(
        &mut self,
        _writer: &mut W,
        first: bool,
    ) -> Result<(), Error> {
        let mut buffer = Vec::new();

        let is_first = first || self.is_first;

        self.inner.begin_object_key(&mut buffer, is_first)?;

        self.pending_entries.push(Entry::Pending {
            buffer,
            is_first,
            child_count: 0,
        });

        Ok(())
    }

    fn end_object_value<W: Write + ?Sized>(&mut self, writer: &mut W) -> Result<(), Error> {
        let data = match self.pending_entries.pop() {
            Some(Entry::Pending { mut buffer, .. }) => {
                self.is_first = false;
                self.inner.end_object_value(&mut buffer)?;

                buffer
            }
            Some(Entry::Dropped { is_first }) => {
                self.is_first = is_first;

                return Ok(());
            }
            None => return Ok(()),
        };

        self.write_data(writer, &data)?;

        Ok(())
    }
}
