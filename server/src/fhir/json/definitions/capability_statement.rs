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

use resources::{
    capability_statement::{
        CapabilityStatement, FhirVersion, Format, Interaction, Mode, Operation, Resource, Rest,
        Status, Type,
    },
    primitives::DateTime,
};
use serde::{Serialize, Serializer};

use super::{
    super::super::constants::RESOURCE_CAPABILITY_STATEMENT,
    misc::{ResourceType, Root, SerializeRoot},
    primitives::DateTimeDef,
};

pub struct CapabilityStatementDef;
pub type CapabilityStatementRoot<'a> = Root<CapabilityStatementCow<'a>>;

#[derive(Clone, Serialize)]
#[serde(rename = "CapabilityStatement")]
pub struct CapabilityStatementCow<'a>(
    #[serde(with = "CapabilityStatementDef")] pub Cow<'a, CapabilityStatement>,
);

#[derive(Serialize)]
#[serde(rename = "CapabilityStatement")]
#[serde(rename_all = "camelCase")]
struct CapabilityStatementHelper {
    #[serde(with = "FhirVersionDef")]
    fhir_version: FhirVersion,

    name: String,

    title: String,

    #[serde(with = "StatusDef")]
    status: Status,

    #[serde(with = "DateTimeDef")]
    date: DateTime,

    kind: KindDef,

    implementation: ImplementationDef,

    format: Vec<FormatDef>,

    rest: Vec<RestDef>,
}

#[derive(Serialize)]
struct ImplementationDef {
    description: String,
}

#[derive(Serialize)]
struct RestDef {
    #[serde(with = "ModeDef")]
    mode: Mode,

    resource: Vec<ResourceDef>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResourceDef {
    #[serde(with = "TypeDef")]
    #[serde(rename = "type")]
    type_: Type,

    profile: String,

    supported_profile: Vec<String>,

    interaction: Vec<InteractionDef>,

    operation: Vec<OperationDef>,
}

#[derive(Serialize)]
struct OperationDef {
    name: String,
    definition: String,
}

#[derive(Serialize)]
struct InteractionDef {
    #[serde(with = "InteractionCodeDef")]
    code: Interaction,
}

#[derive(Serialize)]
#[serde(remote = "FhirVersion")]
enum FhirVersionDef {
    #[serde(rename = "4.0.0")]
    V4_0_0,
    #[serde(rename = "4.0.1")]
    V4_0_1,
}

#[derive(Serialize)]
#[serde(remote = "Status")]
#[serde(rename_all = "kebab-case")]
enum StatusDef {
    Draft,
    Active,
    Retired,
    Unknown,
}

#[allow(dead_code)]
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
enum KindDef {
    Instance,
    Capability,
    Requirements,
}

#[derive(Debug, Serialize)]
enum FormatDef {
    #[serde(rename = "application/fhir+xml")]
    XML,

    #[serde(rename = "application/fhir+json")]
    JSON,
}

#[derive(Serialize)]
#[serde(remote = "Mode")]
#[serde(rename_all = "kebab-case")]
enum ModeDef {
    Client,
    Server,
}

#[derive(Serialize)]
#[serde(remote = "Type")]
enum TypeDef {
    Task,
    Operation,
    Communication,
}

#[derive(Serialize)]
#[serde(remote = "Interaction")]
#[serde(rename_all = "kebab-case")]
pub enum InteractionCodeDef {
    Read,
    Vread,
    Update,
    Patch,
    Delete,
    HistoryInstance,
    HistoryType,
    Create,
    SearchTyp,
}

impl ResourceType for CapabilityStatement {
    fn resource_type() -> &'static str {
        RESOURCE_CAPABILITY_STATEMENT
    }
}

impl<'a> SerializeRoot<'a> for CapabilityStatementCow<'a> {
    type Inner = CapabilityStatement;

    fn from_inner(inner: &'a Self::Inner) -> Self {
        CapabilityStatementCow(Cow::Borrowed(inner))
    }
}

impl CapabilityStatementDef {
    fn serialize<S: Serializer>(
        capability_statement: &CapabilityStatement,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let root: CapabilityStatementHelper = capability_statement.into();

        root.serialize(serializer)
    }
}

impl Into<CapabilityStatementHelper> for &CapabilityStatement {
    fn into(self) -> CapabilityStatementHelper {
        CapabilityStatementHelper {
            fhir_version: self.fhir_version,
            name: self.name.clone(),
            title: self.title.clone(),
            status: self.status,
            date: self.date.clone(),
            kind: KindDef::Instance,
            implementation: ImplementationDef {
                description: self.description.clone(),
            },
            format: self
                .format
                .iter()
                .map(|t| match t {
                    Format::XML => FormatDef::XML,
                    Format::JSON => FormatDef::JSON,
                })
                .collect(),
            rest: self.rest.iter().map(Into::into).collect(),
        }
    }
}

impl Into<RestDef> for &Rest {
    fn into(self) -> RestDef {
        RestDef {
            mode: self.mode,
            resource: self.resource.iter().map(Into::into).collect(),
        }
    }
}

impl Into<ResourceDef> for &Resource {
    fn into(self) -> ResourceDef {
        ResourceDef {
            type_: self.type_,
            profile: self.profile.clone(),
            supported_profile: self.supported_profiles.clone(),
            operation: self.operation.iter().map(Into::into).collect(),
            interaction: self
                .interaction
                .iter()
                .map(Clone::clone)
                .map(Into::into)
                .collect(),
        }
    }
}

impl Into<OperationDef> for &Operation {
    fn into(self) -> OperationDef {
        OperationDef {
            name: self.name.clone(),
            definition: self.definition.clone(),
        }
    }
}

impl Into<InteractionDef> for Interaction {
    fn into(self) -> InteractionDef {
        InteractionDef { code: self }
    }
}
