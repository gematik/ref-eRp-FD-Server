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
    medication_request::{
        AccidentCause, AccidentInformation, CoPayment, DispenseRequest, Dosage, Extension,
    },
    misc::{DecodeStr, EncodeStr},
    primitives::{DateTime, Id},
    MedicationRequest,
};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

use super::{
    super::super::constants::{
        CODING_SYSTEM_ACCIDENT_CAUSE, CODING_SYSTEM_CO_PAYMENT, EXTENSION_URL_ACCIDENT,
        EXTENSION_URL_ACCIDENT_BUSINESS, EXTENSION_URL_ACCIDENT_CAUSE, EXTENSION_URL_ACCIDENT_DATE,
        EXTENSION_URL_BGV, EXTENSION_URL_CO_PAYMENT, EXTENSION_URL_DOSAGE_FLAG,
        EXTENSION_URL_EMERGENCY_SERVICE_FEE, MEDICATION_REQUEST_INTENT,
        MEDICATION_REQUEST_QUANTITY_CODE, MEDICATION_REQUEST_STATUS, QUANTITY_SYSTEM_MEDICATION,
        RESOURCE_PROFILE_MEDICATION_REQUEST, XMLNS_MEDICATION_REQUEST,
    },
    misc::{
        CodingDef, DeserializeRoot, ExtensionDef, MetaDef, QuantityDef, ReferenceDef,
        SerializeRoot, ValueDef, XmlnsType,
    },
    primitives::{DateTimeDef, IdDef, OptionDateTimeDef},
};

pub struct MedicationRequestDef;

#[derive(Serialize, Deserialize)]
#[serde(rename = "MedicationRequest")]
pub struct MedicationRequestCow<'a>(
    #[serde(with = "MedicationRequestDef")] Cow<'a, MedicationRequest>,
);

#[serde(rename = "MedicationRequest")]
#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize)]
struct MedicationRequestHelper {
    #[serde(with = "IdDef")]
    id: Id,

    meta: MetaDef,

    extension: ExtensionsDef,

    #[serde(alias = "status")]
    #[serde(rename = "value-tag=status")]
    status: String,

    #[serde(alias = "intent")]
    #[serde(rename = "value-tag=intent")]
    intent: String,

    medication_reference: ReferenceDef,

    subject: ReferenceDef,

    #[serde(with = "DateTimeDef")]
    authored_on: DateTime,

    requester: ReferenceDef,

    insurance: Vec<ReferenceDef>,

    #[serde(default)]
    note: Option<String>,

    #[serde(default)]
    dosage_instruction: Vec<DosageInstructionDef>,

    dispense_request: DispenseRequestDef,

    substitution: SubstitutionDef,
}

#[derive(Serialize, Deserialize)]
struct ExtensionsDef(Vec<ExtensionDef>);

#[serde(rename = "DosageInstruction")]
#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize)]
struct DosageInstructionDef {
    extension: Vec<ExtensionDef>,

    #[serde(alias = "text")]
    #[serde(rename = "value-tag=text")]
    text: Option<String>,

    patient_instruction: Option<String>,
}

#[serde(rename = "DosageInstruction")]
#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize)]
struct DispenseRequestDef {
    validity_period: Option<ValidityPeriodDef>,
    quantity: QuantityDef,
}

#[serde(rename = "ValidityPeriod")]
#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize)]
struct ValidityPeriodDef {
    #[serde(with = "DateTimeDef")]
    start: DateTime,

    #[serde(with = "OptionDateTimeDef")]
    end: Option<DateTime>,
}

#[serde(rename = "ValidityPeriod")]
#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize)]
struct SubstitutionDef {
    #[serde(alias = "allowedBoolean")]
    #[serde(rename = "value-tag=allowedBoolean")]
    allowed_boolean: bool,
}

impl XmlnsType for MedicationRequest {
    fn xmlns() -> &'static str {
        XMLNS_MEDICATION_REQUEST
    }
}

impl<'a> SerializeRoot<'a> for MedicationRequestCow<'a> {
    type Inner = MedicationRequest;

    fn from_inner(inner: &'a Self::Inner) -> Self {
        MedicationRequestCow(Cow::Borrowed(inner))
    }
}

impl DeserializeRoot for MedicationRequestCow<'_> {
    type Inner = MedicationRequest;

    fn into_inner(self) -> Self::Inner {
        self.0.into_owned()
    }
}

impl MedicationRequestDef {
    pub fn serialize<S: Serializer>(
        medication_request: &MedicationRequest,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let value: MedicationRequestHelper = medication_request.into();

        value.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Cow<'static, MedicationRequest>, D::Error> {
        let value = MedicationRequestHelper::deserialize(deserializer)?;

        Ok(Cow::Owned(value.try_into().map_err(D::Error::custom)?))
    }
}

impl Into<MedicationRequestHelper> for &MedicationRequest {
    fn into(self) -> MedicationRequestHelper {
        MedicationRequestHelper {
            id: self.id.clone(),
            meta: MetaDef {
                profile: vec![RESOURCE_PROFILE_MEDICATION_REQUEST.into()],
                ..Default::default()
            },
            extension: (&self.extension).into(),
            status: MEDICATION_REQUEST_STATUS.into(),
            intent: MEDICATION_REQUEST_INTENT.into(),
            medication_reference: ReferenceDef {
                reference: Some(self.medication.clone()),
                ..Default::default()
            },
            subject: ReferenceDef {
                reference: Some(self.subject.clone()),
                ..Default::default()
            },
            authored_on: self.authored_on.clone(),
            requester: ReferenceDef {
                reference: Some(self.requester.clone()),
                ..Default::default()
            },
            insurance: vec![ReferenceDef {
                reference: Some(self.insurance.clone()),
                ..Default::default()
            }],
            note: self.note.clone(),
            dosage_instruction: self.dosage.iter().map(Into::into).collect(),
            dispense_request: (&self.dispense_request).into(),
            substitution: SubstitutionDef {
                allowed_boolean: self.substitution_allowed,
            },
        }
    }
}

impl Into<ExtensionsDef> for &Extension {
    fn into(self) -> ExtensionsDef {
        let mut ret = vec![
            ExtensionDef {
                url: EXTENSION_URL_EMERGENCY_SERVICE_FEE.into(),
                value: Some(ValueDef::Boolean(self.emergency_service_fee.into())),
                ..Default::default()
            },
            ExtensionDef {
                url: EXTENSION_URL_BGV.into(),
                value: Some(ValueDef::Boolean(self.bvg.into())),
                ..Default::default()
            },
        ];

        if let Some(co_payment) = &self.co_payment {
            ret.push(ExtensionDef {
                url: EXTENSION_URL_CO_PAYMENT.into(),
                value: Some(ValueDef::Coding(CodingDef {
                    system: Some(CODING_SYSTEM_CO_PAYMENT.into()),
                    code: Some(co_payment.encode_str()),
                    ..Default::default()
                })),
                ..Default::default()
            })
        }

        if let Some(accident_information) = &self.accident_information {
            let mut extension = vec![
                ExtensionDef {
                    url: EXTENSION_URL_ACCIDENT_CAUSE.into(),
                    value: Some(ValueDef::Coding(CodingDef {
                        system: Some(CODING_SYSTEM_ACCIDENT_CAUSE.into()),
                        code: Some(accident_information.cause.encode_str()),
                        ..Default::default()
                    })),
                    ..Default::default()
                },
                ExtensionDef {
                    url: EXTENSION_URL_ACCIDENT_DATE.into(),
                    value: Some(ValueDef::Date(accident_information.date.clone())),
                    ..Default::default()
                },
            ];

            if let Some(business) = &accident_information.business {
                extension.push(ExtensionDef {
                    url: EXTENSION_URL_ACCIDENT_BUSINESS.into(),
                    value: Some(ValueDef::String(business.clone().into())),
                    ..Default::default()
                });
            }

            ret.push(ExtensionDef {
                url: EXTENSION_URL_ACCIDENT.into(),
                extension,
                ..Default::default()
            })
        }

        ExtensionsDef(ret)
    }
}

impl Into<DosageInstructionDef> for &Dosage {
    fn into(self) -> DosageInstructionDef {
        DosageInstructionDef {
            extension: self
                .dosage_mark
                .into_iter()
                .map(|flag| ExtensionDef {
                    url: EXTENSION_URL_DOSAGE_FLAG.into(),
                    value: Some(ValueDef::Boolean(flag.into())),
                    ..Default::default()
                })
                .collect(),
            text: self.text.clone(),
            patient_instruction: self.patient_instruction.clone(),
        }
    }
}

impl Into<DispenseRequestDef> for &DispenseRequest {
    fn into(self) -> DispenseRequestDef {
        DispenseRequestDef {
            validity_period: self
                .validity_period_start
                .as_ref()
                .map(|start| ValidityPeriodDef {
                    start: start.clone(),
                    end: self.validity_period_end.clone(),
                }),
            quantity: QuantityDef {
                value: Some(self.quantity),
                system: Some(QUANTITY_SYSTEM_MEDICATION.into()),
                code: Some(MEDICATION_REQUEST_QUANTITY_CODE.into()),
                ..Default::default()
            },
        }
    }
}

impl TryInto<MedicationRequest> for MedicationRequestHelper {
    type Error = String;

    fn try_into(self) -> Result<MedicationRequest, Self::Error> {
        if self.status != MEDICATION_REQUEST_STATUS {
            return Err("Medication request has unexpected status!".to_owned());
        }

        if self.intent != MEDICATION_REQUEST_INTENT {
            return Err("Medication request has unexpected intent!".to_owned());
        }

        Ok(MedicationRequest {
            id: self.id,
            extension: self.extension.try_into()?,
            medication: self
                .medication_reference
                .reference
                .ok_or_else(|| "Mediaction request reference is missing the `reference` field!")?,
            subject: self
                .subject
                .reference
                .ok_or_else(|| "Medication request subject is missing the `reference` field!")?,
            authored_on: self.authored_on,
            requester: self
                .requester
                .reference
                .ok_or_else(|| "Medication request requester is missing the `reference` field!")?,
            insurance: self
                .insurance
                .into_iter()
                .next()
                .and_then(|i| i.reference)
                .ok_or_else(|| "Medication request is missing the `insurance` field!")?,
            note: self.note,
            dosage: self
                .dosage_instruction
                .into_iter()
                .next()
                .map(TryInto::try_into)
                .transpose()?,
            dispense_request: self.dispense_request.try_into()?,
            substitution_allowed: self.substitution.allowed_boolean,
        })
    }
}

impl TryInto<Extension> for ExtensionsDef {
    type Error = String;

    fn try_into(self) -> Result<Extension, Self::Error> {
        let mut emergency_service_fee = None;
        let mut bvg = None;
        let mut co_payment = None;
        let mut accident_information = None;

        for ex in self.0 {
            if ex.url == EXTENSION_URL_EMERGENCY_SERVICE_FEE {
                match ex.value {
                    Some(ValueDef::Boolean(value)) => emergency_service_fee = Some(value.into()),
                    _ => {
                        return Err(
                            "Extension emergency service fee is missing the `valueBoolean` field!"
                                .to_owned(),
                        )
                    }
                }
            } else if ex.url == EXTENSION_URL_BGV {
                match ex.value {
                    Some(ValueDef::Boolean(value)) => bvg = Some(value.into()),
                    _ => {
                        return Err("Extension BGV is missing the `valueBoolean` field!".to_owned())
                    }
                }
            } else if ex.url == EXTENSION_URL_CO_PAYMENT {
                let coding = match ex.value {
                    Some(ValueDef::Coding(coding)) => coding,
                    _ => {
                        return Err(
                            "Extension co payment is missing the `valueCoding` field!".to_owned()
                        )
                    }
                };

                match coding.system.as_deref() {
                    Some(CODING_SYSTEM_CO_PAYMENT) => (),
                    Some(system) => {
                        return Err(format!(
                            "Extension co payment coding has invalid system: {}!",
                            system
                        ))
                    }
                    None => {
                        return Err(
                            "Extension co payment coding is missing the `system`field!".to_owned()
                        )
                    }
                }

                match coding.code.as_deref().map(CoPayment::decode_str) {
                    Some(Ok(value)) => co_payment = Some(value),
                    Some(Err(err)) => {
                        return Err(format!("Extension co payment has invalid code: {}", err))
                    }
                    None => {
                        return Err("Extension co payment is missing the `code` field!".to_owned())
                    }
                }
            } else if ex.url == EXTENSION_URL_ACCIDENT {
                let mut cause = None;
                let mut date = None;
                let mut business = None;

                for ex in ex.extension {
                    if ex.url == EXTENSION_URL_ACCIDENT_CAUSE {
                        let coding =
                            match ex.value {
                                Some(ValueDef::Coding(coding)) => coding,
                                _ => return Err(
                                    "Extension accident cause is missing the `valueCoding` field!"
                                        .to_owned(),
                                ),
                            };

                        match coding.system.as_deref() {
                            Some(CODING_SYSTEM_ACCIDENT_CAUSE) => (),
                            Some(system) => {
                                return Err(format!(
                                    "Extension accident cause coding has unexpected system: {}!",
                                    system
                                ))
                            }
                            None => return Err(
                                "Extension accident cause coding is missing the `system` field!"
                                    .to_owned(),
                            ),
                        }

                        match coding.code.as_deref().map(AccidentCause::decode_str) {
                            Some(Ok(value)) => cause = Some(value),
                            Some(Err(err)) => {
                                return Err(format!(
                                    "Extension accident cause has invalid code: {}",
                                    err
                                ))
                            }
                            None => {
                                return Err(
                                    "Extension accident cause  is missing the `code` field!"
                                        .to_owned(),
                                )
                            }
                        }
                    } else if ex.url == EXTENSION_URL_ACCIDENT_DATE {
                        match ex.value {
                            Some(ValueDef::Date(value)) => date = Some(value),
                            _ => {
                                return Err(
                                    "Extension accident date is missing the `valueDate` field!"
                                        .to_owned(),
                                )
                            }
                        }
                    } else if ex.url == EXTENSION_URL_ACCIDENT_BUSINESS {
                        match ex.value {
                            Some(ValueDef::String(value)) => business = Some(value.into()),
                            _ => return Err(
                                "Extension accident business is missing the `valueString` field!"
                                    .to_owned(),
                            ),
                        }
                    }
                }

                accident_information = Some(AccidentInformation {
                    cause: cause
                        .ok_or_else(|| "Accident information is missing the cause extension!")?,
                    date: date
                        .ok_or_else(|| "Accident information is missing the date extension!")?,
                    business,
                })
            }
        }

        Ok(Extension {
            emergency_service_fee: emergency_service_fee.ok_or_else(|| {
                "Medication request is missing the extension emergency service fee!"
            })?,
            bvg: bvg.ok_or_else(|| "Medication request is missing the extension BVG!")?,
            co_payment,
            accident_information,
        })
    }
}

impl TryInto<Dosage> for DosageInstructionDef {
    type Error = String;

    fn try_into(self) -> Result<Dosage, Self::Error> {
        Ok(Dosage {
            dosage_mark: self
                .extension
                .into_iter()
                .filter_map(|ex| {
                    if ex.url == EXTENSION_URL_DOSAGE_FLAG {
                        if let Some(ValueDef::Boolean(flag)) = ex.value {
                            Some(Ok(flag.into()))
                        } else {
                            Some(Err(
                                "Extension dosage flag is missing the `valueBoolean` field!"
                                    .to_owned(),
                            ))
                        }
                    } else {
                        None
                    }
                })
                .next()
                .transpose()?,
            text: self.text,
            patient_instruction: self.patient_instruction,
        })
    }
}

impl TryInto<DispenseRequest> for DispenseRequestDef {
    type Error = String;

    fn try_into(self) -> Result<DispenseRequest, Self::Error> {
        let mut validity_period_start = None;
        let mut validity_period_end = None;

        match self.quantity.system.as_deref() {
            Some(QUANTITY_SYSTEM_MEDICATION) => (),
            Some(system) => {
                return Err(format!(
                    "Dispense request quantity has invalid system: {}!",
                    system
                ))
            }
            None => {
                return Err("Dispense request quantity is missing the `system` field!".to_owned())
            }
        }

        match self.quantity.code.as_deref() {
            Some(MEDICATION_REQUEST_QUANTITY_CODE) => (),
            Some(code) => {
                return Err(format!(
                    "Dispense request quantity has invalid code: {}!",
                    code
                ))
            }
            None => return Err("Dispense request quantity is missing the `code` field!".to_owned()),
        }

        if let Some(validity_period) = self.validity_period {
            validity_period_start = Some(validity_period.start);
            validity_period_end = validity_period.end;
        }

        Ok(DispenseRequest {
            quantity: self
                .quantity
                .value
                .ok_or_else(|| "Dispense request quantity is missing the `value` field!")?,
            validity_period_start,
            validity_period_end,
        })
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::convert::TryInto;
    use std::fs::read_to_string;

    use resources::medication_request::{
        AccidentCause, AccidentInformation, CoPayment, DispenseRequest, Dosage, Extension,
    };

    use crate::fhir::{
        test::trim_xml_str,
        xml::{from_str as from_xml, to_string as to_xml},
    };

    use super::super::misc::Root;

    type MedicationRequestRoot<'a> = Root<MedicationRequestCow<'a>>;

    #[test]
    fn convert_to() {
        let bundle = test_medication_request();

        let actual = trim_xml_str(&to_xml(&MedicationRequestRoot::new(&bundle)).unwrap());
        let expected = trim_xml_str(&read_to_string("./examples/medication_request.xml").unwrap());

        assert_eq!(actual, expected);
    }

    #[test]
    fn convert_from() {
        let xml = read_to_string("./examples/medication_request.xml").unwrap();
        let actual = from_xml::<MedicationRequestRoot>(&xml)
            .unwrap()
            .into_inner();
        let expected = test_medication_request();

        assert_eq!(actual, expected);
    }

    pub fn test_medication_request() -> MedicationRequest {
        MedicationRequest {
            id: "e930cdee-9eb5-4b44-88b5-2a18b69f3b9a".try_into().unwrap(),
            extension: Extension {
                emergency_service_fee: false,
                bvg: false,
                co_payment: Some(CoPayment::NotExceptFrom),
                accident_information: Some(AccidentInformation {
                    cause: AccidentCause::WorkAccident,
                    date: "2020-05-01".try_into().unwrap(),
                    business: Some("Dummy-Betrieb".into()),
                }),
            },
            medication: "Medication/5fe6e06c-8725-46d5-aecd-e65e041ca3de".into(),
            subject: "Patient/9774f67f-a238-4daf-b4e6-679deeef3811".into(),
            authored_on: "2020-02-03T00:00:00+00:00".try_into().unwrap(),
            requester: "Practioner/20597e0e-cb2a-45b3-95f0-dc3dbdb617c3".into(),
            insurance: "Coverage/1b1ffb6e-eb05-43d7-87eb-e7818fe9661a".into(),
            note: None,
            dosage: Some(Dosage {
                dosage_mark: Some(true),
                text: Some("1-0-1-0".into()),
                patient_instruction: None,
            }),
            dispense_request: DispenseRequest {
                quantity: 1,
                validity_period_start: None,
                validity_period_end: None,
            },
            substitution_allowed: true,
        }
    }
}
