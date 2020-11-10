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

use super::primitives::Id;

#[derive(Clone, PartialEq, Debug)]
pub struct Medication {
    pub id: Id,
    pub data: Data,
    pub extension: Option<Extension>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Data {
    Compounding(CompoundingData),
    FreeText(FreeTextData),
    Ingredient(IngredientData),
    Pzn(PznData),
}

#[derive(Clone, PartialEq, Debug)]
pub struct Extension {
    pub category: Category,
    pub vaccine: bool,
    pub instruction: Option<String>,
    pub packaging: Option<String>,
    pub standard_size: Option<StandardSize>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct CompoundingData {
    pub code: Option<String>,
    pub form: String,
    pub amount: Amount,
    pub ingredient: Vec<Ingredient>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct FreeTextData {
    pub code: String,
    pub form: Option<String>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct IngredientData {
    pub form: String,
    pub amount: Option<Amount>,
    pub ingredient: Ingredient,
}

#[derive(Clone, PartialEq, Debug)]
pub struct PznData {
    pub code: PznCode,
    pub form: PznForm,
    pub amount: Option<Amount>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct PznCode {
    pub text: String,
    pub code: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct PznForm {
    pub system: String,
    pub code: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Amount {
    pub value: usize,
    pub unit: String,
    pub code: Option<String>,
}

#[derive(Default, Clone, PartialEq, Debug)]
pub struct Ingredient {
    pub code: Option<String>,
    pub text: Option<String>,
    pub strength: Option<Amount>,
    pub dosage_form: Option<String>,
    pub amount_free_text: Option<String>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Category {
    Medicine,
    BTM,
    AMVV,
}

#[derive(Clone, PartialEq, Debug)]
pub enum StandardSize {
    N1,
    N2,
    N3,
    KTP,
    KA,
    NB,
    Other,
}
