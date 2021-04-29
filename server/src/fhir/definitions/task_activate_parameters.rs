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

use async_trait::async_trait;
use resources::task::TaskActivateParameters;

use resources::primitives::Id;

use crate::fhir::{
    decode::{DataStream, Decode, DecodeError, DecodeStream, Fields},
    encode::{encode_any, DataStorage, Encode, EncodeError, EncodeStream},
};

use super::primitives::{decode_binary, encode_binary, BinaryEx};

/* Decode */

#[async_trait(?Send)]
impl Decode for TaskActivateParameters {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["parameter"]);

        stream.root("Parameters").await?;
        stream.begin_substream(&mut fields).await?;

        let mut fields = Fields::new(&["name", "resource"]);
        stream.element().await?;

        let _ = stream.fixed(&mut fields, "ePrescription").await?;
        let parameters = stream.resource(&mut fields, decode_binary).await?;

        stream.end().await?;
        stream.end_substream().await?;
        stream.end().await?;

        Ok(parameters)
    }
}

/* Encode */

impl Encode for &TaskActivateParameters {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .root("Parameters")?
            .field_name("parameter")?
            .array()?
            .element()?
            .encode("name", "ePrescription", encode_any)?
            .resource("resource", self, encode_binary)?
            .end()?
            .end()?
            .end()?;

        Ok(())
    }
}

/* Misc */

impl BinaryEx for TaskActivateParameters {
    fn from_parts(_id: Option<Id>, data: String) -> Result<Self, String> {
        Ok(TaskActivateParameters { data })
    }

    fn id(&self) -> Option<&Id> {
        None
    }

    fn data(&self) -> String {
        self.data.clone()
    }

    fn content_type() -> Option<&'static str> {
        Some("application/pkcs7-mime")
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::fs::read_to_string;
    use std::str::from_utf8;

    use crate::fhir::{
        decode::{tests::load_stream, JsonDecode, XmlDecode},
        encode::{JsonEncode, XmlEncode},
    };

    use super::super::super::tests::{trim_json_str, trim_xml_str};

    #[tokio::test]
    async fn test_decode_json() {
        let mut stream = load_stream("./examples/task_activate_parameters.json");

        let actual = stream.json::<TaskActivateParameters>().await.unwrap();
        let expected = test_task_activate_parameters();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_decode_xml() {
        let mut stream = load_stream("./examples/task_activate_parameters.xml");

        let actual = stream.xml::<TaskActivateParameters>().await.unwrap();
        let expected = test_task_activate_parameters();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_encode_json() {
        let value = test_task_activate_parameters();

        let actual = (&value).json().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/task_activate_parameters.json").unwrap();

        assert_eq!(trim_json_str(&actual), trim_json_str(&expected));
    }

    #[tokio::test]
    async fn test_encode_xml() {
        let value = test_task_activate_parameters();

        let actual = (&value).xml().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/task_activate_parameters.xml").unwrap();

        assert_eq!(trim_xml_str(&actual), trim_xml_str(&expected));
    }

    fn test_task_activate_parameters() -> TaskActivateParameters {
        TaskActivateParameters {
            data: std::fs::read_to_string("./examples/cms.base64").unwrap(),
        }
    }
}
