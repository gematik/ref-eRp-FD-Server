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

use std::char::from_u32;
use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::str::from_utf8;

use bytes::Bytes;
use futures::stream::Stream;

use super::{
    super::byte_stream::{ByteStream, StreamError},
    error::Error,
};

pub struct Reader<'a, S, E>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin + 'a,
    E: Display + Debug,
{
    stream: ByteStream<S>,
    buffer: Option<Element>,
    state: Vec<State>,
    marker: PhantomData<&'a &'a mut ()>,
}

#[derive(Debug, PartialEq)]
pub enum Element {
    BeginArray,
    EndArray,
    BeginObject,
    EndObject,
    Field(String),
    Value(Option<String>),
}

enum State {
    Value,
    Array,
    Object,
}

enum Return {
    Element(Element),
    Loop,
}

impl<'a, S, E> Reader<'a, S, E>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin + 'a,
    E: Display + Debug,
{
    pub fn new(stream: S) -> Self {
        Self {
            stream: ByteStream::new(stream),
            state: vec![State::Value],
            buffer: None,
            marker: PhantomData,
        }
    }

    pub fn put_back(&mut self, element: Element) {
        if self.buffer.is_some() {
            panic!("Unable to put back more than one element!");
        }

        self.buffer = Some(element);
    }

    pub async fn next(&mut self) -> Result<Option<Element>, Error<E>> {
        if let Some(element) = self.buffer.take() {
            return Ok(Some(element));
        }

        loop {
            let ret = match self.state.pop() {
                Some(State::Value) => self.decode_value().await?,
                Some(State::Array) => self.decode_array().await?,
                Some(State::Object) => self.decode_object().await?,
                None => {
                    self.decode_eof().await?;

                    return Ok(None);
                }
            };

            match ret {
                Return::Element(element) => return Ok(Some(element)),
                Return::Loop => (),
            }
        }
    }

    async fn decode_value(&mut self) -> Result<Return, Error<E>> {
        self.stream.drop_whitespaces().await?;

        match self.stream.peek().await? {
            Some(b'n') => {
                self.stream.expect(b"null").await?;

                Ok(Return::Element(Element::Value(None)))
            }
            Some(b't') => {
                self.stream.expect(b"true").await?;

                Ok(Return::Element(Element::Value(Some("true".into()))))
            }
            Some(b'f') => {
                self.stream.expect(b"false").await?;

                Ok(Return::Element(Element::Value(Some("false".into()))))
            }
            Some(b'-') | Some(b'0'..=b'9') => {
                let mut dot = true;
                let value = self
                    .stream
                    .take_while(move |i, v| is_number(&mut dot, i, v))
                    .await?
                    .unwrap();

                if value.len() == 1 && value.starts_with(b"-") {
                    return Err(StreamError::UnexpectedEof.into());
                }

                let value = from_utf8(&value[..])?;

                Ok(Return::Element(Element::Value(Some(value.into()))))
            }
            Some(b'"') => {
                let _ = self.stream.take().await?.unwrap();

                let value = self.decode_str().await?;

                Ok(Return::Element(Element::Value(Some(value))))
            }
            Some(b'[') => {
                self.state.push(State::Array);

                Ok(Return::Element(Element::BeginArray))
            }
            Some(b'{') => {
                self.state.push(State::Object);

                Ok(Return::Element(Element::BeginObject))
            }
            None => Err(StreamError::UnexpectedEof.into()),
            _ => Err(Error::ExpectedValue),
        }
    }

    async fn decode_array(&mut self) -> Result<Return, Error<E>> {
        self.stream.drop_whitespaces().await?;

        match self.stream.take().await? {
            Some(b'[') | Some(b',') => {
                self.state.push(State::Array);
                self.state.push(State::Value);

                Ok(Return::Loop)
            }
            Some(b']') => Ok(Return::Element(Element::EndArray)),
            None => Err(StreamError::UnexpectedEof.into()),
            _ => Err(StreamError::UnexpectedIdent.into()),
        }
    }

    async fn decode_object(&mut self) -> Result<Return, Error<E>> {
        self.stream.drop_whitespaces().await?;

        match self.stream.take().await? {
            Some(b'{') | Some(b',') => {
                self.stream.drop_whitespaces().await?;
                self.stream.expect(b"\"").await?;

                let name = self.decode_str().await?;

                self.stream.drop_whitespaces().await?;
                self.stream.expect(b":").await?;

                self.state.push(State::Object);
                self.state.push(State::Value);

                Ok(Return::Element(Element::Field(name)))
            }
            Some(b'}') => Ok(Return::Element(Element::EndObject)),
            None => Err(StreamError::UnexpectedEof.into()),
            _ => Err(StreamError::UnexpectedIdent.into()),
        }
    }

    async fn decode_eof(&mut self) -> Result<(), Error<E>> {
        self.stream.drop_whitespaces().await?;

        if self.stream.buffer().await?.is_some() {
            Err(Error::ExpectedEoF)
        } else {
            Ok(())
        }
    }

    async fn decode_str(&mut self) -> Result<String, Error<E>> {
        let mut escape = false;
        let s = self
            .stream
            .take_while(move |_, v| is_string(&mut escape, v))
            .await?;

        let s = match s {
            Some(s) => s,
            None => return Err(Error::UnexpectedEoS),
        };

        self.stream.expect(b"\"").await?;

        let s = decode_str(&s[..])?;

        Ok(s)
    }
}

fn decode_str<E>(buf: &[u8]) -> Result<String, Error<E>>
where
    E: Display + Debug,
{
    let s = from_utf8(buf)?;

    let mut chars = s.chars();
    let mut s = String::default();

    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next().ok_or_else(|| Error::InvalidEscape)? {
                '"' => s.push('"'),
                '\\' => s.push('\\'),
                '/' => s.push('/'),
                'b' => s.push('\x08'),
                'f' => s.push('\x0c'),
                'n' => s.push('\n'),
                'r' => s.push('\r'),
                't' => s.push('\t'),
                'u' => match decode_escape_str(&mut chars)? {
                    0xDC00..=0xDFFF => {
                        return Err(Error::InvalidEscape);
                    }

                    n1 @ 0xD800..=0xDBFF => {
                        if chars.next() != Some('\\') {
                            return Err(Error::InvalidEscape);
                        }

                        if chars.next() != Some('u') {
                            return Err(Error::InvalidEscape);
                        }

                        let n2 = decode_escape_str(&mut chars)?;

                        if n2 < 0xDC00 || n2 > 0xDFFF {
                            return Err(Error::InvalidEscape);
                        }

                        let n = (((n1 - 0xD800) as u32) << 10 | (n2 - 0xDC00) as u32) + 0x1_0000;

                        match from_u32(n) {
                            Some(c) => s.push(c),
                            None => return Err(Error::InvalidEscape),
                        }
                    }

                    n => match from_u32(n as u32) {
                        Some(c) => s.push(c),
                        None => return Err(Error::InvalidEscape),
                    },
                },
                _ => return Err(Error::InvalidEscape),
            },
            c => s.push(c),
        }
    }

    Ok(s)
}

fn decode_escape_str<I, E>(i: &mut I) -> Result<u16, Error<E>>
where
    I: Iterator<Item = char>,
    E: Display + Debug,
{
    let mut n = 0;

    for _ in 0..4 {
        match i.next() {
            Some(i @ '0'..='9') => n = (n << 4) + (i as u8 - b'0') as u16,
            Some(i @ 'A'..='F') => n = (n << 4) + (i as u8 - b'A' + 10) as u16,
            Some(i @ 'a'..='f') => n = (n << 4) + (i as u8 - b'a' + 10) as u16,
            _ => return Err(Error::InvalidEscape),
        }
    }

    Ok(n)
}

fn is_number(dot: &mut bool, i: usize, v: u8) -> bool {
    if *dot && v == b'.' {
        *dot = false;

        return true;
    }

    (v >= b'0' && v <= b'9') || (i == 0 && v == b'-')
}

fn is_string(escape: &mut bool, v: u8) -> bool {
    if *escape {
        *escape = false;

        true
    } else if v == b'\\' {
        *escape = true;

        true
    } else {
        v != b'"'
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use futures::stream::iter;

    #[tokio::test]
    async fn decode_null() {
        let mut json = from_str(&["   null   "]);

        assert_eq!(Some(Element::Value(None)), json.next().await.unwrap());

        assert_eq!(None, json.next().await.unwrap());
    }

    #[tokio::test]
    async fn decode_true() {
        let mut json = from_str(&["   true   "]);

        assert_eq!(
            Some(Element::Value(Some("true".into()))),
            json.next().await.unwrap()
        );

        assert_eq!(None, json.next().await.unwrap());
    }

    #[tokio::test]
    async fn decode_false() {
        let mut json = from_str(&["     false   "]);

        assert_eq!(
            Some(Element::Value(Some("false".into()))),
            json.next().await.unwrap()
        );

        assert_eq!(None, json.next().await.unwrap());
    }

    #[tokio::test]
    async fn decode_number_unsigned() {
        let mut json = from_str(&["123"]);

        assert_eq!(
            Some(Element::Value(Some("123".into()))),
            json.next().await.unwrap()
        );

        assert_eq!(None, json.next().await.unwrap());
    }

    #[tokio::test]
    async fn decode_number_signed() {
        let mut json = from_str(&["-123"]);

        assert_eq!(
            Some(Element::Value(Some("-123".into()))),
            json.next().await.unwrap()
        );

        assert_eq!(None, json.next().await.unwrap());
    }

    #[tokio::test]
    async fn decode_number_float() {
        let mut json = from_str(&["-123.456"]);

        assert_eq!(
            Some(Element::Value(Some("-123.456".into()))),
            json.next().await.unwrap()
        );

        assert_eq!(None, json.next().await.unwrap());
    }

    #[tokio::test]
    async fn decode_number_float_err() {
        let mut json = from_str(&["-123.456.789"]);

        assert_eq!(
            Some(Element::Value(Some("-123.456".into()))),
            json.next().await.unwrap()
        );

        assert_eq!(true, json.next().await.is_err());
    }

    #[tokio::test]
    async fn decode_string() {
        let mut json =
            from_str(&[" \"test \\\" \\\\ \\/ \\b \\f \\n \\r \\t \\uD83D\\uDE42\"    "]);

        assert_eq!(
            Some(Element::Value(Some(
                "test \" \\ / \x08 \x0C \n \r \t 🙂".into()
            ))),
            json.next().await.unwrap()
        );

        assert_eq!(None, json.next().await.unwrap());
    }

    #[tokio::test]
    async fn decode_array() {
        let mut json = from_str(&[r##"
                [
                    123,
                    456.789,
                    "test",
                    -147
                ]
            "##]);

        assert_eq!(Some(Element::BeginArray), json.next().await.unwrap());

        assert_eq!(
            Some(Element::Value(Some("123".into()))),
            json.next().await.unwrap()
        );

        assert_eq!(
            Some(Element::Value(Some("456.789".into()))),
            json.next().await.unwrap()
        );

        assert_eq!(
            Some(Element::Value(Some("test".into()))),
            json.next().await.unwrap()
        );

        assert_eq!(
            Some(Element::Value(Some("-147".into()))),
            json.next().await.unwrap()
        );

        assert_eq!(Some(Element::EndArray), json.next().await.unwrap());

        assert_eq!(None, json.next().await.unwrap());
    }

    #[tokio::test]
    async fn decode_object() {
        let mut json = from_str(&[r##"
            {
                "test1": 123,
                "test2": 456.789,
                "test3": "test",
                "test4": -147,
                "test5": [
                    123,
                    456,
                    789
                ]
            }
            "##]);

        assert_eq!(Some(Element::BeginObject), json.next().await.unwrap());

        assert_eq!(
            Some(Element::Field("test1".into())),
            json.next().await.unwrap()
        );

        assert_eq!(
            Some(Element::Value(Some("123".into()))),
            json.next().await.unwrap()
        );

        assert_eq!(
            Some(Element::Field("test2".into())),
            json.next().await.unwrap()
        );

        assert_eq!(
            Some(Element::Value(Some("456.789".into()))),
            json.next().await.unwrap()
        );

        assert_eq!(
            Some(Element::Field("test3".into())),
            json.next().await.unwrap()
        );

        assert_eq!(
            Some(Element::Value(Some("test".into()))),
            json.next().await.unwrap()
        );

        assert_eq!(
            Some(Element::Field("test4".into())),
            json.next().await.unwrap()
        );

        assert_eq!(
            Some(Element::Value(Some("-147".into()))),
            json.next().await.unwrap()
        );

        assert_eq!(
            Some(Element::Field("test5".into())),
            json.next().await.unwrap()
        );

        assert_eq!(Some(Element::BeginArray), json.next().await.unwrap());

        assert_eq!(
            Some(Element::Value(Some("123".into()))),
            json.next().await.unwrap()
        );

        assert_eq!(
            Some(Element::Value(Some("456".into()))),
            json.next().await.unwrap()
        );

        assert_eq!(
            Some(Element::Value(Some("789".into()))),
            json.next().await.unwrap()
        );

        assert_eq!(Some(Element::EndArray), json.next().await.unwrap());

        assert_eq!(Some(Element::EndObject), json.next().await.unwrap());

        assert_eq!(None, json.next().await.unwrap());
    }

    fn from_str(
        stream: &'static [&'static str],
    ) -> Reader<impl Stream<Item = Result<Bytes, String>> + Send + Unpin, String> {
        Reader::new(iter(
            stream.iter().map(|s| Ok(Bytes::from_static(s.as_bytes()))),
        ))
    }
}
