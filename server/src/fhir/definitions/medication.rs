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
use std::iter::once;

use async_trait::async_trait;
use resources::medication::{
    Amount, Category, CompoundingData, Data, Extension, FreeTextData, Ingredient, IngredientData,
    Medication, PznCode, PznData, PznForm, StandardSize,
};

use crate::fhir::{
    decode::{decode_any, DataStream, Decode, DecodeError, DecodeStream, Fields, Search},
    encode::{encode_any, DataStorage, Encode, EncodeError, EncodeStream},
};

use super::{
    meta::Meta,
    primitives::{
        decode_amount, decode_code, decode_codeable_concept, decode_coding, encode_amount,
        encode_code, encode_codeable_concept, encode_coding, AmountEx, CodeEx, CodeableConcept,
        CodeableConceptEx, CodingEx,
    },
};

/* Decode */

#[async_trait(?Send)]
impl Decode for Medication {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["id", "meta", "extension"]);

        stream.root("Medication").await?;

        let id = stream.decode(&mut fields, decode_any).await?;
        let meta = stream.decode::<Meta, _>(&mut fields, decode_any).await?;
        let extension = stream.decode_opt(&mut fields, decode_any).await?;

        let data = match meta.profiles.get(0) {
            Some(s) if s == PROFILE_MEDICATION_COMPOUNDING => {
                let mut fields = Fields::new(&["code", "form", "amount", "ingredient"]);

                let (medication_type, code) = stream
                    .decode::<(MedicationType, Option<String>), _>(
                        &mut fields,
                        decode_codeable_concept,
                    )
                    .await?;
                let form = stream.decode(&mut fields, decode_codeable_concept).await?;
                let amount = stream.decode(&mut fields, decode_amount).await?;
                let ingredient = stream
                    .decode_vec::<Vec<PznIngredient<'static>>, _>(&mut fields, decode_ingredient)
                    .await?;

                if medication_type != MedicationType::Rezeptur {
                    return Err(DecodeError::InvalidFixedValue {
                        actual: CodeEx::code(&medication_type).into(),
                        expected: CodeEx::code(&MedicationType::Rezeptur).into(),
                        path: stream.path().into(),
                    });
                }

                let ingredient = ingredient.into_iter().map(|x| x.0.into_owned()).collect();

                Data::Compounding(CompoundingData {
                    code,
                    form,
                    amount,
                    ingredient,
                })
            }
            Some(s) if s == PROFILE_MEDICATION_INGREDIENT => {
                let mut fields = Fields::new(&["code", "form", "amount", "ingredient"]);

                let (medication_type, _code) = stream
                    .decode::<(MedicationType, Option<String>), _>(
                        &mut fields,
                        decode_codeable_concept,
                    )
                    .await?;
                let form = stream.decode(&mut fields, decode_codeable_concept).await?;
                let amount = stream.decode_opt(&mut fields, decode_amount).await?;
                let ingredient = stream
                    .decode::<AskIngredient<'static>, _>(&mut fields, decode_ingredient)
                    .await?;

                if medication_type != MedicationType::Wirkstoff {
                    return Err(DecodeError::InvalidFixedValue {
                        actual: CodeEx::code(&medication_type).into(),
                        expected: CodeEx::code(&MedicationType::Wirkstoff).into(),
                        path: stream.path().into(),
                    });
                }

                let ingredient = ingredient.0.into_owned();

                Data::Ingredient(IngredientData {
                    form,
                    amount,
                    ingredient,
                })
            }
            Some(s) if s == PROFILE_MEDICATION_FREE_TEXT => {
                let mut fields = Fields::new(&["code", "form"]);

                let (medication_type, code) = stream
                    .decode::<(MedicationType, Option<String>), _>(
                        &mut fields,
                        decode_codeable_concept,
                    )
                    .await?;
                let form = stream
                    .decode_opt(&mut fields, decode_codeable_concept)
                    .await?;

                let code = match code {
                    Some(code) => code,
                    None => {
                        return Err(DecodeError::MissingField {
                            id: Some("code").into(),
                            path: stream.path().into(),
                        })
                    }
                };

                if medication_type != MedicationType::Freitext {
                    return Err(DecodeError::InvalidFixedValue {
                        actual: CodeEx::code(&medication_type).into(),
                        expected: CodeEx::code(&MedicationType::Freitext).into(),
                        path: stream.path().into(),
                    });
                }

                Data::FreeText(FreeTextData { code, form })
            }
            Some(s) if s == PROFILE_MEDICATION_PZN => {
                let mut fields = Fields::new(&["code", "form", "amount"]);

                let code = stream.decode(&mut fields, decode_codeable_concept).await?;
                let form = stream.decode(&mut fields, decode_codeable_concept).await?;
                let amount = stream.decode_opt(&mut fields, decode_amount).await?;

                Data::Pzn(PznData { code, form, amount })
            }
            Some(_) | None => {
                return Err(DecodeError::InvalidProfile {
                    actual: meta.profiles,
                    expected: vec![
                        PROFILE_MEDICATION_COMPOUNDING.into(),
                        PROFILE_MEDICATION_FREE_TEXT.into(),
                        PROFILE_MEDICATION_INGREDIENT.into(),
                        PROFILE_MEDICATION_PZN.into(),
                    ],
                })
            }
        };

        stream.end().await?;

        Ok(Medication {
            id,
            data,
            extension,
        })
    }
}

#[async_trait(?Send)]
impl Decode for Extension {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut category = None;
        let mut vaccine = None;
        let mut instruction = None;
        let mut packaging = None;
        let mut standard_size = None;

        let mut fields = Fields::new(&["extension"]);
        while stream.begin_substream_vec(&mut fields).await? {
            stream.element().await?;

            let url = stream.value(Search::Exact("url")).await?.unwrap();

            match url.as_str() {
                URL_CATEGORY => {
                    let mut fields = Fields::new(&["valueCoding"]);

                    category = Some(stream.decode(&mut fields, decode_coding).await?);
                }
                URL_VACCINE => {
                    let mut fields = Fields::new(&["valueBoolean"]);

                    vaccine = Some(stream.decode(&mut fields, decode_any).await?);
                }
                URL_INSTRUCTION => {
                    let mut fields = Fields::new(&["valueString"]);

                    instruction = Some(stream.decode(&mut fields, decode_any).await?);
                }
                URL_PACKAGING => {
                    let mut fields = Fields::new(&["valueString"]);

                    packaging = Some(stream.decode(&mut fields, decode_any).await?);
                }
                URL_STANDARD_SIZE => {
                    let mut fields = Fields::new(&["valueCode"]);

                    standard_size = Some(stream.decode(&mut fields, decode_code).await?);
                }
                _ => (),
            }

            stream.end().await?;
            stream.end_substream().await?;
        }

        let category = category.ok_or_else(|| DecodeError::MissingExtension {
            url: URL_CATEGORY.into(),
            path: stream.path().into(),
        })?;

        let vaccine = vaccine.ok_or_else(|| DecodeError::MissingExtension {
            url: URL_VACCINE.into(),
            path: stream.path().into(),
        })?;

        Ok(Extension {
            category,
            vaccine,
            instruction,
            packaging,
            standard_size,
        })
    }
}

async fn decode_ingredient<T, S>(stream: &mut DecodeStream<S>) -> Result<T, DecodeError<S::Error>>
where
    T: IngredientWrapper<'static>,
    S: DataStream,
{
    let mut fields = Fields::new(&["extension", "itemCodeableConcept", "strength"]);

    let mut code = None;
    let mut text = None;
    let mut strength = None;
    let mut dosage_form = None;
    let mut amount_free_text = None;

    stream.element().await?;

    while stream.begin_substream_vec(&mut fields).await? {
        stream.element().await?;

        let url = stream.value(Search::Exact("url")).await?.unwrap();

        if url == URL_DOSAGE_FORM {
            let mut fields = Fields::new(&["valueString"]);

            dosage_form = Some(stream.decode(&mut fields, decode_any).await?);
        }

        stream.end().await?;
        stream.end_substream().await?;
    }

    if stream.begin_substream_opt(&mut fields).await? {
        let mut fields = Fields::new(&["coding", "text"]);

        stream.element().await?;

        if stream.begin_substream_opt(&mut fields).await? {
            let mut fields = Fields::new(&["system", "code"]);

            stream.element().await?;

            stream.fixed(&mut fields, T::system()).await?;
            code = Some(stream.decode(&mut fields, decode_any).await?);

            stream.end().await?;
            stream.end_substream().await?;
        }

        text = stream.decode_opt(&mut fields, decode_any).await?;

        stream.end().await?;
        stream.end_substream().await?;
    }

    stream.begin_substream(&mut fields).await?;
    stream.element().await?;

    {
        let mut fields = Fields::new(&["extension", "numerator", "denominator"]);

        while stream.begin_substream_vec(&mut fields).await? {
            stream.element().await?;

            let url = stream.value(Search::Exact("url")).await?.unwrap();

            if url == URL_INGREDIENT_AMOUNT {
                let mut fields = Fields::new(&["valueString"]);

                amount_free_text = Some(stream.decode(&mut fields, decode_any).await?);
            }

            stream.end().await?;
            stream.end_substream().await?;
        }

        if stream.begin_substream_opt(&mut fields).await? {
            let mut fields = Fields::new(&["value", "unit", "system"]);

            stream.element().await?;

            let value = stream.decode(&mut fields, decode_any).await?;
            let unit = stream.decode(&mut fields, decode_any).await?;
            stream
                .fixed(&mut fields, "http://unitsofmeasure.org")
                .await?;

            stream.end().await?;
            stream.end_substream().await?;

            strength = Some(Amount {
                value,
                unit,
                code: None,
            });
        }

        if stream.begin_substream_opt(&mut fields).await? {
            let mut fields = Fields::new(&["value"]);

            stream.element().await?;

            stream.fixed(&mut fields, "1").await?;

            stream.end().await?;
            stream.end_substream().await?;
        }
    }

    stream.end().await?;
    stream.end_substream().await?;

    stream.end().await?;

    Ok(T::owned(Ingredient {
        code,
        text,
        strength,
        dosage_form,
        amount_free_text,
    }))
}

/* Encode */

impl Encode for &Medication {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let meta = Meta {
            profiles: vec![match &self.data {
                Data::Compounding { .. } => PROFILE_MEDICATION_COMPOUNDING,
                Data::Ingredient { .. } => PROFILE_MEDICATION_INGREDIENT,
                Data::FreeText { .. } => PROFILE_MEDICATION_FREE_TEXT,
                Data::Pzn { .. } => PROFILE_MEDICATION_PZN,
            }
            .into()],
        };

        stream
            .root("Medication")?
            .encode("id", &self.id, encode_any)?
            .encode("meta", meta, encode_any)?
            .encode_opt("extension", &self.extension, encode_any)?;

        match &self.data {
            Data::Compounding(data) => {
                let code = (MedicationType::Rezeptur, data.code.clone());

                stream
                    .encode("code", &code, encode_codeable_concept)?
                    .encode("form", &data.form, encode_codeable_concept)?
                    .encode("amount", &data.amount, encode_amount)?
                    .encode_vec(
                        "ingredient",
                        data.ingredient.iter().map(PznIngredient::borrow),
                        encode_ingredient,
                    )?;
            }
            Data::Ingredient(data) => {
                let code = (MedicationType::Wirkstoff, None);

                stream
                    .encode("code", &code, encode_codeable_concept)?
                    .encode("form", &data.form, encode_codeable_concept)?
                    .encode_opt("amount", &data.amount, encode_amount)?
                    .encode_vec(
                        "ingredient",
                        once(&data.ingredient).map(AskIngredient::borrow),
                        encode_ingredient,
                    )?;
            }
            Data::FreeText(data) => {
                let code = (MedicationType::Freitext, Some(data.code.clone()));

                stream
                    .encode("code", &code, encode_codeable_concept)?
                    .encode_opt("form", &data.form, encode_codeable_concept)?;
            }
            Data::Pzn(data) => {
                stream
                    .encode("code", &data.code, encode_codeable_concept)?
                    .encode("form", &data.form, encode_codeable_concept)?
                    .encode_opt("amount", &data.amount, encode_amount)?;
            }
        }

        stream.end()?;

        Ok(())
    }
}

impl Encode for &Extension {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream.array()?;

        stream
            .element()?
            .attrib("url", URL_CATEGORY, encode_any)?
            .encode("valueCoding", &self.category, encode_coding)?
            .end()?;

        stream
            .element()?
            .attrib("url", URL_VACCINE, encode_any)?
            .encode("valueBoolean", &self.vaccine, encode_any)?
            .end()?;

        if let Some(instruction) = &self.instruction {
            stream
                .element()?
                .attrib("url", URL_INSTRUCTION, encode_any)?
                .encode("valueString", instruction, encode_any)?
                .end()?;
        }

        if let Some(packaging) = &self.packaging {
            stream
                .element()?
                .attrib("url", URL_PACKAGING, encode_any)?
                .encode("valueString", packaging, encode_any)?
                .end()?;
        }

        if let Some(standard_size) = &self.standard_size {
            stream
                .element()?
                .attrib("url", URL_STANDARD_SIZE, encode_any)?
                .encode("valueCode", standard_size, encode_code)?
                .end()?;
        }

        stream.end()?;

        Ok(())
    }
}

fn encode_ingredient<'a, T, S>(
    data: T,
    stream: &mut EncodeStream<S>,
) -> Result<(), EncodeError<S::Error>>
where
    T: IngredientWrapper<'a>,
    S: DataStorage,
{
    let data = data.get();

    stream.element()?.field_name("extension")?.array()?;

    if let Some(dosage_form) = &data.dosage_form {
        stream
            .element()?
            .attrib("url", URL_DOSAGE_FORM, encode_any)?
            .encode("valueString", dosage_form, encode_any)?
            .end()?;
    }

    stream.end()?.field_name("itemCodeableConcept")?.element()?;

    if let Some(code) = &data.code {
        stream
            .field_name("coding")?
            .array()?
            .element()?
            .encode("system", T::system(), encode_any)?
            .encode("code", code, encode_any)?
            .end()?
            .end()?;
    }
    stream
        .encode_opt("text", &data.text, encode_any)?
        .end()?
        .field_name("strength")?
        .element()?
        .field_name("extension")?
        .array()?;

    if let Some(amount_free_text) = &data.amount_free_text {
        stream
            .element()?
            .attrib("url", URL_INGREDIENT_AMOUNT, encode_any)?
            .encode("valueString", amount_free_text, encode_any)?
            .end()?;
    }

    stream.end()?;

    if let Some(strength) = &data.strength {
        stream
            .field_name("numerator")?
            .element()?
            .encode("value", &strength.value, encode_any)?
            .encode("unit", &strength.unit, encode_any)?
            .encode("system", "http://unitsofmeasure.org", encode_any)?
            .end()?
            .field_name("denominator")?
            .element()?
            .encode("value", 1usize, encode_any)?
            .end()?;
    }

    stream.end()?.end()?;

    Ok(())
}

/* Misc */

impl CodeEx for Category {
    fn from_parts(value: String) -> Result<Self, String> {
        match value.as_str() {
            "00" => Ok(Self::Medicine),
            "01" => Ok(Self::BTM),
            "02" => Ok(Self::AMVV),
            _ => Err(value),
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Medicine => "00",
            Self::BTM => "01",
            Self::AMVV => "02",
        }
    }
}

impl CodingEx for Category {
    type Code = Self;

    fn from_parts(code: Self::Code) -> Self {
        code
    }

    fn code(&self) -> &Self::Code {
        &self
    }

    fn display(&self) -> Option<&'static str> {
        None
    }

    fn system() -> Option<&'static str> {
        Some("https://fhir.kbv.de/CodeSystem/KBV_CS_ERP_Medication_Category")
    }
}

impl CodeEx for StandardSize {
    fn from_parts(value: String) -> Result<Self, String> {
        match value.as_str() {
            "N1" => Ok(Self::N1),
            "N2" => Ok(Self::N2),
            "N3" => Ok(Self::N3),
            "KTP" => Ok(Self::KTP),
            "KA" => Ok(Self::KA),
            "NB" => Ok(Self::NB),
            "Sonstiges" => Ok(Self::Other),
            _ => Err(value),
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::N1 => "N1",
            Self::N2 => "N2",
            Self::N3 => "N3",
            Self::KTP => "KTP",
            Self::KA => "KA",
            Self::NB => "NB",
            Self::Other => "Sonstiges",
        }
    }
}

#[async_trait(?Send)]
impl CodeableConcept for PznCode {
    async fn decode_codeable_concept<S>(
        stream: &mut DecodeStream<S>,
    ) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["coding", "text"]);

        stream.element().await?;

        stream.begin_substream(&mut fields).await?;
        stream.element().await?;

        let code = {
            let mut fields = Fields::new(&["system", "code"]);

            let _system = stream.fixed(&mut fields, SYSTEM_PZN).await?;
            let code = stream.decode(&mut fields, decode_any).await?;

            code
        };

        stream.end().await?;
        stream.end_substream().await?;

        let text = stream.decode(&mut fields, decode_any).await?;

        stream.end().await?;

        Ok(PznCode { text, code })
    }

    fn encode_codeable_concept<S>(
        &self,
        stream: &mut EncodeStream<S>,
    ) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .field_name("coding")?
            .array()?
            .element()?
            .encode("system", SYSTEM_PZN, encode_any)?
            .encode("code", &self.code, encode_any)?
            .end()?
            .end()?
            .encode("text", &self.text, encode_any)?
            .end()?;

        Ok(())
    }
}

#[async_trait(?Send)]
impl CodeableConcept for PznForm {
    async fn decode_codeable_concept<S>(
        stream: &mut DecodeStream<S>,
    ) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        stream.element().await?;

        stream
            .begin_substream(&mut Fields::new(&["coding"]))
            .await?;
        stream.element().await?;

        let mut fields = Fields::new(&["system", "code"]);

        let system = stream.decode(&mut fields, decode_any).await?;
        let code = stream.decode(&mut fields, decode_any).await?;

        stream.end().await?;
        stream.end_substream().await?;

        stream.end().await?;

        Ok(PznForm { system, code })
    }

    fn encode_codeable_concept<S>(
        &self,
        stream: &mut EncodeStream<S>,
    ) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream
            .element()?
            .field_name("coding")?
            .array()?
            .element()?
            .encode("system", &self.system, encode_any)?
            .encode("code", &self.code, encode_any)?
            .end()?
            .end()?
            .end()?;

        Ok(())
    }
}

pub trait IngredientWrapper<'a> {
    fn owned(v: Ingredient) -> Self;
    fn borrow(v: &'a Ingredient) -> Self;
    fn get(&self) -> &Ingredient;
    fn system() -> &'static str;
}

struct PznIngredient<'a>(Cow<'a, Ingredient>);
struct AskIngredient<'a>(Cow<'a, Ingredient>);

impl<'a> IngredientWrapper<'a> for PznIngredient<'a> {
    fn owned(v: Ingredient) -> Self {
        Self(Cow::Owned(v))
    }

    fn borrow(v: &'a Ingredient) -> Self {
        Self(Cow::Borrowed(v))
    }

    fn get(&self) -> &Ingredient {
        &self.0
    }

    fn system() -> &'static str {
        SYSTEM_PZN
    }
}

impl<'a> IngredientWrapper<'a> for AskIngredient<'a> {
    fn owned(v: Ingredient) -> Self {
        Self(Cow::Owned(v))
    }

    fn borrow(v: &'a Ingredient) -> Self {
        Self(Cow::Borrowed(v))
    }

    fn get(&self) -> &Ingredient {
        &self.0
    }

    fn system() -> &'static str {
        SYSTEM_ASK
    }
}

#[derive(Debug, PartialEq)]
enum MedicationType {
    Wirkstoff,
    Freitext,
    Rezeptur,
}

#[async_trait(?Send)]
impl CodeEx for MedicationType {
    fn from_parts(value: String) -> Result<Self, String> {
        match value.as_str() {
            "wirkstoff" => Ok(Self::Wirkstoff),
            "freitext" => Ok(Self::Freitext),
            "rezeptur" => Ok(Self::Rezeptur),
            _ => Err(value),
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Wirkstoff => "wirkstoff",
            Self::Freitext => "freitext",
            Self::Rezeptur => "rezeptur",
        }
    }
}

impl CodingEx for MedicationType {
    type Code = Self;

    fn from_parts(code: Self::Code) -> Self {
        code
    }

    fn code(&self) -> &Self::Code {
        &self
    }

    fn system() -> Option<&'static str> {
        Some("https://fhir.kbv.de/CodeSystem/KBV_CS_ERP_Medication_Type")
    }
}

impl CodeableConceptEx for (MedicationType, Option<String>) {
    type Coding = MedicationType;

    fn from_parts(coding: Self::Coding, text: Option<String>) -> Self {
        (coding, text)
    }

    fn coding(&self) -> &Self::Coding {
        &self.0
    }

    fn text(&self) -> &Option<String> {
        &self.1
    }
}

impl AmountEx for Amount {
    fn from_parts(value: usize, _denominator: usize, unit: String, code: Option<String>) -> Self {
        Self { value, unit, code }
    }

    fn unit(&self) -> &String {
        &self.unit
    }

    fn numerator(&self) -> usize {
        self.value
    }

    fn code(&self) -> &Option<String> {
        &self.code
    }

    fn system() -> Option<&'static str> {
        Some("http://unitsofmeasure.org")
    }
}

const PROFILE_MEDICATION_COMPOUNDING: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_PR_ERP_Medication_Compounding|1.00.000";
const PROFILE_MEDICATION_FREE_TEXT: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_PR_ERP_Medication_FreeText|1.00.000";
const PROFILE_MEDICATION_INGREDIENT: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_PR_ERP_Medication_Ingredient|1.00.000";
const PROFILE_MEDICATION_PZN: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_PR_ERP_Medication_PZN|1.00.000";

const URL_CATEGORY: &str = "https://fhir.kbv.de/StructureDefinition/KBV_EX_ERP_Medication_Category";
const URL_VACCINE: &str = "https://fhir.kbv.de/StructureDefinition/KBV_EX_ERP_Medication_Vaccine";
const URL_INSTRUCTION: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_EX_ERP_Medication_compoundingInstruction";
const URL_PACKAGING: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_EX_ERP_Medication_Packaging";
const URL_STANDARD_SIZE: &str = "http://fhir.de/StructureDefinition/normgroesse";
const URL_DOSAGE_FORM: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_EX_ERP_Medication_Ingredient_Form";
const URL_INGREDIENT_AMOUNT: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_EX_ERP_Medication_Ingredient_Amount";

const SYSTEM_PZN: &str = "https://fhir.kbv.de/NamingSystem/KBV_NS_ERP_PZN";
const SYSTEM_ASK: &str = "https://fhir.kbv.de/NamingSystem/KBV_NS_ERP_ASK";

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
    async fn test_decode_json_medication_compounding() {
        let mut stream = load_stream("./examples/medication_compounding.json");

        let actual: Medication = stream.json().await.unwrap();
        let expected = test_medication_compounding();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_decode_xml_medication_compounding() {
        let mut stream = load_stream("./examples/medication_compounding.xml");

        let actual: Medication = stream.xml().await.unwrap();
        let expected = test_medication_compounding();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_encode_json_medication_compounding() {
        let value = test_medication_compounding();

        let actual = (&value).json().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/medication_compounding.json").unwrap();

        assert_eq!(trim_json_str(&actual), trim_json_str(&expected));
    }

    #[tokio::test]
    async fn test_encode_xml_medication_compounding() {
        let value = test_medication_compounding();

        let actual = (&value).xml().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/medication_compounding.xml").unwrap();

        assert_eq!(trim_xml_str(&actual), trim_xml_str(&expected));
    }

    #[tokio::test]
    async fn test_decode_json_medication_free_text() {
        let mut stream = load_stream("./examples/medication_free_text.json");

        let actual: Medication = stream.json().await.unwrap();
        let expected = test_medication_free_text();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_decode_xml_medication_free_text() {
        let mut stream = load_stream("./examples/medication_free_text.xml");

        let actual: Medication = stream.xml().await.unwrap();
        let expected = test_medication_free_text();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_encode_json_medication_free_text() {
        let value = test_medication_free_text();

        let actual = (&value).json().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/medication_free_text.json").unwrap();

        assert_eq!(trim_json_str(&actual), trim_json_str(&expected));
    }

    #[tokio::test]
    async fn test_encode_xml_medication_free_text() {
        let value = test_medication_free_text();

        let actual = (&value).xml().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/medication_free_text.xml").unwrap();

        assert_eq!(trim_xml_str(&actual), trim_xml_str(&expected));
    }

    #[tokio::test]
    async fn test_decode_json_medication_ingredient() {
        let mut stream = load_stream("./examples/medication_ingredient.json");

        let actual: Medication = stream.json().await.unwrap();
        let expected = test_medication_ingredient();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_decode_xml_medication_ingredient() {
        let mut stream = load_stream("./examples/medication_ingredient.xml");

        let actual: Medication = stream.xml().await.unwrap();
        let expected = test_medication_ingredient();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_encode_json_medication_ingredient() {
        let value = test_medication_ingredient();

        let actual = (&value).json().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/medication_ingredient.json").unwrap();

        assert_eq!(trim_json_str(&actual), trim_json_str(&expected));
    }

    #[tokio::test]
    async fn test_encode_xml_medication_ingredient() {
        let value = test_medication_ingredient();

        let actual = (&value).xml().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/medication_ingredient.xml").unwrap();

        assert_eq!(trim_xml_str(&actual), trim_xml_str(&expected));
    }

    #[tokio::test]
    async fn test_decode_json_medication_pzn() {
        let mut stream = load_stream("./examples/medication_pzn.json");

        let actual: Medication = stream.json().await.unwrap();
        let expected = test_medication_pzn();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_decode_xml_medication_pzn() {
        let mut stream = load_stream("./examples/medication_pzn.xml");

        let actual: Medication = stream.xml().await.unwrap();
        let expected = test_medication_pzn();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_encode_json_medication_pzn() {
        let value = test_medication_pzn();

        let actual = (&value).json().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/medication_pzn.json").unwrap();

        assert_eq!(trim_json_str(&actual), trim_json_str(&expected));
    }

    #[tokio::test]
    async fn test_encode_xml_medication_pzn() {
        let value = test_medication_pzn();

        let actual = (&value).xml().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/medication_pzn.xml").unwrap();

        assert_eq!(trim_xml_str(&actual), trim_xml_str(&expected));
    }

    pub fn test_medication_compounding() -> Medication {
        Medication {
            id: "cabe8dc4-e1b7-4d2a-bc2d-9443167f88d8".try_into().unwrap(),
            data: Data::Compounding(CompoundingData {
                code: Some("Dummy-Rezeptur".into()),
                form: "Salbe".into(),
                amount: Amount {
                    value: 100,
                    unit: "ml".into(),
                    code: None,
                },
                ingredient: vec![
                    Ingredient {
                        code: Some("09703312".into()),
                        text: Some("Hydrocortison ratiopharm 0,5%".into()),
                        strength: Some(Amount {
                            value: 30,
                            unit: "g".into(),
                            code: None,
                        }),
                        dosage_form: None,
                        amount_free_text: None,
                    },
                    Ingredient {
                        code: None,
                        text: Some("weiterer Dummy-Freitextbestandteil".into()),
                        strength: None,
                        dosage_form: Some("Freitextdarreichungsform".into()),
                        amount_free_text: Some("quantum satis".into()),
                    },
                ],
            }),
            extension: Some(Extension {
                category: Category::Medicine,
                vaccine: false,
                standard_size: None,
                instruction: Some("Dummy-Herstellungsanweisung einer Rezeptur".into()),
                packaging: Some("Flasche".into()),
            }),
        }
    }

    pub fn test_medication_free_text() -> Medication {
        Medication {
            id: "a0553ad5-56bc-446c-91de-70f0260b4e7a".try_into().unwrap(),
            data: Data::FreeText(FreeTextData {
                code: "Dummy-Impfstoff als Freitext".into(),
                form: None,
            }),
            extension: Some(Extension {
                category: Category::Medicine,
                vaccine: true,
                standard_size: None,
                instruction: None,
                packaging: None,
            }),
        }
    }

    pub fn test_medication_ingredient() -> Medication {
        Medication {
            id: "e3a4efa7-84fc-465b-b14c-720195097783".try_into().unwrap(),
            data: Data::Ingredient(IngredientData {
                form: "Tabletten".into(),
                amount: Some(Amount {
                    value: 20,
                    unit: "Stk".into(),
                    code: None,
                }),
                ingredient: Ingredient {
                    code: Some("Dummy-ASK".into()),
                    text: Some("Ibuprofen".into()),
                    strength: Some(Amount {
                        value: 800,
                        unit: "mg".into(),
                        code: None,
                    }),
                    dosage_form: None,
                    amount_free_text: None,
                },
            }),
            extension: Some(Extension {
                category: Category::Medicine,
                vaccine: false,
                standard_size: None,
                instruction: None,
                packaging: None,
            }),
        }
    }

    pub fn test_medication_pzn() -> Medication {
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
                standard_size: Some(StandardSize::N1),
                instruction: None,
                packaging: None,
            }),
        }
    }
}
