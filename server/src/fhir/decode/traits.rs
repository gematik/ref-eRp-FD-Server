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

use std::str::FromStr;

use async_trait::async_trait;

use super::{DataStream, DecodeError, DecodeStream, Search};

pub async fn decode_any<T, S>(stream: &mut DecodeStream<S>) -> Result<T, DecodeError<S::Error>>
where
    T: Decode,
    S: DataStream,
{
    let value = T::decode(stream).await?;

    Ok(value)
}

#[async_trait(?Send)]
pub trait Decode: Sized {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream;
}

pub trait FromString: FromStr {
    fn parse(s: String) -> Result<Self, String> {
        match s.parse() {
            Ok(value) => Ok(value),
            Err(_) => Err(s),
        }
    }
}

#[async_trait(?Send)]
impl Decode for String {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let value = stream.value(Search::Any).await?.unwrap();

        Ok(value)
    }
}

#[async_trait(?Send)]
impl<T: FromString> Decode for T {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let value = stream.value(Search::Any).await?.unwrap();
        let value = match T::parse(value) {
            Ok(value) => value,
            Err(value) => {
                return Err(DecodeError::InvalidValue {
                    value,
                    path: stream.path().into(),
                })
            }
        };

        Ok(value)
    }
}

impl FromString for usize {}
impl FromString for isize {}
impl FromString for f64 {}

impl FromString for bool {
    fn parse(s: String) -> Result<Self, String> {
        match s.as_str() {
            "1" => Ok(true),
            "t" => Ok(true),
            "T" => Ok(true),
            "true" => Ok(true),
            "TRUE" => Ok(true),
            "0" => Ok(false),
            "f" => Ok(false),
            "F" => Ok(false),
            "false" => Ok(false),
            "FALSE" => Ok(false),
            _ => Err(s),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::pin::Pin;
    use std::task::{Context, Poll};

    use futures::stream::{iter, Stream, StreamExt};

    use crate::fhir::{Format, WithFormat};

    use super::super::{DecodeError, DecodeStream, Fields, Item, Search};

    struct StreamWithFormat<S> {
        stream: S,
        format: Option<Format>,
    }

    impl<S> StreamWithFormat<S> {
        pub fn new(stream: S, format: Option<Format>) -> Self {
            Self { stream, format }
        }
    }

    impl<S> WithFormat for StreamWithFormat<S> {
        fn format(&self) -> Option<Format> {
            self.format
        }
    }

    impl<S> Stream for StreamWithFormat<S>
    where
        S: Stream + Unpin,
    {
        type Item = S::Item;

        fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            self.stream.poll_next_unpin(cx)
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            self.stream.size_hint()
        }
    }

    #[derive(Debug, PartialEq)]
    struct Element {
        fuu: String,
        bar: Option<usize>,
    }

    #[async_trait(?Send)]
    impl Decode for Element {
        async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
        where
            S: DataStream,
        {
            stream.element().await?;

            let mut fields = Fields::new(&["fuu", "bar"]);
            let fuu = stream.decode(&mut fields, decode_any).await?;
            let bar = stream.decode_opt(&mut fields, decode_any).await?;

            stream.end().await?;

            Ok(Element { fuu, bar })
        }
    }

    #[tokio::test]
    async fn decode_value() {
        let stream = vec![Item::Field {
            name: "test".into(),
            value: "value".into(),
            extension: Vec::new(),
        }];
        let stream = iter(stream.into_iter().map(Result::<Item, String>::Ok));
        let stream = StreamWithFormat::new(stream, None);
        let mut stream = DecodeStream::new(stream);

        let actual = stream
            .decode::<String, _>(&mut Fields::Any, decode_any)
            .await
            .unwrap();
        let expected = "value";

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn decode_value_some() {
        let stream = vec![Item::Field {
            name: "test".into(),
            value: "value".into(),
            extension: Vec::new(),
        }];
        let stream = iter(stream.into_iter().map(Result::<Item, String>::Ok));
        let stream = StreamWithFormat::new(stream, None);
        let mut stream = DecodeStream::new(stream);

        let mut fields = Fields::new(&["test"]);
        let actual = stream
            .decode_opt::<Option<String>, _>(&mut fields, decode_any)
            .await
            .unwrap();
        let expected = Some("value".to_owned());

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn decode_value_none() {
        let stream = vec![Item::Field {
            name: "test".into(),
            value: "value".into(),
            extension: Vec::new(),
        }];
        let stream = iter(stream.into_iter().map(Result::<Item, String>::Ok));
        let stream = StreamWithFormat::new(stream, None);
        let mut stream = DecodeStream::new(stream);

        let mut fields = Fields::new(&["fuu"]);
        let actual = stream
            .decode_opt::<Option<String>, _>(&mut fields, decode_any)
            .await
            .unwrap();
        let expected = None;

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn decode_value_vec() {
        let stream = vec![
            Item::Field {
                name: "test".into(),
                value: "value0".into(),
                extension: Vec::new(),
            },
            Item::Field {
                name: "test".into(),
                value: "value1".into(),
                extension: Vec::new(),
            },
            Item::Field {
                name: "test".into(),
                value: "value2".into(),
                extension: Vec::new(),
            },
        ];
        let stream = iter(stream.into_iter().map(Result::<Item, String>::Ok));
        let stream = StreamWithFormat::new(stream, None);
        let mut stream = DecodeStream::new(stream);

        let mut fields = Fields::new(&["test"]);
        let actual = stream
            .decode_vec::<Vec<String>, _>(&mut fields, decode_any)
            .await
            .unwrap();
        let expected = vec![
            "value0".to_owned(),
            "value1".to_owned(),
            "value2".to_owned(),
        ];

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn decode_element() {
        let stream = vec![
            Item::BeginElement {
                name: "element".into(),
            },
            Item::Field {
                name: "fuu".into(),
                value: "value0".into(),
                extension: Vec::new(),
            },
            Item::EndElement,
        ];
        let stream = iter(stream.into_iter().map(Result::<Item, String>::Ok));
        let stream = StreamWithFormat::new(stream, None);
        let mut stream = DecodeStream::new(stream);

        let actual = stream
            .decode::<Element, _>(&mut Fields::Any, decode_any)
            .await
            .unwrap();
        let expected = Element {
            fuu: "value0".into(),
            bar: None,
        };

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn decode_element_some() {
        let stream = vec![
            Item::BeginElement {
                name: "element".into(),
            },
            Item::Field {
                name: "fuu".into(),
                value: "value0".into(),
                extension: Vec::new(),
            },
            Item::EndElement,
        ];
        let stream = iter(stream.into_iter().map(Result::<Item, String>::Ok));
        let stream = StreamWithFormat::new(stream, None);
        let mut stream = DecodeStream::new(stream);

        let actual = stream
            .decode_opt::<Option<Element>, _>(&mut Fields::new(&["element"]), decode_any)
            .await
            .unwrap();
        let expected = Some(Element {
            fuu: "value0".into(),
            bar: None,
        });

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn decode_element_vec() {
        let stream = vec![
            Item::BeginElement {
                name: "element".into(),
            },
            Item::Field {
                name: "fuu".into(),
                value: "value0".into(),
                extension: Vec::new(),
            },
            Item::EndElement,
            Item::BeginElement {
                name: "element".into(),
            },
            Item::Field {
                name: "fuu".into(),
                value: "value1".into(),
                extension: Vec::new(),
            },
            Item::Field {
                name: "bar".into(),
                value: "123".into(),
                extension: Vec::new(),
            },
            Item::EndElement,
        ];
        let stream = iter(stream.into_iter().map(Result::<Item, String>::Ok));
        let stream = StreamWithFormat::new(stream, None);
        let mut stream = DecodeStream::new(stream);

        let actual = stream
            .decode_vec::<Vec<Element>, _>(&mut Fields::new(&["element"]), decode_any)
            .await
            .unwrap();
        let expected = vec![
            Element {
                fuu: "value0".into(),
                bar: None,
            },
            Element {
                fuu: "value1".into(),
                bar: Some(123),
            },
        ];

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn decode_vec_custom() {
        let stream = vec![
            Item::Field {
                name: "test".into(),
                value: "value0".into(),
                extension: Vec::new(),
            },
            Item::Field {
                name: "test".into(),
                value: "value1".into(),
                extension: Vec::new(),
            },
        ];
        let stream = iter(stream.into_iter().map(Result::<Item, String>::Ok));
        let stream = StreamWithFormat::new(stream, None);
        let mut stream = DecodeStream::new(stream);

        let actual = stream
            .decode_vec::<Vec<String>, _>(&mut Fields::Any, decode)
            .await
            .unwrap();
        let expected = vec!["value0".to_owned(), "value1".to_owned()];

        assert_eq!(actual, expected);
    }

    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<String, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        Ok(stream.value(Search::Any).await?.unwrap())
    }
}
