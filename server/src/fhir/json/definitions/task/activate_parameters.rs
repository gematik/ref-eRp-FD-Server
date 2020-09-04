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

use std::borrow::Cow;
use std::str::from_utf8;

use resources::task::TaskActivateParameters;
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    asn1::{definitions::SignedDataCow, from_bytes},
    fhir::xml::{definitions::KbvBundleRoot, from_str as from_xml},
};

use super::super::{
    super::super::constants::{
        BINARY_CONTENT_TYPE_PKCS7, PARAMETER_TYPE_TASK_ACTIVATE, RESOURCE_TYPE_PARAMETERS,
    },
    misc::{BinaryDef, DeserializeRoot, ParametersDef, ResourceType, Root, SerializeRoot},
};

pub struct TaskActivateParametersDef;
pub type TaskActivateParametersRoot<'a> = Root<TaskActivateParametersCow<'a>>;

#[serde(rename = "Parameters")]
#[derive(Serialize, Deserialize)]
pub struct TaskActivateParametersCow<'a>(
    #[serde(with = "TaskActivateParametersDef")] Cow<'a, TaskActivateParameters>,
);

#[serde(tag = "resourceType")]
#[derive(Serialize, Deserialize)]
enum Resource {
    Binary(BinaryDef),
    Unknown,
}

impl Default for Resource {
    fn default() -> Self {
        Self::Unknown
    }
}

impl ResourceType for TaskActivateParameters {
    fn resource_type() -> &'static str {
        RESOURCE_TYPE_PARAMETERS
    }
}

impl<'a> SerializeRoot<'a> for TaskActivateParametersCow<'a> {
    type Inner = TaskActivateParameters;

    fn from_inner(inner: &'a Self::Inner) -> Self {
        TaskActivateParametersCow(Cow::Borrowed(inner))
    }
}

impl DeserializeRoot for TaskActivateParametersCow<'_> {
    type Inner = TaskActivateParameters;

    fn into_inner(self) -> Self::Inner {
        self.0.into_owned()
    }
}

impl TaskActivateParametersDef {
    fn serialize<S: Serializer>(
        _parameters: &TaskActivateParameters,
        _serializer: S,
    ) -> Result<S::Ok, S::Error> {
        unimplemented!()
    }

    fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Cow<'static, TaskActivateParameters>, D::Error> {
        let root = ParametersDef::<Resource>::deserialize(deserializer)?;

        let parameter = root
            .parameter
            .iter()
            .next()
            .ok_or_else(|| D::Error::custom("Parameters is empty!"))?;

        if parameter.name != PARAMETER_TYPE_TASK_ACTIVATE {
            return Err(D::Error::custom(format!(
                "Parameter has unexpected name: {}",
                parameter.name
            )));
        }

        let resource = parameter
            .resource
            .as_ref()
            .ok_or_else(|| D::Error::custom("Parameter is missing the `resource` field!"))?;

        let binary = if let Resource::Binary(binary) = resource {
            binary
        } else {
            return Err(D::Error::custom("Parameter contains invalid resource!"));
        };

        if binary.content_type != BINARY_CONTENT_TYPE_PKCS7 {
            return Err(D::Error::custom(
                "Parameter binary resource has invalid content type!",
            ));
        }

        let data = binary.data.as_ref().ok_or_else(|| {
            D::Error::custom("Parameter binary resource is missing the `data` field!")
        })?;

        let signed_data = from_bytes::<SignedDataCow<'static>>(&data)
            .map_err(|err| D::Error::custom(format!("Error while parsing PKCS#7 file: {}", err)))?
            .into_inner();

        let data = from_utf8(&signed_data.content).map_err(|_| {
            D::Error::custom("Content of CMS container is not a valid UTF-8 string!")
        })?;

        let kbv_bundle = from_xml::<KbvBundleRoot>(data)
            .map_err(|err| {
                D::Error::custom(format!(
                    "Content of CMS container is not a valid KBV bundle: {}",
                    err
                ))
            })?
            .into_inner();

        Ok(Cow::Owned(TaskActivateParameters { kbv_bundle }))
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::fs::read_to_string;

    use crate::fhir::json::from_str as from_json;

    use super::super::super::kbv_bundle::tests::test_kbv_bundle;

    #[test]
    fn convert_from() {
        let data = read_to_string("./examples/task_activate_parameters.json").unwrap();
        let actual = from_json::<TaskActivateParametersRoot>(&data)
            .unwrap()
            .into_inner();
        let expected = test_parameters();

        assert_eq!(actual, expected);
    }

    fn test_parameters() -> TaskActivateParameters {
        TaskActivateParameters {
            kbv_bundle: test_kbv_bundle(),
        }
    }
}
