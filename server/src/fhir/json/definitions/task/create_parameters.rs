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
use std::convert::TryInto;

use resources::{
    misc::{DecodeStr, EncodeStr},
    task::TaskCreateParameters,
};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

use super::super::{
    super::super::constants::{
        CODING_SYSTEM_FLOW_TYPE, PARAMETER_TYPE_TASK_CREATE, RESOURCE_TYPE_PARAMETERS,
    },
    misc::{
        CodingDef, DeserializeRoot, ParameterDef, ParametersDef, ResourceType, Root, SerializeRoot,
        ValueDef,
    },
};

pub struct TaskCreateParametersDef;
pub type TaskCreateParametersRoot<'a> = Root<TaskCreateParametersCow<'a>>;

#[derive(Serialize, Deserialize)]
pub struct TaskCreateParametersCow<'a>(
    #[serde(with = "TaskCreateParametersDef")] Cow<'a, TaskCreateParameters>,
);

impl ResourceType for TaskCreateParameters {
    fn resource_type() -> &'static str {
        RESOURCE_TYPE_PARAMETERS
    }
}

impl<'a> SerializeRoot<'a> for TaskCreateParametersCow<'a> {
    type Inner = TaskCreateParameters;

    fn from_inner(inner: &'a Self::Inner) -> Self {
        TaskCreateParametersCow(Cow::Borrowed(inner))
    }
}

impl DeserializeRoot for TaskCreateParametersCow<'_> {
    type Inner = TaskCreateParameters;

    fn into_inner(self) -> Self::Inner {
        self.0.into_owned()
    }
}

impl TaskCreateParametersDef {
    fn serialize<S: Serializer>(
        parameters: &TaskCreateParameters,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let value: ParametersDef<()> = parameters.into();

        value.serialize(serializer)
    }

    fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Cow<'static, TaskCreateParameters>, D::Error> {
        let value = ParametersDef::deserialize(deserializer)?;

        Ok(Cow::Owned(value.try_into().map_err(D::Error::custom)?))
    }
}

impl Into<ParametersDef<()>> for &TaskCreateParameters {
    fn into(self) -> ParametersDef<()> {
        ParametersDef {
            parameter: vec![ParameterDef {
                name: PARAMETER_TYPE_TASK_CREATE.into(),
                value: Some(ValueDef::Coding(CodingDef {
                    system: Some(CODING_SYSTEM_FLOW_TYPE.into()),
                    code: Some(self.flow_type.encode_str()),
                    ..Default::default()
                })),
                ..Default::default()
            }],
        }
    }
}

impl TryInto<TaskCreateParameters> for ParametersDef<()> {
    type Error = String;

    fn try_into(self) -> Result<TaskCreateParameters, Self::Error> {
        let parameter = self
            .parameter
            .get(0)
            .ok_or_else(|| "Parameters is empty!")?;

        if parameter.name != PARAMETER_TYPE_TASK_CREATE {
            return Err(format!("Parameter has unexpected name: {}", parameter.name));
        }

        let coding = if let Some(ValueDef::Coding(coding)) = &parameter.value {
            Ok(coding)
        } else {
            Err("Parameter is missing the `valueCoding` field!".to_owned())
        }?;

        match coding.system.as_deref() {
            Some(CODING_SYSTEM_FLOW_TYPE) => Ok(()),
            Some(s) => Err(format!("Coding has invalid system: {}", s)),
            None => Err("Coding is missing the `system` field".to_owned()),
        }?;

        let flow_type = match coding.code.as_deref().map(DecodeStr::decode_str) {
            Some(Ok(flow_type)) => Ok(flow_type),
            Some(Err(code)) => Err(format!("Coding has invalid code: {}", code)),
            None => Err("Coding is missing the `code` field".to_owned()),
        }?;

        Ok(TaskCreateParameters { flow_type })
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::fs::read_to_string;

    use resources::types::FlowType;

    use crate::fhir::{
        json::{from_str as from_json, to_string as to_json},
        test::trim_xml_str,
    };

    #[test]
    fn convert_to() {
        let task = test_parameters();

        let actual = trim_xml_str(&to_json(&TaskCreateParametersRoot::new(&task)).unwrap());
        let expected =
            trim_xml_str(&read_to_string("./examples/task_create_parameters.json").unwrap());

        assert_eq!(actual, expected);
    }

    #[test]
    fn convert_from() {
        let actual = from_json::<TaskCreateParametersRoot>(
            &read_to_string("./examples/task_create_parameters.json").unwrap(),
        )
        .unwrap()
        .into_inner();
        let expected = test_parameters();

        assert_eq!(actual, expected);
    }

    fn test_parameters() -> TaskCreateParameters {
        TaskCreateParameters {
            flow_type: FlowType::PharmaceuticalDrugs,
        }
    }
}
