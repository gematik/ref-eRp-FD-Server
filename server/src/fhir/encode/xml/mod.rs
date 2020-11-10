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

mod encoder;
mod error;
mod writer;

pub use encoder::Xml;
pub use error::Error;

use bytes::Bytes;

use super::{byte_stream::ByteStream, item::ItemStream, Encode, EncodeError, EncodeStream};

use writer::Writer;

pub trait XmlEncode {
    fn xml(self) -> Result<Bytes, EncodeError<Error>>;

    fn xml_stream(self) -> Result<ByteStream<Xml<ItemStream>>, EncodeError<String>>;
}

impl<T> XmlEncode for T
where
    T: Encode,
{
    fn xml(self) -> Result<Bytes, EncodeError<Error>> {
        let mut writer = Writer::default();
        let mut encode_stream = EncodeStream::new(&mut writer);

        self.encode(&mut encode_stream)?;
        writer.write(None).map_err(EncodeError::Data)?;

        Ok(writer.freeze())
    }

    fn xml_stream(self) -> Result<ByteStream<Xml<ItemStream>>, EncodeError<String>> {
        let mut stream = ItemStream::default();
        let mut encode_stream = EncodeStream::new(&mut stream);

        self.encode(&mut encode_stream)?;

        let stream = Xml::new(stream);

        Ok(stream)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::fs::read_to_string;
    use std::str::from_utf8;

    use super::super::{
        super::tests::trim_xml_str,
        tests::{
            stream_extended_array, stream_extended_value, stream_resource, stream_task,
            stream_task_create_parameters,
        },
    };

    #[tokio::test]
    async fn encode_task() {
        let stream = stream_task();
        let stream = Xml::new(stream);

        let actual = stream.into_bytes().await.unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/task.xml").unwrap();

        assert_eq!(trim_xml_str(&expected), trim_xml_str(&actual));
    }

    #[tokio::test]
    async fn encode_task_create_parameters() {
        let stream = stream_task_create_parameters();
        let stream = Xml::new(stream);

        let actual = stream.into_bytes().await.unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/task_create_parameters.xml").unwrap();

        assert_eq!(trim_xml_str(&expected), trim_xml_str(&actual));
    }

    #[tokio::test]
    async fn encode_resource() {
        let stream = stream_resource();
        let stream = Xml::new(stream);

        let actual = stream.into_bytes().await.unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = r##"
            <Root xmlns="http://hl7.org/fhir">
                <resource>
                    <Resource>
                        <key value="value"/>
                    </Resource>
                </resource>
            </Root>
        "##;

        assert_eq!(trim_xml_str(&expected), trim_xml_str(&actual));
    }

    #[tokio::test]
    async fn encode_extended_value() {
        let stream = stream_extended_value();
        let stream = Xml::new(stream);

        let actual = stream.into_bytes().await.unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = r##"
            <Test xmlns="http://hl7.org/fhir">
                <name value="value">
                    <extension>
                        <fuu value="bar"/>
                    </extension>
                </name>
            </Test>
        "##;

        assert_eq!(trim_xml_str(&expected), trim_xml_str(&actual));
    }

    #[tokio::test]
    async fn encode_extended_array() {
        let stream = stream_extended_array();
        let stream = Xml::new(stream);

        let actual = stream.into_bytes().await.unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = r##"
            <Test xmlns="http://hl7.org/fhir">
                <name value="value1">
                    <extension>
                        <fuu value="bar1"/>
                    </extension>
                </name>
                <name value="value2">
                    <extension>
                        <fuu value="bar2"/>
                    </extension>
                </name>
            </Test>
        "##;

        assert_eq!(trim_xml_str(&expected), trim_xml_str(&actual));
    }
}
