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
use resources::medication_dispense::{DosageInstruction, MedicationDispense};

use crate::fhir::{
    decode::{decode_any, DataStream, Decode, DecodeError, DecodeStream, Fields},
    encode::{encode_any, DataStorage, Encode, EncodeError, EncodeStream},
};

use super::{
    meta::Meta,
    primitives::{decode_identifier, decode_reference, encode_identifier, encode_reference},
    DecodeBundleResource, EncodeBundleResource,
};

/* Decode */

impl DecodeBundleResource for MedicationDispense {}

#[async_trait(?Send)]
impl Decode for MedicationDispense {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&[
            "id",
            "meta",
            "identifier",
            "status",
            "medicationReference",
            "subject",
            "supportingInformation",
            "performer",
            "whenPrepared",
            "whenHandedOver",
            "dosageInstruction",
        ]);

        stream.root("MedicationDispense").await?;

        let id = stream.decode_opt(&mut fields, decode_any).await?;
        let meta = stream.decode::<Meta, _>(&mut fields, decode_any).await?;
        let prescription_id = stream.decode(&mut fields, decode_identifier).await?;
        let _status = stream.fixed(&mut fields, "completed").await?;
        let medication = stream.decode(&mut fields, decode_reference).await?;
        let subject = {
            stream.begin_substream(&mut fields).await?;
            stream.element().await?;

            let mut fields = Fields::new(&["identifier"]);
            let identifier = stream.decode(&mut fields, decode_identifier).await?;

            stream.end().await?;
            stream.end_substream().await?;

            identifier
        };
        let supporting_information = stream.decode_vec(&mut fields, decode_reference).await?;
        let performer = {
            stream.begin_substream(&mut fields).await?;
            stream.element().await?;

            let mut fields = Fields::new(&["actor"]);

            stream.begin_substream(&mut fields).await?;
            stream.element().await?;

            let mut fields = Fields::new(&["identifier"]);
            let identifier = stream.decode(&mut fields, decode_identifier).await?;

            stream.end().await?;
            stream.end_substream().await?;

            stream.end().await?;
            stream.end_substream().await?;

            identifier
        };
        let when_prepared = stream.decode_opt(&mut fields, decode_any).await?;
        let when_handed_over = stream.decode(&mut fields, decode_any).await?;
        let dosage_instruction = stream.decode_vec(&mut fields, decode_any).await?;

        stream.end().await?;

        if !meta.profiles.iter().any(|p| icase_eq(p, PROFILE)) {
            return Err(DecodeError::InvalidProfile {
                actual: meta.profiles,
                expected: vec![PROFILE.into()],
            });
        }

        Ok(MedicationDispense {
            id,
            prescription_id,
            medication,
            subject,
            supporting_information,
            performer,
            when_prepared,
            when_handed_over,
            dosage_instruction,
        })
    }
}

#[async_trait(?Send)]
impl Decode for DosageInstruction {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["text"]);

        stream.element().await?;

        let text = stream.decode_opt(&mut fields, decode_any).await?;

        stream.end().await?;

        Ok(DosageInstruction { text })
    }
}

/* Encode */

impl EncodeBundleResource for &MedicationDispense {}

impl Encode for &MedicationDispense {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let meta = Meta {
            profiles: vec![PROFILE.into()],
            ..Default::default()
        };

        stream
            .root("MedicationDispense")?
            .encode_opt("id", &self.id, encode_any)?
            .encode("meta", meta, encode_any)?
            .encode_vec("identifier", once(&self.prescription_id), encode_identifier)?
            .encode("status", "completed", encode_any)?
            .encode("medicationReference", &self.medication, encode_reference)?
            .field_name("subject")?
            .element()?
            .encode("identifier", &self.subject, encode_identifier)?
            .end()?
            .encode_vec(
                "supportingInformation",
                &self.supporting_information,
                encode_reference,
            )?
            .field_name("performer")?
            .array()?
            .element()?
            .field_name("actor")?
            .element()?
            .encode("identifier", &self.performer, encode_identifier)?
            .end()?
            .end()?
            .end()?
            .encode_opt("whenPrepared", &self.when_prepared, encode_any)?
            .encode("whenHandedOver", &self.when_handed_over, encode_any)?
            .encode_vec("dosageInstruction", &self.dosage_instruction, encode_any)?
            .end()?;

        Ok(())
    }
}

impl Encode for &DosageInstruction {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .encode_opt("text", &self.text, encode_any)?
            .end()?;

        Ok(())
    }
}

pub const PROFILE: &str = "https://gematik.de/fhir/StructureDefinition/erxMedicationDispense";

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::convert::TryInto;
    use std::fs::read_to_string;
    use std::str::from_utf8;

    use resources::misc::{Kvnr, TelematikId};

    use crate::fhir::{
        decode::{tests::load_stream, JsonDecode, XmlDecode},
        encode::{JsonEncode, XmlEncode},
    };

    use super::super::super::tests::{trim_json_str, trim_xml_str};

    #[tokio::test]
    async fn test_decode_json() {
        let mut stream = load_stream("./examples/medication_dispense.json");

        let actual = stream.json::<MedicationDispense>().await.unwrap();
        let expected = test_medication_dispense();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_decode_xml() {
        let mut stream = load_stream("./examples/medication_dispense.xml");

        let actual = stream.xml::<MedicationDispense>().await.unwrap();
        let expected = test_medication_dispense();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_encode_json() {
        let value = test_medication_dispense();

        let actual = (&value).json().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/medication_dispense.json").unwrap();

        assert_eq!(trim_json_str(&actual), trim_json_str(&expected));
    }

    #[tokio::test]
    async fn test_encode_xml() {
        let value = test_medication_dispense();

        let actual = (&value).xml().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/medication_dispense.xml").unwrap();

        assert_eq!(trim_xml_str(&actual), trim_xml_str(&expected));
    }

    pub fn test_medication_dispense() -> MedicationDispense {
        MedicationDispense {
            id: None,
            prescription_id: "160.123.456.789.123.58".parse().unwrap(),
            medication: "Medication/1234".into(),
            subject: Kvnr::new("X234567890").unwrap(),
            supporting_information: Vec::new(),
            performer: TelematikId::new("606358757"),
            when_prepared: None,
            when_handed_over: "2020-03-20T07:13:00+05:00".try_into().unwrap(),
            dosage_instruction: vec![DosageInstruction {
                text: Some("1-0-1-0".into()),
            }],
        }
    }
}
