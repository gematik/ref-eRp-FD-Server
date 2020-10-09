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
    communication::{
        Availability, Communication, DispenseReqExtensions, InfoReqExtensions, Inner, Payload,
        ReplyExtensions, RepresentativeExtensions, SupplyOptions,
    },
    misc::{DecodeStr, EncodeStr, Kvnr, TelematikId},
    primitives::{DateTime, Id},
    types::FlowType,
    Medication,
};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

use super::{
    super::super::constants::{
        CODING_SYSTEM_AVAILABILITY_STATUS, CODING_SYSTEM_FLOW_TYPE,
        EXTENSION_URL_AVAILABILITY_STATUS, EXTENSION_URL_INSURANCE_PROVIDER,
        EXTENSION_URL_PRESCRIPTION, EXTENSION_URL_SUBSTITUTION_ALLOWED,
        EXTENSION_URL_SUPPLY_OPTIONS, IDENTITY_SYSTEM_IKNR, IDENTITY_SYSTEM_KVID,
        IDENTITY_SYSTEM_TELEMATIK_ID, RESOURCE_PROFILE_COMMUNICATION_DISPENSE_REQ,
        RESOURCE_PROFILE_COMMUNICATION_INFO_REQ, RESOURCE_PROFILE_COMMUNICATION_REPLY,
        RESOURCE_PROFILE_COMMUNICATION_REPRESENTATIVE, RESOURCE_TYPE_COMMUNICATION,
    },
    misc::{
        CodingDef, DeserializeRoot, ExtensionDef, IdentifierDef, MetaDef, ReferenceDef,
        ResourceType, Root, SerializeRoot, ValueDef,
    },
    primitives::{OptionDateTimeDef, OptionIdDef},
    MedicationDef,
};

pub struct CommunicationDef;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename = "Communication")]
pub struct CommunicationCow<'a>(#[serde(with = "CommunicationDef")] pub Cow<'a, Communication>);

pub type CommunicationRoot<'a> = Root<CommunicationCow<'a>>;

#[serde(rename = "Communication")]
#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize)]
struct CommunicationHelper<'a> {
    #[serde(default)]
    #[serde(with = "OptionIdDef")]
    id: Option<Id>,

    meta: MetaDef,

    #[serde(default)]
    based_on: Vec<ReferenceDef>,

    #[serde(default)]
    contained: Vec<ResourceDef<'a>>,

    status: String,

    #[serde(default)]
    about: Vec<ReferenceDef>,

    #[serde(default)]
    #[serde(with = "OptionDateTimeDef")]
    sent: Option<DateTime>,

    #[serde(default)]
    #[serde(with = "OptionDateTimeDef")]
    received: Option<DateTime>,

    recipient: Vec<ReferenceDef>,

    sender: Option<ReferenceDef>,

    payload: Vec<PayloadDef>,
}

#[serde(rename = "Payload")]
#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize)]
struct PayloadDef {
    #[serde(default)]
    extension: PayloadExtensionsDef,

    #[serde(rename = "contentString")]
    content: String,
}

#[derive(Default, Serialize, Deserialize)]
struct PayloadExtensionsDef(Vec<ExtensionDef>);

#[serde(tag = "resourceType")]
#[derive(Clone, Serialize, Deserialize)]
enum ResourceDef<'a> {
    Medication(#[serde(with = "MedicationDef")] Cow<'a, Medication>),
}

trait Profile {
    fn url() -> &'static str;
}

impl Profile for InfoReqExtensions {
    fn url() -> &'static str {
        RESOURCE_PROFILE_COMMUNICATION_INFO_REQ
    }
}

impl Profile for ReplyExtensions {
    fn url() -> &'static str {
        RESOURCE_PROFILE_COMMUNICATION_REPLY
    }
}

impl Profile for DispenseReqExtensions {
    fn url() -> &'static str {
        RESOURCE_PROFILE_COMMUNICATION_DISPENSE_REQ
    }
}

impl Profile for RepresentativeExtensions {
    fn url() -> &'static str {
        RESOURCE_PROFILE_COMMUNICATION_REPRESENTATIVE
    }
}

impl ResourceType for Communication {
    fn resource_type() -> &'static str {
        RESOURCE_TYPE_COMMUNICATION
    }
}

impl<'a> SerializeRoot<'a> for CommunicationCow<'a> {
    type Inner = Communication;

    fn from_inner(inner: &'a Self::Inner) -> Self {
        CommunicationCow(Cow::Borrowed(inner))
    }
}

impl DeserializeRoot for CommunicationCow<'_> {
    type Inner = Communication;

    fn into_inner(self) -> Self::Inner {
        self.0.into_owned()
    }
}

impl CommunicationDef {
    pub fn serialize<S: Serializer>(
        communication: &Communication,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let value: CommunicationHelper = communication.into();

        value.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Cow<'static, Communication>, D::Error> {
        let value = CommunicationHelper::deserialize(deserializer)?;

        Ok(Cow::Owned(value.try_into().map_err(D::Error::custom)?))
    }
}

impl<'a> Into<CommunicationHelper<'a>> for &'a Communication {
    fn into(self) -> CommunicationHelper<'a> {
        match self {
            Communication::Reply(inner) => inner.into(),
            Communication::InfoReq(inner) => inner.into(),
            Communication::DispenseReq(inner) => inner.into(),
            Communication::Representative(inner) => inner.into(),
        }
    }
}

impl<'a, E, R, S> Into<CommunicationHelper<'a>> for &'a Inner<E, R, S>
where
    E: Clone + PartialEq + Profile,
    R: Clone + PartialEq,
    S: Clone + PartialEq,
    &'a E: Into<PayloadExtensionsDef>,
    &'a R: Into<ReferenceDef>,
    &'a S: Into<ReferenceDef>,
{
    fn into(self) -> CommunicationHelper<'a> {
        CommunicationHelper {
            id: self.id.clone(),
            meta: MetaDef {
                profile: vec![E::url().into()],
                ..Default::default()
            },
            based_on: self
                .based_on
                .iter()
                .map(|r| ReferenceDef {
                    reference: Some(r.clone()),
                    ..Default::default()
                })
                .collect(),
            contained: self
                .about
                .iter()
                .map(|medication| ResourceDef::Medication(Cow::Borrowed(medication)))
                .collect(),
            status: "unknown".into(),
            sent: self.sent.clone(),
            received: self.received.clone(),
            about: self
                .about
                .iter()
                .map(|medication| ReferenceDef {
                    reference: Some(format!("#{}", medication.id)),
                    ..Default::default()
                })
                .collect(),
            recipient: vec![(&self.recipient).into()],
            sender: self.sender.as_ref().map(Into::into),
            payload: vec![(&self.payload).into()],
        }
    }
}

impl Into<ReferenceDef> for &Kvnr {
    fn into(self) -> ReferenceDef {
        ReferenceDef {
            identifier: Some(IdentifierDef {
                system: Some(IDENTITY_SYSTEM_KVID.into()),
                value: Some(self.clone().into()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }
}

impl Into<ReferenceDef> for &TelematikId {
    fn into(self) -> ReferenceDef {
        ReferenceDef {
            identifier: Some(IdentifierDef {
                system: Some(IDENTITY_SYSTEM_TELEMATIK_ID.into()),
                value: Some(self.0.clone()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }
}

impl<'a, E> Into<PayloadDef> for &'a Payload<E>
where
    E: Clone + PartialEq,
    &'a E: Into<PayloadExtensionsDef>,
{
    fn into(self) -> PayloadDef {
        PayloadDef {
            content: self.content.clone(),
            extension: self.extensions.as_ref().map(Into::into).unwrap_or_default(),
        }
    }
}

impl Into<PayloadExtensionsDef> for &InfoReqExtensions {
    fn into(self) -> PayloadExtensionsDef {
        let mut ret = vec![
            ExtensionDef {
                url: EXTENSION_URL_INSURANCE_PROVIDER.into(),
                value: Some(ValueDef::Identifier(IdentifierDef {
                    system: Some(IDENTITY_SYSTEM_IKNR.into()),
                    value: Some(self.insurance_provider.clone()),
                    ..Default::default()
                })),
                ..Default::default()
            },
            ExtensionDef {
                url: EXTENSION_URL_SUBSTITUTION_ALLOWED.into(),
                value: Some(ValueDef::Boolean(self.substitution_allowed)),
                ..Default::default()
            },
            ExtensionDef {
                url: EXTENSION_URL_PRESCRIPTION.into(),
                value: Some(ValueDef::Coding(CodingDef {
                    system: Some(CODING_SYSTEM_FLOW_TYPE.into()),
                    code: Some(self.prescription_type.encode_str()),
                    display: Some(self.prescription_type.to_string()),
                    ..Default::default()
                })),
                ..Default::default()
            },
        ];

        if let Some(opts) = &self.preferred_supply_options {
            ret.push(opts.into())
        }

        PayloadExtensionsDef(ret)
    }
}

impl Into<PayloadExtensionsDef> for &ReplyExtensions {
    fn into(self) -> PayloadExtensionsDef {
        let mut ret = Vec::new();

        if let Some(availability) = &self.availability {
            ret.push(ExtensionDef {
                url: EXTENSION_URL_AVAILABILITY_STATUS.into(),
                value: Some(ValueDef::Coding(CodingDef {
                    system: Some(CODING_SYSTEM_AVAILABILITY_STATUS.into()),
                    code: Some(availability.encode_str()),
                    ..Default::default()
                })),
                ..Default::default()
            })
        }

        if let Some(opts) = &self.offered_supply_options {
            ret.push(opts.into())
        }

        PayloadExtensionsDef(ret)
    }
}

impl Into<PayloadExtensionsDef> for &DispenseReqExtensions {
    fn into(self) -> PayloadExtensionsDef {
        let mut ret = Vec::new();

        if let Some(insurance_provider) = &self.insurance_provider {
            ret.push(ExtensionDef {
                url: EXTENSION_URL_INSURANCE_PROVIDER.into(),
                value: Some(ValueDef::Identifier(IdentifierDef {
                    system: Some(IDENTITY_SYSTEM_IKNR.into()),
                    value: Some(insurance_provider.clone()),
                    ..Default::default()
                })),
                ..Default::default()
            })
        }

        PayloadExtensionsDef(ret)
    }
}

impl Into<PayloadExtensionsDef> for &RepresentativeExtensions {
    fn into(self) -> PayloadExtensionsDef {
        PayloadExtensionsDef(Vec::new())
    }
}

impl Into<ExtensionDef> for &SupplyOptions {
    fn into(self) -> ExtensionDef {
        ExtensionDef {
            url: EXTENSION_URL_SUPPLY_OPTIONS.into(),
            extension: vec![
                ExtensionDef {
                    url: "onPremise".into(),
                    value: Some(ValueDef::Boolean(self.on_premise)),
                    ..Default::default()
                },
                ExtensionDef {
                    url: "delivery".into(),
                    value: Some(ValueDef::Boolean(self.delivery)),
                    ..Default::default()
                },
                ExtensionDef {
                    url: "shipment".into(),
                    value: Some(ValueDef::Boolean(self.shipment)),
                    ..Default::default()
                },
            ],
            ..Default::default()
        }
    }
}

impl TryInto<Communication> for CommunicationHelper<'_> {
    type Error = String;

    fn try_into(self) -> Result<Communication, Self::Error> {
        if self.status != "unknown" {
            return Err(format!(
                "Communication has unexpected status (expected=unknown, actual={}",
                self.status
            ));
        }

        let profile = self
            .meta
            .profile
            .get(0)
            .ok_or_else(|| "Communication meta is missing the `profile` field!")?;

        match profile.as_str() {
            RESOURCE_PROFILE_COMMUNICATION_INFO_REQ => Ok(Communication::InfoReq(self.try_into()?)),
            RESOURCE_PROFILE_COMMUNICATION_REPLY => Ok(Communication::Reply(self.try_into()?)),
            RESOURCE_PROFILE_COMMUNICATION_DISPENSE_REQ => {
                Ok(Communication::DispenseReq(self.try_into()?))
            }
            RESOURCE_PROFILE_COMMUNICATION_REPRESENTATIVE => {
                Ok(Communication::Representative(self.try_into()?))
            }
            _ => Err(format!("Communication has unknown profile: {}", profile)),
        }
    }
}

impl<E, R, S> TryInto<Inner<E, R, S>> for CommunicationHelper<'_>
where
    E: Clone + PartialEq,
    R: Clone + PartialEq,
    S: Clone + PartialEq,
    PayloadExtensionsDef: TryInto<Option<E>, Error = String>,
    ReferenceDef: TryInto<R, Error = String>,
    ReferenceDef: TryInto<S, Error = String>,
{
    type Error = String;

    fn try_into(mut self) -> Result<Inner<E, R, S>, Self::Error> {
        Ok(Inner {
            id: self.id,
            based_on: self
                .based_on
                .into_iter()
                .map(|r| -> Result<String, String> {
                    Ok(r.reference
                        .ok_or_else(|| "Communication basedOn is missing the `reference` field!")?)
                })
                .next()
                .transpose()?,
            about: extract_about(&mut self.contained, self.about)?,
            sent: self.sent,
            received: self.received,
            recipient: self
                .recipient
                .into_iter()
                .next()
                .ok_or_else(|| "Communication is missing the `recipient` field!")?
                .try_into()?,
            sender: self.sender.map(TryInto::try_into).transpose()?,
            payload: self
                .payload
                .into_iter()
                .next()
                .ok_or_else(|| "Communication is missing the `payload` field!")?
                .try_into()?,
        })
    }
}

impl TryInto<Kvnr> for ReferenceDef {
    type Error = String;

    fn try_into(self) -> Result<Kvnr, Self::Error> {
        let identifier = self
            .identifier
            .ok_or_else(|| "Communication reference is missing the `identifier` field!")?;
        let system = identifier
            .system
            .ok_or_else(|| "Communication reference identifier is missing the `system` field!")?;
        let value = identifier
            .value
            .ok_or_else(|| "Communication reference identifier is missing the `system` field!")?;

        if system != IDENTITY_SYSTEM_KVID {
            Err(format!(
                "Communication reference identifier has invalid system: {}!",
                system
            ))
        } else {
            Ok(Kvnr::new(value)?)
        }
    }
}

impl TryInto<TelematikId> for ReferenceDef {
    type Error = String;

    fn try_into(self) -> Result<TelematikId, Self::Error> {
        let identifier = self
            .identifier
            .ok_or_else(|| "Communication reference is missing the `identifier` field!")?;
        let system = identifier
            .system
            .ok_or_else(|| "Communication reference identifier is missing the `system` field!")?;
        let value = identifier
            .value
            .ok_or_else(|| "Communication reference identifier is missing the `system` field!")?;

        if system != IDENTITY_SYSTEM_TELEMATIK_ID {
            Err(format!(
                "Communication reference identifier has invalid system: {}!",
                system
            ))
        } else {
            Ok(TelematikId(value))
        }
    }
}

impl<E> TryInto<Payload<E>> for PayloadDef
where
    E: Clone + PartialEq,
    PayloadExtensionsDef: TryInto<Option<E>, Error = String>,
{
    type Error = String;

    fn try_into(self) -> Result<Payload<E>, Self::Error> {
        Ok(Payload {
            content: self.content,
            extensions: self.extension.try_into()?,
        })
    }
}

impl TryInto<Option<InfoReqExtensions>> for PayloadExtensionsDef {
    type Error = String;

    fn try_into(self) -> Result<Option<InfoReqExtensions>, Self::Error> {
        let exts = self.0;
        if exts.is_empty() {
            return Ok(None);
        }

        let mut insurance_provider = None;
        let mut substitution_allowed = None;
        let mut prescription_type = None;
        let mut preferred_supply_options = None;

        for ext in exts {
            match ext.url.as_str() {
                EXTENSION_URL_INSURANCE_PROVIDER => {
                    match ext.value {
                        Some(ValueDef::Identifier(identifier)) => {
                            match identifier.system.as_deref() {
                            Some(IDENTITY_SYSTEM_IKNR) => (),
                            Some(s) => return Err(format!("Communication extension identifier has invalid system: {}", s)),
                            None => return Err("Communication extension identifier is missing the `system` field".into()),
                        }

                            insurance_provider = Some(identifier.value.ok_or_else(|| {
                                "Communication extension identifier is missing the `value` field!"
                            })?);
                        }
                        _ => {
                            return Err(
                                "Communication extension is missing the `valueIdentifier` field!"
                                    .into(),
                            )
                        }
                    }
                }
                EXTENSION_URL_PRESCRIPTION => match ext.value {
                    Some(ValueDef::Coding(coding)) => {
                        match coding.system.as_deref() {
                            Some(CODING_SYSTEM_FLOW_TYPE) => (),
                            Some(s) => {
                                return Err(format!(
                                    "Communication extension coding has invalid system: {}",
                                    s
                                ))
                            }
                            None => {
                                return Err(
                                    "Communication extension coding is missing the `system` field"
                                        .into(),
                                )
                            }
                        }

                        let code = coding.code.ok_or_else(|| {
                            "Communication extension identifier is missing the `code` field!"
                        })?;

                        prescription_type = Some(FlowType::decode_str(&code)?);
                    }
                    _ => {
                        return Err(
                            "Communication extension is missing the `valueCoding` field!".into(),
                        )
                    }
                },
                EXTENSION_URL_SUBSTITUTION_ALLOWED => {
                    substitution_allowed = Some(
                        ext.value
                            .ok_or_else(|| {
                                "Communication extension is missing the `valueBoolean` field!"
                            })?
                            .try_into()
                            .map_err(|err| {
                                format!("Communication extension has invalid value: {}!", err)
                            })?,
                    )
                }
                EXTENSION_URL_SUPPLY_OPTIONS => preferred_supply_options = Some(ext.try_into()?),
                s => {
                    return Err(format!(
                        "Communication contains unexpected extension: {}",
                        s
                    ))
                }
            }
        }

        Ok(Some(InfoReqExtensions {
            insurance_provider: insurance_provider.ok_or_else(|| {
                format!(
                    "Communication is missing extension: {}!",
                    EXTENSION_URL_INSURANCE_PROVIDER
                )
            })?,
            substitution_allowed: substitution_allowed.ok_or_else(|| {
                format!(
                    "Communication is missing extension: {}!",
                    EXTENSION_URL_SUBSTITUTION_ALLOWED
                )
            })?,
            prescription_type: prescription_type.ok_or_else(|| {
                format!(
                    "Communication is missing extension: {}!",
                    EXTENSION_URL_PRESCRIPTION
                )
            })?,
            preferred_supply_options,
        }))
    }
}

impl TryInto<Option<ReplyExtensions>> for PayloadExtensionsDef {
    type Error = String;

    fn try_into(self) -> Result<Option<ReplyExtensions>, Self::Error> {
        let exts = self.0;
        if exts.is_empty() {
            return Ok(None);
        }

        let mut availability = None;
        let mut offered_supply_options = None;

        for ext in exts {
            match ext.url.as_str() {
                EXTENSION_URL_AVAILABILITY_STATUS => match ext.value {
                    Some(ValueDef::Coding(coding)) => {
                        match coding.system.as_deref() {
                            Some(CODING_SYSTEM_AVAILABILITY_STATUS) => (),
                            Some(s) => {
                                return Err(format!(
                                    "Communication extension coding has invalid system: {}",
                                    s
                                ))
                            }
                            None => {
                                return Err(
                                    "Communication extension coding is missing the `system` field"
                                        .into(),
                                )
                            }
                        }

                        let code = coding.code.ok_or_else(|| {
                            "Communication extension identifier is missing the `code` field!"
                        })?;

                        availability = Some(Availability::decode_str(&code)?);
                    }
                    _ => {
                        return Err(
                            "Communication extension is missing the `valueCoding` field!".into(),
                        )
                    }
                },
                EXTENSION_URL_SUPPLY_OPTIONS => offered_supply_options = Some(ext.try_into()?),
                s => {
                    return Err(format!(
                        "Communication contains unexpected extension: {}",
                        s
                    ))
                }
            }
        }

        Ok(Some(ReplyExtensions {
            availability,
            offered_supply_options,
        }))
    }
}

impl TryInto<Option<DispenseReqExtensions>> for PayloadExtensionsDef {
    type Error = String;

    fn try_into(self) -> Result<Option<DispenseReqExtensions>, Self::Error> {
        let exts = self.0;
        if exts.is_empty() {
            return Ok(None);
        }

        let mut insurance_provider = None;

        for ext in exts {
            match ext.url.as_str() {
                EXTENSION_URL_INSURANCE_PROVIDER => {
                    match ext.value {
                        Some(ValueDef::Identifier(identifier)) => {
                            match identifier.system.as_deref() {
                            Some(IDENTITY_SYSTEM_IKNR) => (),
                            Some(s) => return Err(format!("Communication extension identifier has invalid system: {}", s)),
                            None => return Err("Communication extension identifier is missing the `system` field".into()),
                        }

                            insurance_provider = Some(identifier.value.ok_or_else(|| {
                                "Communication extension identifier is missing the `value` field!"
                            })?);
                        }
                        _ => {
                            return Err(
                                "Communication extension is missing the `valueIdentifier` field!"
                                    .into(),
                            )
                        }
                    }
                }
                s => {
                    return Err(format!(
                        "Communication contains unexpected extension: {}",
                        s
                    ))
                }
            }
        }

        Ok(Some(DispenseReqExtensions { insurance_provider }))
    }
}

impl TryInto<Option<RepresentativeExtensions>> for PayloadExtensionsDef {
    type Error = String;

    fn try_into(self) -> Result<Option<RepresentativeExtensions>, Self::Error> {
        Ok(None)
    }
}

impl TryInto<SupplyOptions> for ExtensionDef {
    type Error = String;

    fn try_into(self) -> Result<SupplyOptions, Self::Error> {
        let mut on_premise = None;
        let mut delivery = None;
        let mut shipment = None;

        for ext in self.extension {
            match ext.url.as_str() {
                "onPremise" => {
                    on_premise = Some(
                        ext.value
                            .ok_or_else(|| {
                                "Supply Option Extension onPremise is missing the `value` field"
                            })?
                            .try_into()
                            .map_err(|err| {
                                format!(
                                    "Supply Option Extension onPremise has invalid value: {}!",
                                    err
                                )
                            })?,
                    )
                }
                "delivery" => {
                    delivery = Some(
                        ext.value
                            .ok_or_else(|| {
                                "Supply Option Extension delivery is missing the `value` field"
                            })?
                            .try_into()
                            .map_err(|err| {
                                format!(
                                    "Supply Option Extension delivery has invalid value: {}!",
                                    err
                                )
                            })?,
                    )
                }
                "shipment" => {
                    shipment = Some(
                        ext.value
                            .ok_or_else(|| {
                                "Supply Option Extension shipment is missing the `value` field"
                            })?
                            .try_into()
                            .map_err(|err| {
                                format!(
                                    "Supply Option Extension shipment has invalid value: {}!",
                                    err
                                )
                            })?,
                    )
                }
                _ => (),
            }
        }

        Ok(SupplyOptions {
            on_premise: on_premise
                .ok_or_else(|| "Supply Option Extension is missing the `onPremise` extension!")?,
            delivery: delivery
                .ok_or_else(|| "Supply Option Extension is missing the `delivery` extension!")?,
            shipment: shipment
                .ok_or_else(|| "Supply Option Extension is missing the `shipment` extension!")?,
        })
    }
}

fn extract_about(
    contained: &mut Vec<ResourceDef<'_>>,
    about: Vec<ReferenceDef>,
) -> Result<Vec<Medication>, String> {
    about
        .into_iter()
        .map(|r| {
            let r: String = r
                .reference
                .ok_or_else(|| "Communication about is missing the `reference` field!")?;

            if !r.starts_with('#') {
                return Err(format!(
                    "Communication about contains invalid reference: {}!",
                    r
                ));
            }

            let r = &r[1..];
            let p = contained
                .iter()
                .position(|c| {
                    let ResourceDef::Medication(m) = c;

                    *m.id == r
                })
                .ok_or_else(|| {
                    format!(
                        "Communication does not contain the referenced object: {}!",
                        r
                    )
                })?;

            let ResourceDef::Medication(ret) = contained.remove(p);

            Ok(ret.into_owned())
        })
        .collect()
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::convert::TryInto;
    use std::fs::read_to_string;

    use resources::{
        communication::Availability,
        medication::{
            Amount, Category, Data, Extension, Medication, PznCode, PznData, PznForm, StandardSize,
        },
        types::FlowType,
    };

    use crate::fhir::{
        json::{from_str as from_json, to_string as to_json},
        test::trim_json_str,
    };

    #[test]
    fn convert_to_info_req() {
        let communication = test_communication_info_req();

        let actual = trim_json_str(&to_json(&CommunicationRoot::new(&communication)).unwrap());
        let expected =
            trim_json_str(&read_to_string("./examples/communication_info_req.json").unwrap());

        assert_eq!(actual, expected);
    }

    #[test]
    fn convert_to_reply() {
        let communication = test_communication_reply();

        let actual = trim_json_str(&to_json(&CommunicationRoot::new(&communication)).unwrap());
        let expected =
            trim_json_str(&read_to_string("./examples/communication_reply.json").unwrap());

        assert_eq!(actual, expected);
    }

    #[test]
    fn convert_to_dispense_req() {
        let communication = test_communication_dispense_req();

        let actual = trim_json_str(&to_json(&CommunicationRoot::new(&communication)).unwrap());
        let expected =
            trim_json_str(&read_to_string("./examples/communication_dispense_req.json").unwrap());

        assert_eq!(actual, expected);
    }

    #[test]
    fn convert_from_info_req() {
        let json = read_to_string("./examples/communication_info_req.json").unwrap();
        let actual = from_json::<CommunicationRoot>(&json).unwrap().into_inner();
        let expected = test_communication_info_req();

        assert_eq!(actual, expected);
    }

    #[test]
    fn convert_from_reply() {
        let json = read_to_string("./examples/communication_reply.json").unwrap();
        let actual = from_json::<CommunicationRoot>(&json).unwrap().into_inner();
        let expected = test_communication_reply();

        assert_eq!(actual, expected);
    }

    #[test]
    fn convert_from_dispense_req() {
        let json = read_to_string("./examples/communication_dispense_req.json").unwrap();
        let actual = from_json::<CommunicationRoot>(&json).unwrap().into_inner();
        let expected = test_communication_dispense_req();

        assert_eq!(actual, expected);
    }

    pub fn test_communication_info_req() -> Communication {
        Communication::InfoReq(Inner {
            id: None,
            based_on: None,
            about: vec![test_medication()],
            sent: Some("2020-03-12T18:01:10+00:00".try_into().unwrap()),
            received: None,
            recipient: TelematikId("606358757".into()),
            sender: Some(Kvnr::new("X234567890").unwrap()),
            payload: Payload {
                content:
                    "Hallo, ich wollte gern fragen, ob das Medikament bei Ihnen vorraetig ist."
                        .into(),
                extensions: Some(InfoReqExtensions {
                    insurance_provider: "104212059".into(),
                    substitution_allowed: true,
                    prescription_type: FlowType::PharmaceuticalDrugs,
                    preferred_supply_options: Some(SupplyOptions {
                        on_premise: true,
                        delivery: true,
                        shipment: false,
                    }),
                }),
            },
        })
    }

    pub fn test_communication_reply() -> Communication {
        Communication::Reply(Inner {
            id: None,
            based_on: None,
            about: vec![],
            sent: Some("2020-03-12T18:01:10+00:00".try_into().unwrap()),
            received: None,
            recipient: Kvnr::new("X234567890").unwrap(),
            sender: Some(TelematikId("606358757".into())),
            payload: Payload {
                content:
                    "Hallo, wir haben das Medikament vorraetig. Kommen Sie gern in die Filiale oder wir schicken einen Boten."
                        .into(),
                extensions: Some(ReplyExtensions {
                    availability: Some(Availability::Now),
                    offered_supply_options: Some(SupplyOptions {
                        on_premise: true,
                        delivery: true,
                        shipment: true,
                    }),
                }),
            },
        })
    }

    pub fn test_communication_dispense_req() -> Communication {
        Communication::DispenseReq(Inner {
            id: None,
            based_on: Some("Task/4711/$accept?ac=777bea0e13cc9c42ceec14aec3ddee2263325dc2c6c699db115f58fe423607ea".into()),
            about: vec![],
            sent: Some("2020-03-12T18:01:10+00:00".try_into().unwrap()),
            received: None,
            recipient: TelematikId("606358757".into()),
            sender: Some(Kvnr::new("X234567890").unwrap()),
            payload: Payload {
                content: "Bitte schicken Sie einen Boten.".into(),
                extensions: None,
            },
        })
    }

    pub fn test_medication() -> Medication {
        Medication {
            id: "5fe6e06c-8725-46d5-aecd-e65e041ca3de".try_into().unwrap(),
            data: Data::Pzn(PznData {
                code: PznCode {
                    text: "Sumatriptan-1a Pharma 100 mg Tabletten".into(),
                    code: "06313728".into(),
                },
                form: PznForm {
                    system: "https://fhir.kbv.de/CodeSystem/KBV_CS_SFHIR_KBV_DARREICHUNGSFORM"
                        .into(),
                    code: "TAB".into(),
                },
                amount: Some(Amount {
                    value: 12,
                    unit: "TAB".into(),
                    code: Some("{tbl}".into()),
                }),
            }),
            extension: Some(Extension {
                category: Category::Medicine,
                vaccine: false,
                instruction: None,
                packaging: None,
                standard_size: Some(StandardSize::N1),
            }),
        }
    }
}
