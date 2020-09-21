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

mod activate_parameters;
mod create_parameters;

use std::borrow::Cow;
use std::convert::TryInto;

use resources::{
    misc::{Decode, DecodeStr, EncodeStr, Kvnr},
    primitives::{DateTime, Id},
    task::{Extension, Identifier, Input, Output, Status, Task},
    types::{DocumentType, PerformerType},
};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

use super::{
    super::super::constants::{
        CODING_SYSTEM_DOCUMENT_TYPE, CODING_SYSTEM_FLOW_TYPE, CODING_SYSTEM_PERFORMER_TYPE,
        EXTENSION_URL_ACCEPT_DATE, EXTENSION_URL_EXPIRY_DATE, EXTENSION_URL_PRESCRIPTION,
        IDENTIFIER_SYSTEM_ACCESS_CODE, IDENTIFIER_SYSTEM_PRESCRIPTION_ID, IDENTIFIER_SYSTEM_SECRET,
        IDENTITY_SYSTEM_KVID, RESOURCE_PROFILE_TASK, TASK_INTENT, XMLNS_TASK,
    },
    misc::{
        CodableConceptDef, CodingDef, DeserializeRoot, ExtensionDef, IdentifierDef, MetaDef,
        ReferenceDef, Root, SerializeRoot, ValueDef, XmlnsType,
    },
    primitives::{OptionDateTimeDef, OptionIdDef},
};

pub use activate_parameters::{TaskActivateParametersDef, TaskActivateParametersRoot};
pub use create_parameters::{TaskCreateParametersDef, TaskCreateParametersRoot};

pub struct TaskDef;
pub type TaskRoot<'a> = Root<TaskCow<'a>>;

#[serde(rename = "Task")]
#[derive(Clone, Serialize, Deserialize)]
pub struct TaskCow<'a>(#[serde(with = "TaskDef")] pub Cow<'a, Task>);

#[serde(rename = "Task")]
#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize)]
struct TaskHelper {
    #[serde(default)]
    #[serde(with = "OptionIdDef")]
    pub id: Option<Id>,

    #[serde(default)]
    pub meta: Option<MetaDef>,

    pub extension: ExtensionsDef,

    pub identifier: IdentifiersDef,

    #[serde(with = "StatusDef")]
    pub status: Status,

    #[serde(alias = "intent")]
    #[serde(rename = "value-tag=intent")]
    pub intent: String,

    #[serde(rename = "for")]
    pub for_: Option<ReferenceDef>,

    #[serde(with = "OptionDateTimeDef")]
    pub authored_on: Option<DateTime>,

    #[serde(with = "OptionDateTimeDef")]
    pub last_modified: Option<DateTime>,

    pub performer_type: Vec<CodableConceptDef>,

    pub input: InputDef,

    pub output: OutputDef,
}

#[derive(Serialize, Deserialize)]
struct ExtensionsDef(Vec<ExtensionDef>);

#[derive(Serialize, Deserialize)]
struct IdentifiersDef(Vec<IdentifierDef>);

#[derive(Serialize, Deserialize)]
struct InputDef(Vec<InputOutputDef>);

#[derive(Serialize, Deserialize)]
struct OutputDef(Vec<InputOutputDef>);

#[serde(rename_all = "camelCase")]
#[derive(Default, Serialize, Deserialize)]
pub struct InputOutputDef {
    #[serde(default)]
    #[serde(rename = "type")]
    pub type_: Option<CodableConceptDef>,

    #[serde(default)]
    pub value_reference: Option<ReferenceDef>,
}

impl XmlnsType for Task {
    fn xmlns() -> &'static str {
        XMLNS_TASK
    }
}

impl<'a> SerializeRoot<'a> for TaskCow<'a> {
    type Inner = Task;

    fn from_inner(inner: &'a Self::Inner) -> Self {
        TaskCow(Cow::Borrowed(inner))
    }
}

impl DeserializeRoot for TaskCow<'_> {
    type Inner = Task;

    fn into_inner(self) -> Self::Inner {
        self.0.into_owned()
    }
}

impl TaskDef {
    pub fn serialize<S: Serializer>(task: &Task, serializer: S) -> Result<S::Ok, S::Error> {
        let root: TaskHelper = task.into();

        root.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Cow<'static, Task>, D::Error> {
        let value = TaskHelper::deserialize(deserializer)?;

        Ok(Cow::Owned(value.try_into().map_err(D::Error::custom)?))
    }
}

impl Into<TaskHelper> for &Task {
    fn into(self) -> TaskHelper {
        TaskHelper {
            id: self.id.clone(),
            meta: Some(MetaDef {
                profile: vec![RESOURCE_PROFILE_TASK.into()],
                ..Default::default()
            }),
            extension: (&self.extension).into(),
            identifier: (&self.identifier).into(),
            status: self.status,
            intent: TASK_INTENT.into(),
            for_: self.for_.as_ref().map(|v| ReferenceDef {
                identifier: Some(IdentifierDef {
                    system: Some(IDENTITY_SYSTEM_KVID.into()),
                    value: Some(v.clone().into()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            authored_on: self.authored_on.clone(),
            last_modified: self.last_modified.clone(),
            performer_type: self.performer_type.iter().map(Into::into).collect(),
            input: (&self.input).into(),
            output: (&self.output).into(),
        }
    }
}

impl Into<ExtensionsDef> for &Extension {
    fn into(self) -> ExtensionsDef {
        let mut ret = vec![ExtensionDef {
            url: EXTENSION_URL_PRESCRIPTION.into(),
            value: Some(ValueDef::Coding(CodingDef {
                system: Some(CODING_SYSTEM_FLOW_TYPE.into()),
                code: Some(self.flow_type.encode_str()),
                display: Some(self.flow_type.to_string()),
                ..Default::default()
            })),
            ..Default::default()
        }];

        if let Some(accept_date) = &self.accept_date {
            ret.push(ExtensionDef {
                url: EXTENSION_URL_ACCEPT_DATE.into(),
                value: Some(ValueDef::DateTime(accept_date.clone())),
                ..Default::default()
            })
        }

        if let Some(expiry_date) = &self.expiry_date {
            ret.push(ExtensionDef {
                url: EXTENSION_URL_EXPIRY_DATE.into(),
                value: Some(ValueDef::DateTime(expiry_date.clone())),
                ..Default::default()
            });
        }

        ExtensionsDef(ret)
    }
}

impl Into<IdentifiersDef> for &Identifier {
    fn into(self) -> IdentifiersDef {
        let mut ret = Vec::new();

        if let Some(prescription_id) = &self.prescription_id {
            ret.push(IdentifierDef {
                system: Some(IDENTIFIER_SYSTEM_PRESCRIPTION_ID.into()),
                value: Some(prescription_id.to_string()),
                ..Default::default()
            })
        }

        if let Some(access_code) = &self.access_code {
            ret.push(IdentifierDef {
                system: Some(IDENTIFIER_SYSTEM_ACCESS_CODE.into()),
                value: Some(access_code.clone()),
                ..Default::default()
            })
        }

        if let Some(secret) = &self.secret {
            ret.push(IdentifierDef {
                system: Some(IDENTIFIER_SYSTEM_SECRET.into()),
                value: Some(secret.clone()),
                ..Default::default()
            })
        }

        IdentifiersDef(ret)
    }
}

impl Into<CodableConceptDef> for &PerformerType {
    fn into(self) -> CodableConceptDef {
        CodableConceptDef {
            coding: vec![CodingDef {
                system: Some(CODING_SYSTEM_PERFORMER_TYPE.into()),
                code: Some(self.encode_str()),
                display: Some(self.to_string()),
                ..Default::default()
            }],
            ..Default::default()
        }
    }
}

impl Into<InputDef> for &Input {
    fn into(self) -> InputDef {
        let mut ret = Vec::new();

        if let Some(e_prescription) = &self.e_prescription {
            ret.push(InputOutputDef {
                type_: Some(CodableConceptDef {
                    coding: vec![CodingDef {
                        system: Some(CODING_SYSTEM_DOCUMENT_TYPE.into()),
                        code: Some(DocumentType::EPrescription.encode_str()),
                        display: Some(DocumentType::EPrescription.to_string()),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                value_reference: Some(ReferenceDef {
                    reference: Some(e_prescription.clone()),
                    ..Default::default()
                }),
            })
        }

        if let Some(patient_receipt) = &self.patient_receipt {
            ret.push(InputOutputDef {
                type_: Some(CodableConceptDef {
                    coding: vec![CodingDef {
                        system: Some(CODING_SYSTEM_DOCUMENT_TYPE.into()),
                        code: Some(DocumentType::PatientReceipt.encode_str()),
                        display: Some(DocumentType::PatientReceipt.to_string()),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                value_reference: Some(ReferenceDef {
                    reference: Some(patient_receipt.clone()),
                    ..Default::default()
                }),
            })
        }

        InputDef(ret)
    }
}

impl Into<OutputDef> for &Output {
    fn into(self) -> OutputDef {
        let mut ret = Vec::new();

        if let Some(receipt) = &self.receipt {
            ret.push(InputOutputDef {
                type_: Some(CodableConceptDef {
                    coding: vec![CodingDef {
                        system: Some(CODING_SYSTEM_DOCUMENT_TYPE.into()),
                        code: Some(DocumentType::Receipt.encode_str()),
                        display: Some(DocumentType::Receipt.to_string()),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                value_reference: Some(ReferenceDef {
                    reference: Some(receipt.clone()),
                    ..Default::default()
                }),
            })
        }

        OutputDef(ret)
    }
}

impl TryInto<Task> for TaskHelper {
    type Error = String;

    fn try_into(self) -> Result<Task, String> {
        let meta = self
            .meta
            .ok_or_else(|| "Task is missing the `meta` field")?;

        if meta.profile != vec![RESOURCE_PROFILE_TASK.into()] {
            return Err("Task has an invalid profile".to_owned());
        }

        if self.intent != TASK_INTENT {
            return Err(format!(
                "Task `intent` has unexpected value (expected=`{}`, actual=`{}`)",
                TASK_INTENT, self.intent
            ));
        }

        Ok(Task {
            id: self.id,
            extension: self.extension.try_into()?,
            identifier: self.identifier.try_into()?,
            status: self.status,
            for_: self
                .for_
                .map(|for_| {
                    let identifier = for_
                        .identifier
                        .ok_or_else(|| "Task for is missing the `identifier` field!")?;

                    let system = identifier
                        .system
                        .ok_or_else(|| "Task for identifier is missing the `system` field!")?;

                    if system != IDENTITY_SYSTEM_KVID {
                        return Err("Task for identifier has invalid system!".to_owned());
                    }

                    let value = identifier
                        .value
                        .ok_or_else(|| "Task for identifier is missing the `value` field!")?;

                    Ok(Kvnr::new(value)?)
                })
                .transpose()?,
            authored_on: self.authored_on,
            last_modified: self.last_modified,
            performer_type: self
                .performer_type
                .into_iter()
                .filter_map(find_performer_type)
                .map(TryInto::try_into)
                .collect::<Result<Vec<_>, _>>()?,
            input: self.input.try_into()?,
            output: self.output.try_into()?,
        })
    }
}

impl TryInto<Extension> for ExtensionsDef {
    type Error = String;

    fn try_into(self) -> Result<Extension, Self::Error> {
        let mut accept_date = None;
        let mut expiry_date = None;
        let mut flow_type = None;

        for ext in self.0 {
            if ext.url == EXTENSION_URL_PRESCRIPTION {
                let coding = if let Some(ValueDef::Coding(coding)) = ext.value {
                    Ok(coding)
                } else {
                    Err("Extension is missing the `valueCoding` field!".to_owned())
                }?;

                let system = coding
                    .system
                    .ok_or_else(|| "Extension type is missing the `system` field!".to_owned())?;

                if system != CODING_SYSTEM_FLOW_TYPE {
                    return Err(format!(
                        "Extension contains invalid system type: {}!",
                        system
                    ));
                }

                match coding.code.as_deref().map(DecodeStr::decode_str) {
                    Some(Ok(value)) => flow_type = Some(value),
                    Some(Err(code)) => {
                        return Err(format!(
                            "Extension coding value contains invalid flow type: {}!",
                            code
                        ))
                    }
                    None => {
                        return Err("Extension coding value is missing the `code` field!".to_owned())
                    }
                }
            } else if ext.url == EXTENSION_URL_ACCEPT_DATE {
                accept_date = if let Some(ValueDef::DateTime(value)) = ext.value {
                    Ok(Some(value))
                } else {
                    Err("Extension is missing the `valueDateTime` field!".to_owned())
                }?;
            } else if ext.url == EXTENSION_URL_EXPIRY_DATE {
                expiry_date = if let Some(ValueDef::DateTime(value)) = ext.value {
                    Ok(Some(value))
                } else {
                    Err("Extension is missing the `valueDateTime` field!".to_owned())
                }?;
            }
        }

        Ok(Extension {
            accept_date,
            expiry_date,
            flow_type: flow_type.ok_or_else(|| "Extension is missing: flow_type!".to_owned())?,
        })
    }
}

impl TryInto<Identifier> for IdentifiersDef {
    type Error = String;

    fn try_into(self) -> Result<Identifier, Self::Error> {
        let mut prescription_id = None;
        let mut access_code = None;
        let mut secret = None;

        for ident in self.0 {
            match ident.system.as_deref() {
                Some(IDENTIFIER_SYSTEM_PRESCRIPTION_ID) => {
                    prescription_id = Some(
                        ident
                            .value
                            .ok_or_else(|| {
                                "Identifier `prescription_id` is missing the `value` field!"
                                    .to_owned()
                            })?
                            .parse()
                            .map_err(|err| format!("Invalid prescription ID: {}", err))?,
                    );
                }
                Some(IDENTIFIER_SYSTEM_ACCESS_CODE) => {
                    access_code = Some(ident.value.ok_or_else(|| {
                        "Identifier `access_code` is missing the `value` field!".to_owned()
                    })?);
                }
                Some(IDENTIFIER_SYSTEM_SECRET) => {
                    secret = Some(ident.value.ok_or_else(|| {
                        "Identifier `secret` is missing the `value` field!".to_owned()
                    })?);
                }
                _ => (),
            }
        }

        Ok(Identifier {
            prescription_id,
            access_code,
            secret,
        })
    }
}

impl TryInto<PerformerType> for CodingDef {
    type Error = String;

    fn try_into(self) -> Result<PerformerType, Self::Error> {
        match self.code.map(PerformerType::decode) {
            Some(Ok(performer_type)) => Ok(performer_type),
            Some(Err(code)) => Err(format!(
                "Performer type coding contains invalid code: {}!",
                code
            )),
            None => Err("Performer type coding is missing the `code` field!".to_owned()),
        }
    }
}

impl TryInto<Input> for InputDef {
    type Error = String;

    fn try_into(self) -> Result<Input, Self::Error> {
        let mut e_prescription = None;
        let mut patient_receipt = None;

        for input in self.0 {
            let coding = if let Some(coding) = input.type_.and_then(find_document_type) {
                coding
            } else {
                continue;
            };

            let document_type = coding
                .code
                .map(DocumentType::decode)
                .transpose()
                .map_err(|code| format!("Invalid document type: {}", code))?;

            match document_type {
                Some(DocumentType::EPrescription) => {
                    let value = input.value_reference.ok_or_else(|| {
                        "Input `e_prescription` is missing the `valueReference` field!".to_owned()
                    })?;
                    let value = value.reference.ok_or_else(|| {
                        "Input `e_prescription` value is missong the `reference` field!".to_owned()
                    })?;

                    e_prescription = Some(value);
                }
                Some(DocumentType::PatientReceipt) => {
                    let value = input.value_reference.ok_or_else(|| {
                        "Input `patient_receipt` is missing the `valueReference` field!".to_owned()
                    })?;
                    let value = value.reference.ok_or_else(|| {
                        "Input `patient_receipt` value is missong the `reference` field!".to_owned()
                    })?;

                    patient_receipt = Some(value);
                }
                Some(document_type) => {
                    return Err(format!("Unexpected document type: {:?}", document_type))
                }
                None => return Err("Input coding is missing the `code` field".to_owned()),
            }
        }

        Ok(Input {
            e_prescription,
            patient_receipt,
        })
    }
}

impl TryInto<Output> for OutputDef {
    type Error = String;

    fn try_into(self) -> Result<Output, Self::Error> {
        let mut receipt = None;

        for output in self.0 {
            let coding = if let Some(coding) = output.type_.and_then(find_document_type) {
                coding
            } else {
                continue;
            };

            let document_type = coding
                .code
                .map(DocumentType::decode)
                .transpose()
                .map_err(|code| format!("Invalid document type: {}", code))?;

            match document_type {
                Some(DocumentType::Receipt) => {
                    let value = output.value_reference.ok_or_else(|| {
                        "Output `receipt` is missing the `valueReference` field!".to_owned()
                    })?;
                    let value = value.reference.ok_or_else(|| {
                        "Output `receipt` value is missong the `reference` field!".to_owned()
                    })?;

                    receipt = Some(value);
                }
                Some(document_type) => {
                    return Err(format!("Unexpected document type: {:?}", document_type))
                }
                None => return Err("Output coding is missing the `code` field".to_owned()),
            }
        }

        Ok(Output { receipt })
    }
}

fn find_performer_type(c: CodableConceptDef) -> Option<CodingDef> {
    for coding in c.coding {
        if let Some(CODING_SYSTEM_PERFORMER_TYPE) = coding.system.as_deref() {
            return Some(coding);
        }
    }

    None
}

fn find_document_type(c: CodableConceptDef) -> Option<CodingDef> {
    for coding in c.coding {
        if let Some(CODING_SYSTEM_DOCUMENT_TYPE) = coding.system.as_deref() {
            return Some(coding);
        }
    }

    None
}

#[serde(remote = "Status")]
#[serde(rename_all = "kebab-case")]
#[serde(rename = "value-tag=Status")]
#[derive(Serialize, Deserialize)]
pub enum StatusDef {
    Draft,
    Requested,
    Received,
    Accepted,
    Rejected,
    Ready,
    Cancelled,
    InProgress,
    OnHold,
    Failed,
    Completed,
    EnteredInError,
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::convert::TryInto;
    use std::fs::read_to_string;

    use resources::{
        task::{Extension, Identifier, Input, Output, Status, Task},
        types::{FlowType, PerformerType},
    };

    use crate::fhir::{
        test::trim_xml_str,
        xml::{from_str as from_xml, to_string as to_xml},
    };

    #[test]
    fn convert_to() {
        let task = test_task();

        let actual = trim_xml_str(&to_xml(&TaskRoot::new(&task)).unwrap());
        let expected = trim_xml_str(&read_to_string("./examples/task.xml").unwrap());

        assert_eq!(actual, expected);
    }

    #[test]
    fn convert_from() {
        let actual = from_xml::<TaskRoot>(&read_to_string("./examples/task.xml").unwrap())
            .unwrap()
            .into_inner();
        let expected = test_task();

        assert_eq!(actual, expected);
    }

    fn test_task() -> Task {
        Task {
            id: None,
            extension: Extension {
                accept_date: Some("2020-03-02T08:25:05+00:00".try_into().unwrap()),
                expiry_date: Some("2020-05-02T08:25:05+00:00".try_into().unwrap()),
                flow_type: FlowType::PharmaceuticalDrugs,
            },
            identifier: Identifier {
                prescription_id: Some("160.123.456.789.123.58".parse().unwrap()),
                access_code: Some(
                    "777bea0e13cc9c42ceec14aec3ddee2263325dc2c6c699db115f58fe423607ea".into(),
                ),
                secret: Some(
                    "c36ca26502892b371d252c99b496e31505ff449aca9bc69e231c58148f6233cf".into(),
                ),
            },
            status: Status::InProgress,
            for_: Some(Kvnr::new("X123456789").unwrap()),
            authored_on: Some("2020-03-02T08:25:05+00:00".try_into().unwrap()),
            last_modified: Some("2020-03-02T08:45:05+00:00".try_into().unwrap()),
            performer_type: vec![PerformerType::PublicPharmacy],
            input: Input {
                e_prescription: Some("Bundle/KbvPrescriptionExample".into()),
                patient_receipt: Some("Bundle/KbvPatientReceiptExample".into()),
            },
            output: Output {
                receipt: Some("Bundle/KbvReceiptExample".into()),
            },
        }
    }
}
