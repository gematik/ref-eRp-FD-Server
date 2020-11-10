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
    elements: Vec<String>,
    marker: PhantomData<&'a &'a mut ()>,
}

#[derive(Debug, PartialEq)]
pub enum Element {
    Text { value: String },
    Empty { name: String, attribs: Attribs },
    Begin { name: String, attribs: Attribs },
    End,
}

#[derive(Debug, Default, PartialEq)]
pub struct Attribs(pub Vec<(String, String)>);

impl<'a, S, E> Reader<'a, S, E>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin + 'a,
    E: Display + Debug,
{
    pub fn new(stream: S) -> Self {
        Self {
            stream: ByteStream::new(stream),
            buffer: None,
            elements: Vec::new(),
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

        match self.stream.peek().await? {
            Some(b'<') => {
                self.stream.take().await?;
            }
            Some(_) => {
                let value = self.stream.take_while(|_, v| v != b'<').await?.unwrap();
                let value = from_utf8(&value)?;
                let value = value.to_owned();

                return Ok(Some(Element::Text { value }));
            }
            None if !self.elements.is_empty() => return Err(StreamError::UnexpectedEof.into()),
            None => return Ok(None),
        }

        self.stream.drop_whitespaces().await?;

        let is_close = match self.stream.peek().await? {
            Some(b'/') => {
                self.stream.take().await?;

                true
            }
            Some(_) => false,
            None => return Err(StreamError::UnexpectedEof.into()),
        };

        self.stream.drop_whitespaces().await?;

        let name = self
            .stream
            .take_while(is_ident)
            .await?
            .ok_or_else(|| StreamError::UnexpectedEof)?;
        let name = from_utf8(&name)?;
        let name = name.to_owned();

        let mut attribs = Attribs::default();
        let is_empty = loop {
            self.stream.drop_whitespaces().await?;

            match self.stream.peek().await? {
                Some(b'/') => {
                    self.stream.take().await?;
                    self.stream.drop_whitespaces().await?;
                    self.stream.expect(b">").await?;

                    break true;
                }
                Some(b'>') => {
                    self.stream.take().await?;

                    break false;
                }
                Some(c) if is_ident(0, c) => {
                    let attrib = self
                        .stream
                        .take_while(is_ident)
                        .await?
                        .ok_or_else(|| StreamError::UnexpectedEof)?;
                    let attrib = from_utf8(&attrib)?;

                    self.stream.drop_whitespaces().await?;
                    self.stream.expect(b"=").await?;
                    self.stream.drop_whitespaces().await?;
                    self.stream.expect(b"\"").await?;

                    let value = self
                        .stream
                        .take_while(|_, v| v != b'"')
                        .await?
                        .ok_or_else(|| StreamError::UnexpectedEof)?;
                    let value = from_utf8(&value)?;

                    self.stream.expect(b"\"").await?;

                    attribs.0.push((attrib.into(), value.into()));
                }
                Some(_) => return Err(StreamError::UnexpectedIdent.into()),
                None => return Err(StreamError::UnexpectedEof.into()),
            }
        };

        match (is_close, is_empty) {
            (true, true) => Err(Error::InvalidTag(name)),
            (true, false) => match self.elements.pop() {
                Some(tag) if name == tag => Ok(Some(Element::End)),
                Some(tag) => Err(Error::InvalidCloseTag(tag, name)),
                None => Err(Error::ExpectedEof),
            },
            (false, true) => Ok(Some(Element::Empty { name, attribs })),
            (false, false) => {
                self.elements.push(name.clone());

                Ok(Some(Element::Begin { name, attribs }))
            }
        }
    }
}

impl Attribs {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find_map(|(n, v)| if n == name { Some(v.as_str()) } else { None })
    }
}

fn is_ident(_: usize, v: u8) -> bool {
    (v >= b'a' && v <= b'z')
        || (v >= b'A' && v <= b'Z')
        || (v >= b'0' && v <= b'9')
        || v == b'-'
        || v == b'_'
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use futures::stream::iter;

    #[tokio::test]
    async fn decode_empty() {
        let mut xml = from_str(&["   < Empty  />   "]);

        assert_eq!(
            Some(Element::Text {
                value: "   ".into(),
            }),
            xml.next().await.unwrap()
        );

        assert_eq!(
            Some(Element::Empty {
                name: "Empty".into(),
                attribs: Attribs::default()
            }),
            xml.next().await.unwrap()
        );

        assert_eq!(
            Some(Element::Text {
                value: "   ".into(),
            }),
            xml.next().await.unwrap()
        );

        assert_eq!(None, xml.next().await.unwrap());
    }

    #[tokio::test]
    async fn decode_empty_with_attribs() {
        let mut xml = from_str(&["< Empty  fuu = \"fuu value\"    bar  =\"bar value\"  />"]);

        assert_eq!(
            Some(Element::Empty {
                name: "Empty".into(),
                attribs: Attribs(vec![
                    ("fuu".into(), "fuu value".into()),
                    ("bar".into(), "bar value".into()),
                ])
            }),
            xml.next().await.unwrap()
        );

        assert_eq!(None, xml.next().await.unwrap());
    }

    #[tokio::test]
    async fn decode_element_with_text() {
        let mut xml = from_str(&["<Fuu>Test123</Fuu>"]);

        assert_eq!(
            Some(Element::Begin {
                name: "Fuu".into(),
                attribs: Attribs::default()
            }),
            xml.next().await.unwrap()
        );

        assert_eq!(
            Some(Element::Text {
                value: "Test123".into(),
            }),
            xml.next().await.unwrap()
        );

        assert_eq!(Some(Element::End), xml.next().await.unwrap());

        assert_eq!(None, xml.next().await.unwrap());
    }

    #[tokio::test]
    async fn decode_nested_with_text() {
        let mut xml = from_str(&["<Fuu>Test123<Bar>Test456</Bar><Empty fuu=\"bar\" /></Fuu>"]);

        assert_eq!(
            Some(Element::Begin {
                name: "Fuu".into(),
                attribs: Attribs::default(),
            }),
            xml.next().await.unwrap()
        );

        assert_eq!(
            Some(Element::Text {
                value: "Test123".into(),
            }),
            xml.next().await.unwrap()
        );

        assert_eq!(
            Some(Element::Begin {
                name: "Bar".into(),
                attribs: Attribs::default(),
            }),
            xml.next().await.unwrap()
        );

        assert_eq!(
            Some(Element::Text {
                value: "Test456".into(),
            }),
            xml.next().await.unwrap()
        );

        assert_eq!(Some(Element::End), xml.next().await.unwrap());

        assert_eq!(
            Some(Element::Empty {
                name: "Empty".into(),
                attribs: Attribs(vec![("fuu".into(), "bar".into())]),
            }),
            xml.next().await.unwrap()
        );

        assert_eq!(Some(Element::End), xml.next().await.unwrap());

        assert_eq!(None, xml.next().await.unwrap());
    }

    fn from_str(
        stream: &'static [&'static str],
    ) -> Reader<impl Stream<Item = Result<Bytes, String>> + Unpin, String> {
        Reader::new(iter(
            stream.iter().map(|s| Ok(Bytes::from_static(s.as_bytes()))),
        ))
    }
}
