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

mod encoder;
mod error;
mod writer;

pub use encoder::Json;
pub use error::Error;

use bytes::Bytes;

use super::{byte_stream::ByteStream, item::ItemStream, Encode, EncodeError, EncodeStream};

use writer::Writer;

pub trait JsonEncode {
    fn json(self) -> Result<Bytes, EncodeError<Error>>;

    fn json_stream(self) -> Result<ByteStream<Json<ItemStream>>, EncodeError<String>>;
}

impl<T> JsonEncode for T
where
    T: Encode,
{
    fn json(self) -> Result<Bytes, EncodeError<Error>> {
        let mut writer = Writer::default();
        let mut encode_stream = EncodeStream::new(&mut writer);

        self.encode(&mut encode_stream)?;
        writer.write(None).map_err(EncodeError::Data)?;

        Ok(writer.freeze())
    }

    fn json_stream(self) -> Result<ByteStream<Json<ItemStream>>, EncodeError<String>> {
        let mut stream = ItemStream::default();
        let mut encode_stream = EncodeStream::new(&mut stream);

        self.encode(&mut encode_stream)?;

        let stream = Json::new(stream);

        Ok(stream)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::fs::read_to_string;
    use std::str::from_utf8;

    use super::super::{
        super::tests::trim_json_str,
        tests::{
            stream_extended_array, stream_extended_value, stream_extended_value_empty,
            stream_resource, stream_task, stream_task_create_parameters,
        },
    };

    #[tokio::test]
    async fn encode_task() {
        let stream = stream_task();
        let stream = Json::new(stream);

        let actual = stream.into_bytes().await.unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/task.json").unwrap();

        assert_eq!(trim_json_str(&expected), trim_json_str(&actual));
    }

    #[tokio::test]
    async fn encode_task_create_parameters() {
        let stream = stream_task_create_parameters();
        let stream = Json::new(stream);

        let actual = stream.into_bytes().await.unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/task_create_parameters.json").unwrap();

        assert_eq!(trim_json_str(&expected), trim_json_str(&actual));
    }

    #[tokio::test]
    async fn encode_resource() {
        let stream = stream_resource();
        let stream = Json::new(stream);

        let actual = stream.into_bytes().await.unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = r##"{
            "resourceType":"Root",
            "resource":{
                "resourceType":"Resource",
                "key":"value"
            }
        }"##;

        assert_eq!(trim_json_str(&expected), trim_json_str(&actual));
    }

    #[tokio::test]
    async fn encode_extended_value() {
        let stream = stream_extended_value();
        let stream = Json::new(stream);

        let actual = stream.into_bytes().await.unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = r##"{
            "resourceType":"Test",
            "name":"value",
            "_name":{
                "extension":[{
                    "fuu":"bar"
                }]
            }
        }"##;

        assert_eq!(trim_json_str(&expected), trim_json_str(&actual));
    }

    #[tokio::test]
    async fn encode_extended_array() {
        let stream = stream_extended_array();
        let stream = Json::new(stream);

        let actual = stream.into_bytes().await.unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = r##"{
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
        }"##;

        assert_eq!(trim_json_str(&expected), trim_json_str(&actual));
    }

    #[tokio::test]
    async fn encode_extended_value_empty() {
        let stream = stream_extended_value_empty();
        let stream = Json::new(stream);

        let actual = stream.into_bytes().await.unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = r##"{
            "resourceType":"Test",
            "name":"value"
        }"##;

        assert_eq!(trim_json_str(&expected), trim_json_str(&actual));
    }
}
