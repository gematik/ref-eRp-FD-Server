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

mod byte_stream;
mod encode_stream;
mod item;
mod json;
mod traits;
mod xml;

pub use encode_stream::{DataStorage, EncodeError, EncodeStream};
pub use item::{Item, ItemStream, Value};
pub use json::{Error as JsonError, Json, JsonEncode};
pub use traits::{encode_any, Encode};
pub use xml::{Error as XmlError, Xml, XmlEncode};

#[cfg(test)]
pub mod tests {
    use super::*;

    use futures::stream::{iter, Stream};

    pub fn stream_task() -> impl Stream<Item = Item> {
        let items = vec![
            Item::Root {
                name: "Task".into(),
            },
            Item::Field { name: "id".into() },
            Item::Value {
                value: "1234567890".into(),
                extension: vec![],
            },
            Item::Field {
                name: "meta".into(),
            },
            Item::Element,
            Item::Field {
                name: "profile".into(),
            },
            Item::Array,
            Item::Value {
                value: "https://gematik.de/fhir/StructureDefinition/ErxTask".into(),
                extension: vec![],
            },
            Item::End,
            Item::End,
            Item::Field {
                name: "extension".into(),
            },
            Item::Array,
            Item::Element,
            Item::Attrib { name: "url".into() },
            Item::Value {
                value: "https://gematik.de/fhir/StructureDefinition/PrescriptionType".into(),
                extension: vec![],
            },
            Item::Field {
                name: "valueCoding".into(),
            },
            Item::Element,
            Item::Field {
                name: "system".into(),
            },
            Item::Value {
                value: "https://gematik.de/fhir/CodeSystem/Flowtype".into(),
                extension: vec![],
            },
            Item::Field {
                name: "code".into(),
            },
            Item::Value {
                value: "160".into(),
                extension: vec![],
            },
            Item::Field {
                name: "display".into(),
            },
            Item::Value {
                value: "Muster 16 (Apothekenpflichtige Arzneimittel)".into(),
                extension: vec![],
            },
            Item::End,
            Item::End,
            Item::Element,
            Item::Attrib { name: "url".into() },
            Item::Value {
                value: "https://gematik.de/fhir/StructureDefinition/AcceptDate".into(),
                extension: vec![],
            },
            Item::Field {
                name: "valueDate".into(),
            },
            Item::Value {
                value: "2020-03-02".into(),
                extension: vec![],
            },
            Item::End,
            Item::Element,
            Item::Attrib { name: "url".into() },
            Item::Value {
                value: "https://gematik.de/fhir/StructureDefinition/ExpiryDate".into(),
                extension: vec![],
            },
            Item::Field {
                name: "valueDate".into(),
            },
            Item::Value {
                value: "2020-05-02".into(),
                extension: vec![],
            },
            Item::End,
            Item::End,
            Item::Field {
                name: "identifier".into(),
            },
            Item::Array,
            Item::Element,
            Item::Field {
                name: "system".into(),
            },
            Item::Value {
                value: "https://gematik.de/fhir/NamingSystem/PrescriptionID".into(),
                extension: vec![],
            },
            Item::Field {
                name: "value".into(),
            },
            Item::Value {
                value: "160.123.456.789.123.58".into(),
                extension: vec![],
            },
            Item::End,
            Item::Element,
            Item::Field {
                name: "system".into(),
            },
            Item::Value {
                value: "https://gematik.de/fhir/NamingSystem/AccessCode".into(),
                extension: vec![],
            },
            Item::Field {
                name: "value".into(),
            },
            Item::Value {
                value: "777bea0e13cc9c42ceec14aec3ddee2263325dc2c6c699db115f58fe423607ea".into(),
                extension: vec![],
            },
            Item::End,
            Item::Element,
            Item::Field {
                name: "system".into(),
            },
            Item::Value {
                value: "https://gematik.de/fhir/NamingSystem/Secret".into(),
                extension: vec![],
            },
            Item::Field {
                name: "value".into(),
            },
            Item::Value {
                value: "c36ca26502892b371d252c99b496e31505ff449aca9bc69e231c58148f6233cf".into(),
                extension: vec![],
            },
            Item::End,
            Item::End,
            Item::Field {
                name: "status".into(),
            },
            Item::Value {
                value: "in-progress".into(),
                extension: vec![],
            },
            Item::Field {
                name: "intent".into(),
            },
            Item::Value {
                value: "order".into(),
                extension: vec![],
            },
            Item::Field { name: "for".into() },
            Item::Element,
            Item::Field {
                name: "identifier".into(),
            },
            Item::Element,
            Item::Field {
                name: "system".into(),
            },
            Item::Value {
                value: "http://fhir.de/NamingSystem/gkv/kvid-10".into(),
                extension: vec![],
            },
            Item::Field {
                name: "value".into(),
            },
            Item::Value {
                value: "X123456789".into(),
                extension: vec![],
            },
            Item::End,
            Item::End,
            Item::Field {
                name: "authoredOn".into(),
            },
            Item::Value {
                value: "2020-03-02T08:25:05+00:00".into(),
                extension: vec![],
            },
            Item::Field {
                name: "lastModified".into(),
            },
            Item::Value {
                value: "2020-03-02T08:45:05+00:00".into(),
                extension: vec![],
            },
            Item::Field {
                name: "performerType".into(),
            },
            Item::Array,
            Item::Element,
            Item::Field {
                name: "coding".into(),
            },
            Item::Array,
            Item::Element,
            Item::Field {
                name: "system".into(),
            },
            Item::Value {
                value: "urn:ietf:rfc:3986".into(),
                extension: vec![],
            },
            Item::Field {
                name: "code".into(),
            },
            Item::Value {
                value: "urn:oid:1.2.276.0.76.4.54".into(),
                extension: vec![],
            },
            Item::Field {
                name: "display".into(),
            },
            Item::Value {
                value: "Öffentliche Apotheke".into(),
                extension: vec![],
            },
            Item::End,
            Item::End,
            Item::End,
            Item::End,
            Item::Field {
                name: "input".into(),
            },
            Item::Array,
            Item::Element,
            Item::Field {
                name: "type".into(),
            },
            Item::Element,
            Item::Field {
                name: "coding".into(),
            },
            Item::Array,
            Item::Element,
            Item::Field {
                name: "system".into(),
            },
            Item::Value {
                value: "https://gematik.de/fhir/CodeSystem/Documenttype".into(),
                extension: vec![],
            },
            Item::Field {
                name: "code".into(),
            },
            Item::Value {
                value: "1".into(),
                extension: vec![],
            },
            Item::Field {
                name: "display".into(),
            },
            Item::Value {
                value: "Health Care Provider Prescription".into(),
                extension: vec![],
            },
            Item::End,
            Item::End,
            Item::End,
            Item::Field {
                name: "valueReference".into(),
            },
            Item::Element,
            Item::Field {
                name: "reference".into(),
            },
            Item::Value {
                value: "Bundle/KbvPrescriptionExample".into(),
                extension: vec![],
            },
            Item::End,
            Item::End,
            Item::Element,
            Item::Field {
                name: "type".into(),
            },
            Item::Element,
            Item::Field {
                name: "coding".into(),
            },
            Item::Array,
            Item::Element,
            Item::Field {
                name: "system".into(),
            },
            Item::Value {
                value: "https://gematik.de/fhir/CodeSystem/Documenttype".into(),
                extension: vec![],
            },
            Item::Field {
                name: "code".into(),
            },
            Item::Value {
                value: "2".into(),
                extension: vec![],
            },
            Item::Field {
                name: "display".into(),
            },
            Item::Value {
                value: "Patient Confirmation".into(),
                extension: vec![],
            },
            Item::End,
            Item::End,
            Item::End,
            Item::Field {
                name: "valueReference".into(),
            },
            Item::Element,
            Item::Field {
                name: "reference".into(),
            },
            Item::Value {
                value: "Bundle/KbvPatientReceiptExample".into(),
                extension: vec![],
            },
            Item::End,
            Item::End,
            Item::End,
            Item::Field {
                name: "output".into(),
            },
            Item::Array,
            Item::Element,
            Item::Field {
                name: "type".into(),
            },
            Item::Element,
            Item::Field {
                name: "coding".into(),
            },
            Item::Array,
            Item::Element,
            Item::Field {
                name: "system".into(),
            },
            Item::Value {
                value: "https://gematik.de/fhir/CodeSystem/Documenttype".into(),
                extension: vec![],
            },
            Item::Field {
                name: "code".into(),
            },
            Item::Value {
                value: "3".into(),
                extension: vec![],
            },
            Item::Field {
                name: "display".into(),
            },
            Item::Value {
                value: "Receipt".into(),
                extension: vec![],
            },
            Item::End,
            Item::End,
            Item::End,
            Item::Field {
                name: "valueReference".into(),
            },
            Item::Element,
            Item::Field {
                name: "reference".into(),
            },
            Item::Value {
                value: "Bundle/KbvReceiptExample".into(),
                extension: vec![],
            },
            Item::End,
            Item::End,
            Item::End,
            Item::End,
        ];

        iter(items.into_iter())
    }

    pub fn stream_task_create_parameters() -> impl Stream<Item = Item> {
        let items = vec![
            Item::Root {
                name: "Parameters".into(),
            },
            Item::Field {
                name: "parameter".into(),
            },
            Item::Array,
            Item::Element,
            Item::Field {
                name: "name".into(),
            },
            Item::Value {
                value: "workflowType".into(),
                extension: Vec::new(),
            },
            Item::Field {
                name: "valueCoding".into(),
            },
            Item::Element,
            Item::Field {
                name: "system".into(),
            },
            Item::Value {
                value: "https://gematik.de/fhir/CodeSystem/Flowtype".into(),
                extension: Vec::new(),
            },
            Item::Field {
                name: "code".into(),
            },
            Item::Value {
                value: "160".into(),
                extension: Vec::new(),
            },
            Item::Field {
                name: "display".into(),
            },
            Item::Value {
                value: "Muster 16 (Apothekenpflichtige Arzneimittel)".into(),
                extension: Vec::new(),
            },
            Item::End,
            Item::End,
            Item::End,
            Item::End,
        ];

        iter(items.into_iter())
    }

    pub fn stream_resource() -> impl Stream<Item = Item> {
        let items = vec![
            Item::Root {
                name: "Root".into(),
            },
            Item::Field {
                name: "resource".into(),
            },
            Item::Root {
                name: "Resource".into(),
            },
            Item::Field { name: "key".into() },
            Item::Value {
                value: "value".into(),
                extension: vec![],
            },
            Item::End,
            Item::End,
        ];

        iter(items.into_iter())
    }

    pub fn stream_extended_value() -> impl Stream<Item = Item> {
        let items = vec![
            Item::Root {
                name: "Test".into(),
            },
            Item::Field {
                name: "name".into(),
            },
            Item::Value {
                value: "value".into(),
                extension: vec![
                    Item::Field {
                        name: "extension".into(),
                    },
                    Item::Array,
                    Item::Element,
                    Item::Field { name: "fuu".into() },
                    Item::Value {
                        value: "bar".into(),
                        extension: vec![],
                    },
                    Item::End,
                    Item::End,
                ],
            },
            Item::End,
        ];

        iter(items.into_iter())
    }

    pub fn stream_extended_array() -> impl Stream<Item = Item> {
        let items = vec![
            Item::Root {
                name: "Test".into(),
            },
            Item::Field {
                name: "name".into(),
            },
            Item::Array,
            Item::Value {
                value: "value1".into(),
                extension: vec![
                    Item::Field {
                        name: "extension".into(),
                    },
                    Item::Array,
                    Item::Element,
                    Item::Field { name: "fuu".into() },
                    Item::Value {
                        value: "bar1".into(),
                        extension: vec![],
                    },
                    Item::End,
                    Item::End,
                ],
            },
            Item::Value {
                value: "value2".into(),
                extension: vec![
                    Item::Field {
                        name: "extension".into(),
                    },
                    Item::Array,
                    Item::Element,
                    Item::Field { name: "fuu".into() },
                    Item::Value {
                        value: "bar2".into(),
                        extension: vec![],
                    },
                    Item::End,
                    Item::End,
                ],
            },
            Item::End,
            Item::End,
        ];

        iter(items.into_iter())
    }

    pub fn stream_extended_value_empty() -> impl Stream<Item = Item> {
        let items = vec![
            Item::Root {
                name: "Test".into(),
            },
            Item::Field {
                name: "name".into(),
            },
            Item::Value {
                value: "value".into(),
                extension: vec![
                    Item::Field {
                        name: "extension".into(),
                    },
                    Item::Array,
                    Item::End,
                ],
            },
            Item::End,
        ];

        iter(items.into_iter())
    }
}
