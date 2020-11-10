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

mod decoder;
mod error;
mod reader;

pub use decoder::Json;
pub use error::Error;

use std::fmt::{Debug, Display};

use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::Stream;

use super::{decode_any, Decode, DecodeError, DecodeStream, Fields};

#[async_trait(?Send)]
pub trait JsonDecode<'a> {
    type Error: Debug + Display;

    async fn json<T: Decode>(&mut self) -> Result<T, DecodeError<Error<Self::Error>>>;
}

#[async_trait(?Send)]
impl<'a, S, E> JsonDecode<'a> for S
where
    S: Stream<Item = Result<Bytes, E>> + Unpin + 'a,
    E: Display + Debug + Unpin + 'static,
{
    type Error = E;

    async fn json<T: Decode>(&mut self) -> Result<T, DecodeError<Error<Self::Error>>> {
        let stream = Json::new(self);
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
        let json = load_stream("./examples/task.json");
        let json = Json::new(json);

        assert_stream_task(json).await;
    }

    #[tokio::test]
    async fn decode_task_create_parameters() {
        let json = load_stream("./examples/task_create_parameters.json");
        let json = Json::new(json);

        assert_stream_task_create_parameters(json).await;
    }

    #[tokio::test]
    async fn decode_resource() {
        let json = load_str(
            r##"{
                "resourceType":"Root",
                "resource":{
                    "resourceType":"Resource",
                    "key":"value"
                }
            }"##,
        );
        let json = Json::new(json);

        assert_stream_resource(json).await;
    }

    #[tokio::test]
    async fn decode_extended_value() {
        let json = load_str(
            r##"{
                "resourceType":"Test",
                "name":"value",
                "_name":{
                    "extension":[{
                        "fuu":"bar"
                    }]
                }
            }"##,
        );
        let json = Json::new(json);

        assert_stream_extended_value(json).await;
    }

    #[tokio::test]
    async fn decode_extended_array() {
        let json = load_str(
            r##"{
                "resourceType":"Test",
                "name":["value1","value2"],
                "_name":[{
                    "extension":[{
                        "fuu":"bar1"
                    }]
                },{
                    "extension":[{
                        "fuu":"bar2"
                    }]
                }]
            }"##,
        );
        let json = Json::new(json);

        assert_stream_extended_array(json).await;
    }

    #[tokio::test]
    async fn decode_stream() {
        let mut stream = load_str(
            r##"{
                "resourceType":"Test",
                "name":["value1","value2"]
            }"##,
        );
        let actual = stream.json::<Test>().await.unwrap();
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
