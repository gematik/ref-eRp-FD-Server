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

use std::iter::once;

use async_trait::async_trait;
use miscellaneous::str::icase_eq;
use resources::device::{Device, DeviceName, Status, Type};

use crate::fhir::{
    decode::{decode_any, DataStream, Decode, DecodeError, DecodeStream, Fields},
    encode::{encode_any, DataStorage, Encode, EncodeError, EncodeStream},
};

use super::{
    meta::Meta,
    primitives::{decode_code, encode_code, CodeEx},
    DecodeBundleResource, EncodeBundleResource,
};

/* Decode */

impl DecodeBundleResource for Device {}

#[async_trait(?Send)]
impl Decode for Device {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&[
            "id",
            "meta",
            "status",
            "serialNumber",
            "deviceName",
            "version",
        ]);

        stream.root("Device").await?;

        let id = stream.decode(&mut fields, decode_any).await?;
        let meta = stream.decode::<Meta, _>(&mut fields, decode_any).await?;
        let status = stream.decode(&mut fields, decode_code).await?;
        let serial_number = stream.decode_opt(&mut fields, decode_any).await?;
        let device_name = stream.decode(&mut fields, decode_any).await?;
        let version = stream.decode(&mut fields, decode_version).await?;

        stream.end().await?;

        if !meta.profiles.iter().any(|p| icase_eq(p, PROFILE)) {
            return Err(DecodeError::InvalidProfile {
                actual: meta.profiles,
                expected: vec![PROFILE.into()],
            });
        }

        Ok(Device {
            id,
            status,
            serial_number,
            device_name,
            version,
        })
    }
}

#[async_trait(?Send)]
impl Decode for DeviceName {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["name", "type"]);

        stream.element().await?;

        let name = stream.decode(&mut fields, decode_any).await?;
        let type_ = stream.decode(&mut fields, decode_code).await?;

        stream.end().await?;

        Ok(DeviceName { name, type_ })
    }
}

/* Encode */

impl EncodeBundleResource for &Device {}

impl Encode for &Device {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let meta = Meta {
            profiles: vec![PROFILE.into()],
            ..Default::default()
        };

        stream
            .root("Device")?
            .encode("id", &self.id, encode_any)?
            .encode("meta", meta, encode_any)?
            .encode("status", &self.status, encode_code)?
            .encode_opt("serialNumber", &self.serial_number, encode_any)?
            .encode_vec("deviceName", once(&self.device_name), encode_any)?
            .encode_vec("version", once(&self.version), encode_version)?
            .end()?;

        Ok(())
    }
}

impl Encode for &DeviceName {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode("name", &self.name, encode_any)?
            .encode("type", &self.type_, encode_code)?
            .end()?;

        Ok(())
    }
}

/* Misc */

impl CodeEx for Status {
    fn from_parts(value: String) -> Result<Self, String> {
        match value.as_str() {
            "active" => Ok(Self::Active),
            "inactive" => Ok(Self::Inactive),
            "entered-in-error" => Ok(Self::EnteredInError),
            "unknown" => Ok(Self::Unknown),
            _ => Err(value),
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::EnteredInError => "entered-in-error",
            Self::Unknown => "unknown",
        }
    }
}

impl CodeEx for Type {
    fn from_parts(value: String) -> Result<Self, String> {
        match value.as_str() {
            "udi-label-name" => Ok(Self::UdiLabelName),
            "user-friendly-name" => Ok(Self::UserFriendlyName),
            "patient-reported-name" => Ok(Self::PatientReportedName),
            "manufacturer-name" => Ok(Self::ManufacturerName),
            "model-name" => Ok(Self::ModelName),
            "other" => Ok(Self::Other),
            _ => Err(value),
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::UdiLabelName => "udi-label-name",
            Self::UserFriendlyName => "user-friendly-name",
            Self::PatientReportedName => "patient-reported-name",
            Self::ManufacturerName => "manufacturer-name",
            Self::ModelName => "model-name",
            Self::Other => "other",
        }
    }
}

async fn decode_version<S>(stream: &mut DecodeStream<S>) -> Result<String, DecodeError<S::Error>>
where
    S: DataStream,
{
    let mut fields = Fields::new(&["value"]);

    stream.element().await?;

    let value = stream.decode(&mut fields, decode_any).await?;

    stream.end().await?;

    Ok(value)
}

#[allow(clippy::ptr_arg)]
fn encode_version<S>(
    value: &String,
    stream: &mut EncodeStream<S>,
) -> Result<(), EncodeError<S::Error>>
where
    S: DataStorage,
{
    stream
        .element()?
        .encode("value", value, encode_any)?
        .end()?;

    Ok(())
}

pub const PROFILE: &str = "https://gematik.de/fhir/StructureDefinition/ErxDevice";

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::convert::TryInto;
    use std::fs::read_to_string;
    use std::str::from_utf8;

    use crate::fhir::{
        decode::{tests::load_stream, JsonDecode, XmlDecode},
        encode::{JsonEncode, XmlEncode},
    };

    use super::super::super::tests::{trim_json_str, trim_xml_str};

    #[tokio::test]
    async fn test_decode_json() {
        let mut stream = load_stream("./examples/device.json");

        let actual: Device = stream.json().await.unwrap();
        let expected = test_device();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_decode_xml() {
        let mut stream = load_stream("./examples/device.xml");

        let actual: Device = stream.xml().await.unwrap();
        let expected = test_device();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_encode_json() {
        let value = test_device();

        let actual = (&value).json().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/device.json").unwrap();

        assert_eq!(trim_json_str(&actual), trim_json_str(&expected));
    }

    #[tokio::test]
    async fn test_encode_xml() {
        let value = test_device();

        let actual = (&value).xml().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/device.xml").unwrap();

        assert_eq!(trim_xml_str(&actual), trim_xml_str(&expected));
    }

    pub fn test_device() -> Device {
        Device {
            id: "ErxService".try_into().unwrap(),
            status: Status::Active,
            serial_number: Some("R4.0.0.287342834".into()),
            device_name: DeviceName {
                name: "E-Rezept Fachdienst".into(),
                type_: Type::UserFriendlyName,
            },
            version: "1.0.0".into(),
        }
    }
}
