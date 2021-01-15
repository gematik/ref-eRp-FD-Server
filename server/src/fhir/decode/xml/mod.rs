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

mod decoder;
mod error;
mod reader;

pub use decoder::Xml;
pub use error::Error;

use std::fmt::{Debug, Display};

use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::Stream;

use super::{decode_any, Decode, DecodeError, DecodeStream, Fields};

#[async_trait(?Send)]
pub trait XmlDecode<'a> {
    type Error: Debug + Display;

    async fn xml<T: Decode>(&mut self) -> Result<T, DecodeError<Error<Self::Error>>>;
}

#[async_trait(?Send)]
impl<'a, S, E> XmlDecode<'a> for S
where
    S: Stream<Item = Result<Bytes, E>> + Unpin + 'a,
    E: Display + Debug + Unpin + 'static,
{
    type Error = E;

    async fn xml<T: Decode>(&mut self) -> Result<T, DecodeError<Error<Self::Error>>> {
        let stream = Xml::new(self);
        let mut stream = DecodeStream::new(stream);

        stream.decode(&mut Fields::Any, decode_any).await
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use super::super::DataStream;

    use super::super::tests::{
        assert_stream_extended_array, assert_stream_extended_value, assert_stream_resource,
        assert_stream_task, assert_stream_task_create_parameters, load_str, load_stream,
    };

    #[tokio::test]
    async fn decode_task() {
        let xml = load_stream("./examples/task.xml");
        let xml = Xml::new(xml);

        assert_stream_task(xml).await;
    }

    #[tokio::test]
    async fn decode_task_create_parameters() {
        let xml = load_stream("./examples/task_create_parameters.xml");
        let xml = Xml::new(xml);

        assert_stream_task_create_parameters(xml).await;
    }

    #[tokio::test]
    async fn decode_resource() {
        let xml = load_str(
            r##"
                <Root xmlns="http://hl7.org/fhir">
                    <resource>
                        <Resource xmlns="http://hl7.org/fhir">
                            <key value="value" />
                        </Resource>
                    </resource>
                </Root>
            "##,
        );
        let xml = Xml::new(xml);

        assert_stream_resource(xml).await;
    }

    #[tokio::test]
    async fn decode_extended_value() {
        let xml = load_str(
            r##"
                <Test xmlns="http://hl7.org/fhir">
                    <name value="value">
                        <extension>
                            <fuu value="bar" />
                        </extension>
                    </name>
                </Test>
            "##,
        );
        let xml = Xml::new(xml);

        assert_stream_extended_value(xml).await;
    }

    #[tokio::test]
    async fn decode_extended_array() {
        let xml = load_str(
            r##"
                <Test xmlns="http://hl7.org/fhir">
                    <name value="value1">
                        <extension>
                            <fuu value="bar1" />
                        </extension>
                    </name>
                    <name value="value2">
                        <extension>
                            <fuu value="bar2" />
                        </extension>
                    </name>
                </Test>
            "##,
        );
        let xml = Xml::new(xml);

        assert_stream_extended_array(xml).await;
    }

    #[tokio::test]
    async fn decode_stream() {
        let mut stream = load_str(
            r##"
                <Test xmlns="http://hl7.org/fhir">
                    <name value="value1" />
                    <name value="value2" />
                </Test>
            "##,
        );
        let actual = stream.xml::<Test>().await.unwrap();
        let expected = Test {
            name: vec!["value1".into(), "value2".into()],
        };

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn decode_stream_with_comment() {
        let mut stream = load_str(
            r##"
                <Test xmlns="http://hl7.org/fhir">
                    <name value="value1" />
                    <!-- This is a test comment -->
                    <name value="value2" />
                </Test>
            "##,
        );
        let actual = stream.xml::<Test>().await.unwrap();
        let expected = Test {
            name: vec!["value1".into(), "value2".into()],
        };

        assert_eq!(actual, expected);
    }

    #[derive(Debug, PartialEq)]
    struct Test {
        name: Vec<String>,
    }

    #[async_trait(?Send)]
    impl Decode for Test {
        async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
        where
            S: DataStream,
        {
            stream.root("Test").await?;

            let mut fields = Fields::new(&["name"]);
            let name = stream.decode_vec(&mut fields, decode_any).await?;

            stream.end().await?;

            Ok(Test { name })
        }
    }
}
