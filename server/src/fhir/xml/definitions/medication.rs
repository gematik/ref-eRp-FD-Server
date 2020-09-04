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
use std::ops::Deref;

use resources::{
    medication::{
        Amount, Category, CompoundingData, Data, Extension, FreeTextData, Ingredient,
        IngredientData, PznCode, PznData, PznForm, StandardSize,
    },
    misc::{Decode, DecodeStr, EncodeStr},
    primitives::Id,
    Medication,
};
use serde::{
    de::Error as DeError, ser::Error as SerError, Deserialize, Deserializer, Serialize, Serializer,
};

use super::{
    super::super::constants::{
        CODING_SYSTEM_ASK, CODING_SYSTEM_MEDICATION_CATEGORY, CODING_SYSTEM_MEDICATION_TYPE,
        CODING_SYSTEM_PZN, EXTENSION_URL_MEDICATION_CATEGORY,
        EXTENSION_URL_MEDICATION_INGREDIENT_AMOUNT, EXTENSION_URL_MEDICATION_INGREDIENT_FORM,
        EXTENSION_URL_MEDICATION_INSTRUCTION, EXTENSION_URL_MEDICATION_PACKAGING,
        EXTENSION_URL_MEDICATION_VACCINE, EXTENSION_URL_STANDARD_SIZE,
        MEDICATION_TYPE_CODE_COMPOUNDING, MEDICATION_TYPE_CODE_FREE_TEXT,
        MEDICATION_TYPE_CODE_INGREDIENT, QUANTITY_SYSTEM_MEDICATION,
        RESOURCE_PROFILE_MEDICATION_COMPOUNDING, RESOURCE_PROFILE_MEDICATION_FREE_TEXT,
        RESOURCE_PROFILE_MEDICATION_INGREDIENT, RESOURCE_PROFILE_MEDICATION_PZN, XMLNS_MEDICATION,
    },
    misc::{
        CodableConceptDef, CodingDef, DeserializeRoot, ExtensionDef, MetaDef, QuantityDef,
        RatioDef, SerializeRoot, ValueDef, XmlnsType,
    },
    primitives::IdDef,
};

pub struct MedicationDef;

#[derive(Serialize, Deserialize)]
#[serde(rename = "Medication")]
pub struct MedicationCow<'a>(#[serde(with = "MedicationDef")] Cow<'a, Medication>);

#[serde(rename = "Medication")]
#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize)]
struct MedicationHelper {
    #[serde(with = "IdDef")]
    id: Id,

    meta: MetaDef,

    #[serde(default)]
    extension: Vec<ExtensionDef>,

    code: CodableConceptDef,

    #[serde(default)]
    form: Option<CodableConceptDef>,

    #[serde(default)]
    amount: Option<RatioDef>,

    #[serde(default)]
    ingredient: Vec<IngredientDef>,
}

#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize, Default)]
struct IngredientDef {
    #[serde(default)]
    extension: Vec<ExtensionDef>,

    item_codeable_concept: Option<CodableConceptDef>,

    strength: RatioDef,
}

struct DataHelper {
    code: CodableConceptDef,
    form: Option<CodableConceptDef>,
    amount: Option<RatioDef>,
    ingredient: Vec<IngredientDef>,
}

impl XmlnsType for Medication {
    fn xmlns() -> &'static str {
        XMLNS_MEDICATION
    }
}

impl<'a> SerializeRoot<'a> for MedicationCow<'a> {
    type Inner = Medication;

    fn from_inner(inner: &'a Self::Inner) -> Self {
        MedicationCow(Cow::Borrowed(inner))
    }
}

impl DeserializeRoot for MedicationCow<'_> {
    type Inner = Medication;

    fn into_inner(self) -> Self::Inner {
        self.0.into_owned()
    }
}

impl MedicationDef {
    pub fn serialize<S: Serializer>(
        medication: &Medication,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let value: MedicationHelper = medication.try_into().map_err(S::Error::custom)?;

        value.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Cow<'static, Medication>, D::Error> {
        let value = MedicationHelper::deserialize(deserializer)?;

        Ok(Cow::Owned(value.try_into().map_err(D::Error::custom)?))
    }
}

impl TryInto<MedicationHelper> for &Medication {
    type Error = String;

    fn try_into(self) -> Result<MedicationHelper, Self::Error> {
        Ok(MedicationHelper {
            id: self.id.clone(),
            meta: serialize_meta(self),
            extension: serialize_extension(self)?,
            code: serialize_code(self),
            form: serialize_form(self),
            amount: serialize_amount(self),
            ingredient: serialize_ingredient(self)?,
        })
    }
}

impl TryInto<Medication> for MedicationHelper {
    type Error = String;

    fn try_into(self) -> Result<Medication, Self::Error> {
        let profile = self
            .meta
            .profile
            .into_iter()
            .next()
            .ok_or_else(|| "Medication meta is missing the `profile` field!")?;
        let profile = profile.deref();

        let data = DataHelper {
            code: self.code,
            form: self.form,
            amount: self.amount,
            ingredient: self.ingredient,
        };

        let mut accepted_extensions = Vec::new();

        Ok(Medication {
            id: self.id,
            data: deserialize_data(profile, data, &mut accepted_extensions)?,
            extension: deserialize_extension(self.extension, &accepted_extensions)?,
        })
    }
}

fn serialize_meta(value: &Medication) -> MetaDef {
    MetaDef {
        profile: vec![match value.data {
            Data::Compounding(_) => RESOURCE_PROFILE_MEDICATION_COMPOUNDING,
            Data::FreeText(_) => RESOURCE_PROFILE_MEDICATION_FREE_TEXT,
            Data::Ingredient(_) => RESOURCE_PROFILE_MEDICATION_INGREDIENT,
            Data::Pzn(_) => RESOURCE_PROFILE_MEDICATION_PZN,
        }
        .into()],
        ..Default::default()
    }
}

fn serialize_extension(value: &Medication) -> Result<Vec<ExtensionDef>, String> {
    let extension = match &value.extension {
        Some(extension) => extension,
        None => return Ok(Vec::new()),
    };

    let mut result = vec![
        ExtensionDef {
            url: EXTENSION_URL_MEDICATION_CATEGORY.into(),
            value: Some(ValueDef::Coding(CodingDef {
                system: Some(CODING_SYSTEM_MEDICATION_CATEGORY.into()),
                code: Some(extension.category.encode_str()),
                ..Default::default()
            })),
            ..Default::default()
        },
        ExtensionDef {
            url: EXTENSION_URL_MEDICATION_VACCINE.into(),
            value: Some(ValueDef::Boolean(extension.vaccine.into())),
            ..Default::default()
        },
    ];

    let mut instruction = extension.instruction.is_some();
    let mut packaging = extension.packaging.is_some();
    let mut standard_size = extension.standard_size.is_some();

    match value.data {
        Data::Compounding(_) => {
            instruction = false;
            packaging = false;
        }
        Data::Pzn(_) => {
            standard_size = false;
        }
        _ => (),
    }

    if instruction {
        return Err("Instruction extension is not allowed for this medication!".to_owned());
    }

    if packaging {
        return Err("Packing extension is not allowed for this medication!".to_owned());
    }

    if standard_size {
        return Err("Standard size extension is not allowed for this medication!".to_owned());
    }

    if let Some(instruction) = &extension.instruction {
        result.push(ExtensionDef {
            url: EXTENSION_URL_MEDICATION_INSTRUCTION.into(),
            value: Some(ValueDef::String(instruction.clone().into())),
            ..Default::default()
        });
    }

    if let Some(packaging) = &extension.packaging {
        result.push(ExtensionDef {
            url: EXTENSION_URL_MEDICATION_PACKAGING.into(),
            value: Some(ValueDef::String(packaging.clone().into())),
            ..Default::default()
        });
    }

    if let Some(standard_size) = &extension.standard_size {
        result.push(ExtensionDef {
            url: EXTENSION_URL_STANDARD_SIZE.into(),
            value: Some(ValueDef::Code(standard_size.encode_str().into())),
            ..Default::default()
        });
    }

    Ok(result)
}

fn serialize_code(value: &Medication) -> CodableConceptDef {
    match &value.data {
        Data::Compounding(data) => CodableConceptDef {
            coding: vec![CodingDef {
                system: Some(CODING_SYSTEM_MEDICATION_TYPE.into()),
                code: Some(MEDICATION_TYPE_CODE_COMPOUNDING.into()),
                ..Default::default()
            }],
            text: data.code.clone(),
        },
        Data::FreeText(data) => CodableConceptDef {
            coding: vec![CodingDef {
                system: Some(CODING_SYSTEM_MEDICATION_TYPE.into()),
                code: Some(MEDICATION_TYPE_CODE_FREE_TEXT.into()),
                ..Default::default()
            }],
            text: Some(data.code.clone()),
        },
        Data::Ingredient(_) => CodableConceptDef {
            coding: vec![CodingDef {
                system: Some(CODING_SYSTEM_MEDICATION_TYPE.into()),
                code: Some(MEDICATION_TYPE_CODE_INGREDIENT.into()),
                ..Default::default()
            }],
            ..Default::default()
        },
        Data::Pzn(data) => CodableConceptDef {
            coding: vec![CodingDef {
                system: Some(CODING_SYSTEM_PZN.into()),
                code: Some(data.code.code.clone()),
                ..Default::default()
            }],
            text: Some(data.code.text.clone()),
        },
    }
}

fn serialize_form(value: &Medication) -> Option<CodableConceptDef> {
    match &value.data {
        Data::Compounding(data) => Some(CodableConceptDef {
            text: Some(data.form.clone()),
            ..Default::default()
        }),
        Data::FreeText(data) => data.form.as_ref().map(|form| CodableConceptDef {
            text: Some(form.clone()),
            ..Default::default()
        }),
        Data::Ingredient(data) => Some(CodableConceptDef {
            text: Some(data.form.clone()),
            ..Default::default()
        }),
        Data::Pzn(data) => Some(CodableConceptDef {
            coding: vec![CodingDef {
                system: Some(data.form.system.clone()),
                code: Some(data.form.code.clone()),
                ..Default::default()
            }],
            ..Default::default()
        }),
    }
}

fn serialize_amount(value: &Medication) -> Option<RatioDef> {
    match &value.data {
        Data::Compounding(data) => Some(RatioDef {
            numerator: Some(QuantityDef {
                value: Some(data.amount.value),
                unit: Some(data.amount.unit.clone()),
                system: Some(QUANTITY_SYSTEM_MEDICATION.into()),
                code: data.amount.code.clone(),
            }),
            denominator: Some(QuantityDef {
                value: Some(1),
                ..Default::default()
            }),
            ..Default::default()
        }),
        Data::FreeText(_) => None,
        Data::Ingredient(IngredientData { amount, .. }) | Data::Pzn(PznData { amount, .. }) => {
            amount.as_ref().map(|amount| RatioDef {
                numerator: Some(QuantityDef {
                    value: Some(amount.value),
                    unit: Some(amount.unit.clone()),
                    system: Some(QUANTITY_SYSTEM_MEDICATION.into()),
                    code: amount.code.clone(),
                }),
                denominator: Some(QuantityDef {
                    value: Some(1),
                    ..Default::default()
                }),
                ..Default::default()
            })
        }
    }
}

fn serialize_ingredient(value: &Medication) -> Result<Vec<IngredientDef>, String> {
    Ok(match &value.data {
        Data::Compounding(data) => data
            .ingredient
            .iter()
            .map(|ingredient| {
                let mut extension = Vec::new();
                let mut strength = RatioDef::default();

                let item_codeable_concept =
                    match (ingredient.code.as_ref(), ingredient.text.as_ref()) {
                        (None, None) => None,
                        (None, Some(text)) => Some(CodableConceptDef {
                            coding: vec![],
                            text: Some(text.clone()),
                        }),
                        (Some(code), text) => Some(CodableConceptDef {
                            coding: vec![CodingDef {
                                system: Some(CODING_SYSTEM_PZN.into()),
                                code: Some(code.clone()),
                                ..Default::default()
                            }],
                            text: text.map(Clone::clone),
                        }),
                    };

                if let Some(s) = &ingredient.strength {
                    strength.numerator = Some(QuantityDef {
                        system: Some(QUANTITY_SYSTEM_MEDICATION.into()),
                        value: Some(s.value),
                        unit: Some(s.unit.clone()),
                        code: s.code.clone(),
                    });

                    strength.denominator = Some(QuantityDef {
                        value: Some(1),
                        ..Default::default()
                    });
                }

                if let Some(dosage_form) = &ingredient.dosage_form {
                    extension.push(ExtensionDef {
                        url: EXTENSION_URL_MEDICATION_INGREDIENT_FORM.into(),
                        value: Some(ValueDef::String(dosage_form.clone().into())),
                        ..Default::default()
                    })
                }

                if let Some(amount_free_text) = &ingredient.amount_free_text {
                    strength.extension.push(ExtensionDef {
                        url: EXTENSION_URL_MEDICATION_INGREDIENT_AMOUNT.into(),
                        value: Some(ValueDef::String(amount_free_text.clone().into())),
                        ..Default::default()
                    })
                }

                IngredientDef {
                    extension,
                    item_codeable_concept,
                    strength,
                }
            })
            .collect(),
        Data::Ingredient(data) => vec![IngredientDef {
            item_codeable_concept: Some(CodableConceptDef {
                coding: data
                    .ingredient
                    .code
                    .iter()
                    .map(|code| CodingDef {
                        system: Some(CODING_SYSTEM_ASK.into()),
                        code: Some(code.clone()),
                        ..Default::default()
                    })
                    .collect(),
                text: Some(
                    data.ingredient
                        .text
                        .as_ref()
                        .map(Clone::clone)
                        .ok_or_else(|| "Ingredient text is not set!")?,
                ),
            }),
            strength: data
                .ingredient
                .strength
                .as_ref()
                .map(|strength| RatioDef {
                    numerator: Some(QuantityDef {
                        value: Some(strength.value),
                        unit: Some(strength.unit.clone()),
                        code: strength.code.as_ref().map(Clone::clone),
                        system: Some(QUANTITY_SYSTEM_MEDICATION.into()),
                    }),
                    denominator: Some(QuantityDef {
                        value: Some(1),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .ok_or_else(|| "Ingredient strength is not set!")?,
            ..Default::default()
        }],
        _ => Vec::new(),
    })
}

fn deserialize_data(
    profile: &str,
    v: DataHelper,
    accepted_extensions: &mut Vec<&str>,
) -> Result<Data, String> {
    accepted_extensions.push(EXTENSION_URL_MEDICATION_CATEGORY);
    accepted_extensions.push(EXTENSION_URL_MEDICATION_VACCINE);

    match profile {
        RESOURCE_PROFILE_MEDICATION_COMPOUNDING => {
            accepted_extensions.push(EXTENSION_URL_MEDICATION_INSTRUCTION);
            accepted_extensions.push(EXTENSION_URL_MEDICATION_PACKAGING);

            Ok(Data::Compounding(deserialize_medication_compounding(v)?))
        }
        RESOURCE_PROFILE_MEDICATION_FREE_TEXT => {
            Ok(Data::FreeText(deserialize_medication_free_text(v)?))
        }
        RESOURCE_PROFILE_MEDICATION_INGREDIENT => {
            accepted_extensions.push(EXTENSION_URL_STANDARD_SIZE);

            Ok(Data::Ingredient(deserialize_medication_ingredient(v)?))
        }
        RESOURCE_PROFILE_MEDICATION_PZN => {
            accepted_extensions.push(EXTENSION_URL_STANDARD_SIZE);

            Ok(Data::Pzn(deserialize_medication_pzn(v)?))
        }
        _ => Err("Medication has unexpected profile!".to_owned()),
    }
}

fn deserialize_medication_compounding(v: DataHelper) -> Result<CompoundingData, String> {
    /* code */
    let code = v.code.text;
    let coding = match v.code.coding.into_iter().next() {
        Some(coding) => coding,
        None => return Err("Medication is missing the `code` field!".to_owned()),
    };

    match coding.system {
        Some(s) if s == CODING_SYSTEM_MEDICATION_TYPE => (),
        Some(_) => return Err("Code coding has invalid system!".to_owned()),
        None => return Err("Code coding is missing the `system` field!".to_owned()),
    }

    match coding.code {
        Some(s) if s == MEDICATION_TYPE_CODE_COMPOUNDING => (),
        Some(_) => return Err("Code coding has invalid code!".to_owned()),
        None => return Err("Code coding is missing the `code` field!".to_owned()),
    }

    /* form */
    let form = match v.form.map(|f| f.text) {
        None => return Err("Medication is missing the `form` field!".to_owned()),
        Some(None) => return Err("Form is missing the `text` field!".to_owned()),
        Some(Some(value)) => value,
    };

    /* amount */
    let amount = v
        .amount
        .ok_or_else(|| "Medication is missing the `amount` field!")?;
    let amount = deserialize_amount(amount)?;

    /* ingredient */
    let ingredient = v
        .ingredient
        .into_iter()
        .map(|ingredient| deserialize_ingredient(ingredient, true))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(CompoundingData {
        code,
        form,
        amount,
        ingredient,
    })
}

fn deserialize_medication_free_text(v: DataHelper) -> Result<FreeTextData, String> {
    /* code */
    let code = v.code.text;
    let coding = match v.code.coding.into_iter().next() {
        Some(coding) => coding,
        None => return Err("Medication is missing the `code` field!".to_owned()),
    };

    match coding.system {
        Some(s) if s == CODING_SYSTEM_MEDICATION_TYPE => (),
        Some(_) => return Err("Code coding has invalid system!".to_owned()),
        None => return Err("Code coding is missing the `system` field!".to_owned()),
    }

    match coding.code {
        Some(s) if s == MEDICATION_TYPE_CODE_FREE_TEXT => (),
        Some(_) => return Err("Code coding has invalid code!".to_owned()),
        None => return Err("Code coding is missing the `code` field!".to_owned()),
    }

    /* form */
    let form = v.form.and_then(|f| f.text);

    Ok(FreeTextData {
        code: code.ok_or_else(|| "Medication code is missing the `text` field!")?,
        form,
    })
}

fn deserialize_medication_ingredient(v: DataHelper) -> Result<IngredientData, String> {
    /* code */
    let coding = match v.code.coding.into_iter().next() {
        Some(coding) => coding,
        None => return Err("Medication is missing the `code` field!".to_owned()),
    };

    match coding.system {
        Some(s) if s == CODING_SYSTEM_MEDICATION_TYPE => (),
        Some(_) => return Err("Code coding has invalid system!".to_owned()),
        None => return Err("Code coding is missing the `system` field!".to_owned()),
    }

    match coding.code {
        Some(s) if s == MEDICATION_TYPE_CODE_INGREDIENT => (),
        Some(_) => return Err("Code coding has invalid code!".to_owned()),
        None => return Err("Code coding is missing the `code` field!".to_owned()),
    }

    /* form */
    let form = match v.form.map(|f| f.text) {
        None => return Err("Medication is missing the `form` field!".to_owned()),
        Some(None) => return Err("Form is missing the `text` field!".to_owned()),
        Some(Some(value)) => value,
    };

    /* amount */
    let amount = v.amount.map(deserialize_amount).transpose()?;

    /* ingredient */
    let ingredient = v
        .ingredient
        .into_iter()
        .map(|ingredient| deserialize_ingredient(ingredient, false))
        .next()
        .transpose()?
        .ok_or_else(|| "Medication is missing the `ingredient` field!")?;

    Ok(IngredientData {
        form,
        amount,
        ingredient,
    })
}

fn deserialize_medication_pzn(v: DataHelper) -> Result<PznData, String> {
    /* code */
    let coding = match v.code.coding.into_iter().next() {
        Some(coding) => coding,
        None => return Err("Medication is missing the `code` field!".to_owned()),
    };

    match coding.system {
        Some(s) if s == CODING_SYSTEM_PZN => (),
        Some(_) => return Err("Code coding has invalid system!".to_owned()),
        None => return Err("Code coding is missing the `system` field!".to_owned()),
    }

    let code = match coding.code {
        Some(code) => code,
        None => return Err("Code coding is missing the `code` field!".to_owned()),
    };
    let text = v
        .code
        .text
        .ok_or_else(|| "Medication code is missing the `text` field!".to_owned())?;

    /* form */
    let form = match v.form {
        Some(form) => {
            let coding = form
                .coding
                .into_iter()
                .next()
                .ok_or_else(|| "Medication form is missing the `coding` field!")?;

            PznForm {
                system: coding
                    .system
                    .ok_or_else(|| "Medication form coding is missing the `system` field!")?,
                code: coding
                    .code
                    .ok_or_else(|| "Medication form coding is missing the `code` field!")?,
            }
        }
        None => return Err("Medication is missing the `form` field!".to_owned()),
    };

    /* amount */
    let amount = v.amount.map(deserialize_amount).transpose()?;

    Ok(PznData {
        code: PznCode { text, code },
        form,
        amount,
    })
}

fn deserialize_amount(amount: RatioDef) -> Result<Amount, String> {
    let numerator = amount
        .numerator
        .ok_or_else(|| "Amount is missing the `numerator` field!")?;
    let denominator = amount
        .denominator
        .ok_or_else(|| "Amount is missing the `denominator` field!")?;

    match denominator.value {
        Some(1) => (),
        Some(_) => return Err("Amount denominator has unexpected value!".to_owned()),
        None => return Err("Amount denominator is missing the `value` field!".to_owned()),
    }

    match numerator.system {
        Some(s) if s == QUANTITY_SYSTEM_MEDICATION => (),
        Some(_) => return Err("Numerator has invalid system!".to_owned()),
        None => return Err("Numerator is missing the `system` field!".to_owned()),
    }

    Ok(Amount {
        value: numerator
            .value
            .ok_or_else(|| "Numerator is missing the `value` field!")?,
        unit: numerator
            .unit
            .ok_or_else(|| "Numerator is missing the `unit` field!")?,
        code: numerator.code,
    })
}

fn deserialize_ingredient(
    ingredient: IngredientDef,
    is_compounding: bool,
) -> Result<Ingredient, String> {
    let mut code = None;
    let mut text = None;

    if let Some(item) = ingredient.item_codeable_concept {
        if let Some(coding) = item.coding.into_iter().next() {
            match coding.system {
                Some(s) if is_compounding && s == CODING_SYSTEM_PZN => (),
                Some(s) if !is_compounding && s == CODING_SYSTEM_ASK => (),
                Some(_) => {
                    return Err(
                        "Ingredient codable concept coding has unexpected system!".to_owned()
                    )
                }
                None => {
                    return Err(
                        "Ingredient codable concept coding is missing the `system` field!"
                            .to_owned(),
                    )
                }
            }

            code =
                Some(coding.code.ok_or_else(|| {
                    "Ingredient codable concept coding is missing the `code` field!"
                })?);
        }

        text = Some(
            item.text
                .ok_or_else(|| "Ingredient codable concept is missing the `text` field!")?,
        );
    }

    let strength = match (
        ingredient.strength.numerator,
        ingredient.strength.denominator,
    ) {
        (Some(numerator), Some(denominator)) => {
            match denominator.value {
                Some(1) => (),
                Some(_) => return Err("Strength denominator has unexpected value!".to_owned()),
                None => return Err("Strength denominator is missing the `value` field!".to_owned()),
            }

            match numerator.system {
                Some(s) if s == QUANTITY_SYSTEM_MEDICATION => (),
                Some(_) => return Err("Strength numerator has invalid system!".to_owned()),
                None => return Err("Strength numerator is missing the `system` field!".to_owned()),
            }

            Some(Amount {
                value: numerator
                    .value
                    .ok_or_else(|| "Strength numerator is missing the `value` field!")?,
                unit: numerator
                    .unit
                    .ok_or_else(|| "Strength numerator is missing the `unit` field!")?,
                code: numerator.code,
            })
        }
        (Some(_), None) => {
            return Err("Ingredient strength is missing the `denominator` field!".to_owned())
        }
        (None, Some(_)) => {
            return Err("Ingredient strength is missing the `numerator` field!".to_owned())
        }
        (None, None) => None,
    };

    let mut dosage_form = None;
    let mut amount_free_text = None;

    if is_compounding {
        for ex in ingredient.extension {
            match ex.url.as_str() {
                EXTENSION_URL_MEDICATION_INGREDIENT_FORM => match ex.value {
                    Some(ValueDef::String(value)) => dosage_form = Some(value.into()),
                    _ => {
                        return Err(
                            "Ingredient extension is missing the `valueString` field!".to_owned()
                        )
                    }
                },
                _ => return Err(format!("Ingredient extension is unexpected: {}", ex.url,)),
            }
        }

        for ex in ingredient.strength.extension {
            match ex.url.as_str() {
                EXTENSION_URL_MEDICATION_INGREDIENT_AMOUNT => match ex.value {
                    Some(ValueDef::String(value)) => amount_free_text = Some(value.into()),
                    _ => {
                        return Err(
                            "Ingredient strength extension is missing the `valueString` field!"
                                .to_owned(),
                        )
                    }
                },
                _ => {
                    return Err(format!(
                        "Ingredient strength extension is unexpected: {}",
                        ex.url,
                    ))
                }
            }
        }
    } else {
        if !ingredient.extension.is_empty() {
            return Err("Ingredient has unexpected extension!".to_owned());
        }

        if !ingredient.strength.extension.is_empty() {
            return Err("Ingredient strength has unexpected extension!".to_owned());
        }
    }

    Ok(Ingredient {
        code,
        text,
        strength,
        dosage_form,
        amount_free_text,
    })
}

fn deserialize_extension(
    extensions: Vec<ExtensionDef>,
    accepted_extensions: &[&str],
) -> Result<Option<Extension>, String> {
    if extensions.is_empty() {
        return Ok(None);
    }

    let mut category = None;
    let mut vaccine = None;
    let mut instruction = None;
    let mut packaging = None;
    let mut standard_size = None;

    for ex in extensions {
        if !accepted_extensions.contains(&ex.url.as_str()) {
            return Err(format!("Unexpceted extension: {}", ex.url));
        }

        match ex.url.as_str() {
            EXTENSION_URL_MEDICATION_CATEGORY => {
                let coding = match ex.value {
                    Some(ValueDef::Coding(coding)) => coding,
                    _ => {
                        return Err(
                            "Extension category is missing the `valueCoding` field!".to_owned()
                        )
                    }
                };

                match coding.system {
                    Some(s) if s == CODING_SYSTEM_MEDICATION_CATEGORY => (),
                    Some(_) => {
                        return Err("Extension category coding has unexpected system!".to_owned())
                    }
                    None => {
                        return Err(
                            "Extension category coding is missing the `system` field!".to_owned()
                        )
                    }
                }

                category = Some(
                    coding
                        .code
                        .as_deref()
                        .map(Category::decode_str)
                        .transpose()
                        .map_err(|err| format!("Extension category has invalid code: {}", err))?
                        .ok_or_else(|| "Extension category coding is missing the `code` field!")?,
                );
            }
            EXTENSION_URL_MEDICATION_VACCINE => {
                vaccine = match ex.value {
                    Some(ValueDef::Boolean(value)) => Some(value.into()),
                    _ => {
                        return Err(
                            "Extension vaccine is missing the `valueBoolean` field!".to_owned()
                        )
                    }
                };
            }
            EXTENSION_URL_MEDICATION_INSTRUCTION => {
                instruction = match ex.value {
                    Some(ValueDef::String(value)) => Some(value.into()),
                    _ => {
                        return Err(
                            "Extension instruction is missing the `valueString` field!".to_owned()
                        )
                    }
                };
            }
            EXTENSION_URL_MEDICATION_PACKAGING => {
                packaging = match ex.value {
                    Some(ValueDef::String(value)) => Some(value.into()),
                    _ => {
                        return Err(
                            "Extension packing is missing the `valueString` field!".to_owned()
                        )
                    }
                };
            }
            EXTENSION_URL_STANDARD_SIZE => {
                standard_size = match ex.value {
                    Some(ValueDef::Code(code)) => {
                        Some(StandardSize::decode(code.0).map_err(|err| {
                            format!("Extension standard size has invalid code: {}", err)
                        })?)
                    }
                    _ => {
                        return Err(
                            "Extension standard size is missing the `valueCode` field!".to_owned()
                        )
                    }
                };
            }
            _ => return Err(format!("Unexpceted extension: {}", ex.url)),
        }
    }

    Ok(Some(Extension {
        category: category.ok_or_else(|| "Category extension is missing!")?,
        vaccine: vaccine.ok_or_else(|| "Vaccone extension is missing!")?,
        instruction,
        packaging,
        standard_size,
    }))
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::convert::TryInto;
    use std::fs::read_to_string;

    use crate::fhir::{
        test::trim_xml_str,
        xml::{from_str as from_xml, to_string as to_xml},
    };

    use super::super::misc::Root;

    type MedicationRoot<'a> = Root<MedicationCow<'a>>;

    #[test]
    fn convert_to_compounding() {
        let medication = test_medication_compounding();
        let actual = trim_xml_str(&to_xml(&MedicationRoot::new(&medication)).unwrap());
        let expected =
            trim_xml_str(&read_to_string("./examples/medication_compounding.xml").unwrap());

        assert_eq!(actual, expected);
    }

    #[test]
    fn convert_to_free_text() {
        let medication = test_medication_free_text();
        let actual = trim_xml_str(&to_xml(&MedicationRoot::new(&medication)).unwrap());
        let expected =
            trim_xml_str(&read_to_string("./examples/medication_free_text.xml").unwrap());

        assert_eq!(actual, expected);
    }

    #[test]
    fn convert_to_ingredient() {
        let medication = test_medication_ingredient();
        let actual = trim_xml_str(&to_xml(&MedicationRoot::new(&medication)).unwrap());
        let expected =
            trim_xml_str(&read_to_string("./examples/medication_ingredient.xml").unwrap());

        assert_eq!(actual, expected);
    }

    #[test]
    fn convert_to_pzn() {
        let medication = test_medication_pzn();
        let actual = trim_xml_str(&to_xml(&MedicationRoot::new(&medication)).unwrap());
        let expected = trim_xml_str(&read_to_string("./examples/medication_pzn.xml").unwrap());

        assert_eq!(actual, expected);
    }

    #[test]
    fn convert_from_compounding() {
        let xml = read_to_string("./examples/medication_compounding.xml").unwrap();
        let actual = from_xml::<MedicationRoot>(&xml).unwrap().into_inner();
        let expected = test_medication_compounding();

        assert_eq!(actual, expected);
    }

    #[test]
    fn convert_from_free_text() {
        let xml = read_to_string("./examples/medication_free_text.xml").unwrap();
        let actual = from_xml::<MedicationRoot>(&xml).unwrap().into_inner();
        let expected = test_medication_free_text();

        assert_eq!(actual, expected);
    }

    #[test]
    fn convert_from_ingredient() {
        let xml = read_to_string("./examples/medication_ingredient.xml").unwrap();
        let actual = from_xml::<MedicationRoot>(&xml).unwrap().into_inner();
        let expected = test_medication_ingredient();

        assert_eq!(actual, expected);
    }

    #[test]
    fn convert_from_pzn() {
        let xml = read_to_string("./examples/medication_pzn.xml").unwrap();
        let actual = from_xml::<MedicationRoot>(&xml).unwrap().into_inner();
        let expected = test_medication_pzn();

        assert_eq!(actual, expected);
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
