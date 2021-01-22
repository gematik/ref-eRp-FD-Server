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
use resources::medication_request::{
    AccidentCause, AccidentInformation, CoPayment, DispenseRequest, Dosage, Extension,
    MedicationRequest, MultiPrescription, SeriesElement, TimeRange,
};

use crate::fhir::{
    decode::{decode_any, DataStream, Decode, DecodeError, DecodeStream, Fields, Search},
    encode::{encode_any, DataStorage, Encode, EncodeError, EncodeStream},
};

use super::{
    meta::Meta,
    primitives::{
        decode_coding, decode_reference, encode_coding, encode_reference, CodeEx, CodingEx,
    },
};

/* Decode */

#[async_trait(?Send)]
impl Decode for MedicationRequest {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&[
            "id",
            "meta",
            "extension",
            "status",
            "intent",
            "medicationReference",
            "subject",
            "authoredOn",
            "requester",
            "insurance",
            "note",
            "dosageInstruction",
            "dispenseRequest",
            "substitution",
        ]);

        stream.root("MedicationRequest").await?;

        let id = stream.decode(&mut fields, decode_any).await?;
        let meta = stream.decode::<Meta, _>(&mut fields, decode_any).await?;
        let extension = stream.decode(&mut fields, decode_any).await?;
        let _status = stream.fixed(&mut fields, "active").await?;
        let _intent = stream.fixed(&mut fields, "order").await?;
        let medication = stream.decode(&mut fields, decode_reference).await?;
        let subject = stream.decode(&mut fields, decode_reference).await?;
        let authored_on = stream.decode(&mut fields, decode_any).await?;
        let requester = stream.decode(&mut fields, decode_reference).await?;
        let insurance = stream.decode(&mut fields, decode_reference).await?;
        let note = if stream.begin_substream_opt(&mut fields).await? {
            let mut fields = Fields::new(&["text"]);

            stream.element().await?;

            let text = stream.decode(&mut fields, decode_any).await?;

            stream.end().await?;
            stream.end_substream().await?;

            Some(text)
        } else {
            None
        };
        let dosage = stream.decode_opt(&mut fields, decode_any).await?;
        let dispense_request = stream.decode(&mut fields, decode_any).await?;
        let substitution_allowed = {
            stream.begin_substream(&mut fields).await?;
            stream.element().await?;

            let mut fields = Fields::new(&["allowedBoolean"]);
            let allowed = stream.decode(&mut fields, decode_any).await?;

            stream.end().await?;
            stream.end_substream().await?;

            allowed
        };

        stream.end().await?;

        if !meta.profiles.iter().any(|p| icase_eq(p, PROFILE)) {
            return Err(DecodeError::InvalidProfile {
                actual: meta.profiles,
                expected: vec![PROFILE.into()],
            });
        }

        Ok(MedicationRequest {
            id,
            extension,
            medication,
            subject,
            authored_on,
            requester,
            insurance,
            note,
            dosage,
            dispense_request,
            substitution_allowed,
        })
    }
}

#[async_trait(?Send)]
impl Decode for Extension {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut co_payment = None;
        let mut emergency_service_fee = None;
        let mut bvg = None;
        let mut accident_information = None;
        let mut multi_prescription = None;

        let mut fields = Fields::new(&["extension"]);
        while stream.begin_substream_vec(&mut fields).await? {
            stream.element().await?;

            let url = stream.value(Search::Exact("url")).await?.unwrap();

            match url.as_str() {
                x if icase_eq(x, URL_CO_PAYMENT) => {
                    let mut fields = Fields::new(&["valueCoding"]);

                    co_payment = Some(stream.decode(&mut fields, decode_coding).await?);
                }
                x if icase_eq(x, URL_EMERGENCY_SERVICE_FEE) => {
                    let mut fields = Fields::new(&["valueBoolean"]);

                    emergency_service_fee = Some(stream.decode(&mut fields, decode_any).await?);
                }
                x if icase_eq(x, URL_BVG) => {
                    let mut fields = Fields::new(&["valueBoolean"]);

                    bvg = Some(stream.decode(&mut fields, decode_any).await?);
                }
                x if icase_eq(x, URL_ACCIDENT_INFORMATION) => {
                    let mut fields = Fields::new(&["extension"]);

                    accident_information = Some(stream.decode(&mut fields, decode_any).await?);
                }
                x if icase_eq(x, URL_MULTI_PRESCRIPTION) => {
                    let mut fields = Fields::new(&["extension"]);

                    multi_prescription = Some(stream.decode(&mut fields, decode_any).await?);
                }
                _ => (),
            }

            stream.end().await?;
            stream.end_substream().await?;
        }

        let emergency_service_fee =
            emergency_service_fee.ok_or_else(|| DecodeError::MissingExtension {
                url: URL_EMERGENCY_SERVICE_FEE.into(),
                path: stream.path().into(),
            })?;
        let bvg = bvg.ok_or_else(|| DecodeError::MissingExtension {
            url: URL_BVG.into(),
            path: stream.path().into(),
        })?;
        let multi_prescription =
            multi_prescription.ok_or_else(|| DecodeError::MissingExtension {
                url: URL_MULTI_PRESCRIPTION.into(),
                path: stream.path().into(),
            })?;

        Ok(Extension {
            co_payment,
            emergency_service_fee,
            bvg,
            accident_information,
            multi_prescription,
        })
    }
}

#[async_trait(?Send)]
impl Decode for AccidentInformation {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut cause = None;
        let mut business = None;
        let mut date = None;

        let mut fields = Fields::new(&["extension"]);
        while stream.begin_substream_vec(&mut fields).await? {
            stream.element().await?;

            let url = stream.value(Search::Exact("url")).await?.unwrap();

            match url.as_str() {
                "unfallkennzeichen" => {
                    let mut fields = Fields::new(&["valueCoding"]);

                    cause = Some(stream.decode(&mut fields, decode_coding).await?);
                }
                "unfallbetrieb" => {
                    let mut fields = Fields::new(&["valueString"]);

                    business = Some(stream.decode(&mut fields, decode_any).await?);
                }
                "unfalltag" => {
                    let mut fields = Fields::new(&["valueDate"]);

                    date = Some(stream.decode(&mut fields, decode_any).await?);
                }
                _ => (),
            }

            stream.end().await?;
            stream.end_substream().await?;
        }

        let cause = cause.ok_or_else(|| DecodeError::MissingExtension {
            url: "unfallkennzeichen".into(),
            path: stream.path().into(),
        })?;
        let date = date.ok_or_else(|| DecodeError::MissingExtension {
            url: "unfalltag".into(),
            path: stream.path().into(),
        })?;

        Ok(AccidentInformation {
            cause,
            business,
            date,
        })
    }
}

#[async_trait(?Send)]
impl Decode for Dosage {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["extension", "text", "patientInstruction"]);

        stream.element().await?;

        let dosage_mark = if stream.begin_substream_opt(&mut fields).await? {
            let mut dosage_mark = None;

            while stream.begin_substream_vec(&mut Fields::Any).await? && dosage_mark.is_none() {
                stream.element().await?;

                let url = stream.value(Search::Exact("url")).await?.unwrap();

                if url == URL_DOSAGE_MARK {
                    let mut fields = Fields::new(&["valueBoolean"]);

                    dosage_mark = Some(stream.decode(&mut fields, decode_any).await?);
                }

                stream.end().await?;
                stream.end_substream().await?;
            }

            stream.end_substream().await?;

            dosage_mark
        } else {
            None
        };
        let text = stream.decode_opt(&mut fields, decode_any).await?;
        let patient_instruction = stream.decode_opt(&mut fields, decode_any).await?;

        stream.end().await?;

        Ok(Dosage {
            dosage_mark,
            text,
            patient_instruction,
        })
    }
}

#[async_trait(?Send)]
impl Decode for DispenseRequest {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["validityPeriod", "quantity"]);

        stream.element().await?;

        let (validity_period_start, validity_period_end) =
            if stream.begin_substream_opt(&mut fields).await? {
                let mut fields = Fields::new(&["start", "end"]);

                stream.element().await?;

                let start = stream.decode(&mut fields, decode_any).await?;
                let end = stream.decode_opt(&mut fields, decode_any).await?;

                stream.end().await?;
                stream.end_substream().await?;

                (Some(start), end)
            } else {
                (None, None)
            };
        let quantity = {
            stream.begin_substream(&mut fields).await?;
            stream.element().await?;

            let mut fields = Fields::new(&["value", "system", "code"]);

            let value = stream.decode(&mut fields, decode_any).await?;
            let _system = stream.ifixed(&mut fields, SYSTEM_QUANTITY).await?;
            let _code = stream.fixed(&mut fields, "{Package}").await?;

            stream.end().await?;
            stream.end_substream().await?;

            value
        };

        stream.end().await?;

        Ok(DispenseRequest {
            quantity,
            validity_period_start,
            validity_period_end,
        })
    }
}

#[async_trait(?Send)]
impl Decode for Option<MultiPrescription> {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut flag = None;
        let mut series_element = None;
        let mut time_range = None;

        let mut fields = Fields::new(&["extension"]);
        while stream.begin_substream_vec(&mut fields).await? {
            stream.element().await?;

            let url = stream.value(Search::Exact("url")).await?.unwrap();

            match url.as_str() {
                "Kennzeichen" => {
                    let mut fields = Fields::new(&["valueBoolean"]);

                    flag = Some(stream.decode(&mut fields, decode_any).await?);
                }
                "Nummerierung" => {
                    let mut fields = Fields::new(&["valueRatio"]);

                    series_element = Some(stream.decode(&mut fields, decode_any).await?);
                }
                "Zeitraum" => {
                    let mut fields = Fields::new(&["valuePeriod"]);

                    time_range = Some(stream.decode(&mut fields, decode_any).await?);
                }
                _ => (),
            }

            stream.end().await?;
            stream.end_substream().await?;
        }

        let flag = flag.ok_or_else(|| DecodeError::MissingExtension {
            url: "Kennzeichen".into(),
            path: stream.path().into(),
        })?;

        if flag {
            let series_element = series_element.ok_or_else(|| DecodeError::MissingExtension {
                url: "Nummerierung".into(),
                path: stream.path().into(),
            })?;
            let time_range = time_range.ok_or_else(|| DecodeError::MissingExtension {
                url: "Zeitraum".into(),
                path: stream.path().into(),
            })?;

            Ok(Some(MultiPrescription {
                series_element,
                time_range,
            }))
        } else {
            Ok(None)
        }
    }
}

#[async_trait(?Send)]
impl Decode for SeriesElement {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["numerator", "denominator"]);

        stream.element().await?;

        stream.begin_substream(&mut fields).await?;
        stream.element().await?;
        let mut subfield = Fields::new(&["value"]);
        let numerator = stream.decode(&mut subfield, decode_any).await?;
        stream.end().await?;
        stream.end_substream().await?;

        stream.begin_substream(&mut fields).await?;
        stream.element().await?;
        let mut subfield = Fields::new(&["value"]);
        let denominator = stream.decode(&mut subfield, decode_any).await?;
        stream.end().await?;
        stream.end_substream().await?;

        stream.end().await?;

        Ok(SeriesElement {
            numerator,
            denominator,
        })
    }
}

#[async_trait(?Send)]
impl Decode for TimeRange {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["start", "end"]);

        stream.element().await?;

        let start = stream.decode_opt(&mut fields, decode_any).await?;
        let end = stream.decode_opt(&mut fields, decode_any).await?;

        stream.end().await?;

        Ok(TimeRange { start, end })
    }
}

/* Encode */

impl Encode for &MedicationRequest {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let meta = Meta {
            profiles: vec![PROFILE.into()],
            ..Default::default()
        };

        stream
            .root("MedicationRequest")?
            .encode("id", &self.id, encode_any)?
            .encode("meta", meta, encode_any)?
            .encode("extension", &self.extension, encode_any)?
            .encode("status", "active", encode_any)?
            .encode("intent", "order", encode_any)?
            .encode("medicationReference", &self.medication, encode_reference)?
            .encode("subject", &self.subject, encode_reference)?
            .encode("authoredOn", &self.authored_on, encode_any)?
            .encode("requester", &self.requester, encode_reference)?
            .encode_vec("insurance", once(&self.insurance), encode_reference)?;

        if let Some(note) = &self.note {
            stream
                .field_name("note")?
                .element()?
                .encode("text", note, encode_any)?
                .end()?;
        }

        stream
            .encode_vec("dosageInstruction", &self.dosage, encode_any)?
            .encode("dispenseRequest", &self.dispense_request, encode_any)?
            .field_name("substitution")?
            .element()?
            .encode("allowedBoolean", self.substitution_allowed, encode_any)?
            .end()?
            .end()?;

        Ok(())
    }
}

impl Encode for &Extension {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream.array()?;

        if let Some(co_payment) = &self.co_payment {
            stream
                .element()?
                .attrib("url", URL_CO_PAYMENT, encode_any)?
                .encode("valueCoding", co_payment, encode_coding)?
                .end()?;
        }

        stream
            .element()?
            .attrib("url", URL_EMERGENCY_SERVICE_FEE, encode_any)?
            .encode("valueBoolean", &self.emergency_service_fee, encode_any)?
            .end()?;

        stream
            .element()?
            .attrib("url", URL_BVG, encode_any)?
            .encode("valueBoolean", &self.bvg, encode_any)?
            .end()?;

        if let Some(accident_information) = &self.accident_information {
            stream
                .element()?
                .attrib("url", URL_ACCIDENT_INFORMATION, encode_any)?
                .encode("extension", accident_information, encode_any)?
                .end()?;
        }

        stream
            .element()?
            .attrib("url", URL_MULTI_PRESCRIPTION, encode_any)?
            .encode("extension", &self.multi_prescription, encode_any)?
            .end()?;

        stream.end()?;

        Ok(())
    }
}

impl Encode for &AccidentInformation {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .array()?
            .element()?
            .attrib("url", "unfallkennzeichen", encode_any)?
            .encode("valueCoding", &self.cause, encode_coding)?
            .end()?;

        if let Some(business) = &self.business {
            stream
                .element()?
                .attrib("url", "unfallbetrieb", encode_any)?
                .encode("valueString", business, encode_any)?
                .end()?;
        }

        stream
            .element()?
            .attrib("url", "unfalltag", encode_any)?
            .encode("valueDate", &self.date, encode_any)?
            .end()?;

        stream.end()?;

        Ok(())
    }
}

impl Encode for &Option<MultiPrescription> {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let flag = self.is_some();

        stream
            .array()?
            .element()?
            .attrib("url", "Kennzeichen", encode_any)?
            .encode("valueBoolean", &flag, encode_any)?
            .end()?;

        if let Some(multi_prescription) = self {
            stream
                .element()?
                .attrib("url", "Nummerierung", encode_any)?
                .encode("valueRatio", &multi_prescription.series_element, encode_any)?
                .end()?;

            stream
                .element()?
                .attrib("url", "Zeitraum", encode_any)?
                .encode("valuePeriod", &multi_prescription.time_range, encode_any)?
                .end()?;
        }

        stream.end()?;

        Ok(())
    }
}

impl Encode for &Dosage {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream.element()?;

        if let Some(dosage_mark) = &self.dosage_mark {
            stream
                .field_name("extension")?
                .array()?
                .element()?
                .attrib("url", URL_DOSAGE_MARK, encode_any)?
                .encode("valueBoolean", dosage_mark, encode_any)?
                .end()?
                .end()?;
        }

        stream
            .encode_opt("text", &self.text, encode_any)?
            .encode_opt("patientInstruction", &self.patient_instruction, encode_any)?
            .end()?;

        Ok(())
    }
}

impl Encode for &DispenseRequest {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream.element()?;

        if let Some(start) = &self.validity_period_start {
            stream
                .field_name("validityPeriod")?
                .element()?
                .encode("start", start, encode_any)?
                .encode_opt("end", &self.validity_period_end, encode_any)?
                .end()?;
        }

        stream
            .field_name("quantity")?
            .element()?
            .encode("value", &self.quantity, encode_any)?
            .encode("system", SYSTEM_QUANTITY, encode_any)?
            .encode("code", "{Package}", encode_any)?
            .end()?
            .end()?;

        Ok(())
    }
}

impl Encode for &SeriesElement {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .field_name("numerator")?
            .element()?
            .encode("value", &self.numerator, encode_any)?
            .end()?
            .field_name("denominator")?
            .element()?
            .encode("value", &self.denominator, encode_any)?
            .end()?
            .end()?;

        Ok(())
    }
}

impl Encode for &TimeRange {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode_opt("start", &self.start, encode_any)?
            .encode_opt("end", &self.end, encode_any)?
            .end()?;

        Ok(())
    }
}

/* Misc */

impl CodingEx for CoPayment {
    type Code = Self;

    fn from_parts(code: Self::Code) -> Self {
        code
    }

    fn code(&self) -> &Self::Code {
        &self
    }

    fn system() -> Option<&'static str> {
        Some(SYSTEM_CO_PAYMENT)
    }
}

impl CodeEx for CoPayment {
    fn from_parts(value: String) -> Result<Self, String> {
        match value.as_str() {
            "0" => Ok(Self::NotExceptFrom),
            "1" => Ok(Self::ExceptFrom),
            "2" => Ok(Self::ArtificialFertilization),
            _ => Err(value),
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::NotExceptFrom => "0",
            Self::ExceptFrom => "1",
            Self::ArtificialFertilization => "2",
        }
    }
}

impl CodingEx for AccidentCause {
    type Code = Self;

    fn from_parts(code: Self::Code) -> Self {
        code
    }

    fn code(&self) -> &Self::Code {
        &self
    }

    fn system() -> Option<&'static str> {
        Some(SYSTEM_ACCIDENT_CAUSE)
    }
}

impl CodeEx for AccidentCause {
    fn from_parts(value: String) -> Result<Self, String> {
        match value.as_str() {
            "1" => Ok(Self::Accident),
            "2" => Ok(Self::WorkAccident),
            "3" => Ok(Self::SupplyProblem),
            _ => Err(value),
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Accident => "1",
            Self::WorkAccident => "2",
            Self::SupplyProblem => "3",
        }
    }
}

const PROFILE: &str = "https://fhir.kbv.de/StructureDefinition/KBV_PR_ERP_Prescription|1.0.0";

const URL_CO_PAYMENT: &str = "https://fhir.kbv.de/StructureDefinition/KBV_EX_ERP_StatusCoPayment";
const URL_EMERGENCY_SERVICE_FEE: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_EX_ERP_EmergencyServicesFee";
const URL_BVG: &str = "https://fhir.kbv.de/StructureDefinition/KBV_EX_ERP_BVG";
const URL_ACCIDENT_INFORMATION: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_EX_ERP_Accident";
const URL_DOSAGE_MARK: &str = "https://fhir.kbv.de/StructureDefinition/KBV_EX_ERP_DosageFlag";
const URL_MULTI_PRESCRIPTION: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_EX_ERP_Multiple_Prescription";
const SYSTEM_CO_PAYMENT: &str = "https://fhir.kbv.de/CodeSystem/KBV_CS_ERP_StatusCoPayment";
const SYSTEM_ACCIDENT_CAUSE: &str = "https://fhir.kbv.de/CodeSystem/KBV_CS_FOR_Ursache_Type";
const SYSTEM_QUANTITY: &str = "http://unitsofmeasure.org";

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
        let mut stream = load_stream("./examples/medication_request.json");

        let actual = stream.json::<MedicationRequest>().await.unwrap();
        let expected = test_medication_request();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_decode_xml() {
        let mut stream = load_stream("./examples/medication_request.xml");

        let actual = stream.xml::<MedicationRequest>().await.unwrap();
        let expected = test_medication_request();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_encode_json() {
        let value = test_medication_request();

        let actual = (&value).json().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/medication_request.json").unwrap();

        assert_eq!(trim_json_str(&actual), trim_json_str(&expected));
    }

    #[tokio::test]
    async fn test_encode_xml() {
        let value = test_medication_request();

        let actual = (&value).xml().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/medication_request.xml").unwrap();

        assert_eq!(trim_xml_str(&actual), trim_xml_str(&expected));
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
                multi_prescription: Some(MultiPrescription {
                    series_element: SeriesElement {
                        numerator: 2,
                        denominator: 4,
                    },
                    time_range: TimeRange {
                        start: Some("2021-01-02".try_into().unwrap()),
                        end: Some("2021-03-30".try_into().unwrap()),
                    },
                }),
            },
            medication: "Medication/5fe6e06c-8725-46d5-aecd-e65e041ca3de".into(),
            subject: "Patient/9774f67f-a238-4daf-b4e6-679deeef3811".into(),
            authored_on: "2020-02-03T00:00:00+00:00".try_into().unwrap(),
            requester: "Practitioner/20597e0e-cb2a-45b3-95f0-dc3dbdb617c3".into(),
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
