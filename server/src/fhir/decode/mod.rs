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
mod decode_stream;
mod item_stream;
mod json;
mod traits;
mod xml;

pub use decode_stream::{DataStream, DecodeError, DecodeStream, Fields, Optional, Search, Vector};
pub use item_stream::{DecodeFuture, Item};
pub use json::{Error as JsonError, Json, JsonDecode};
pub use traits::{decode_any, Decode};
pub use xml::{Error as XmlError, Xml, XmlDecode};

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::fmt::{Debug, Display};
    use std::fs::read;
    use std::path::Path;

    use bytes::Bytes;
    use futures::stream::{iter, Stream, StreamExt};

    pub fn load_stream<P: AsRef<Path>>(
        filename: P,
    ) -> impl Stream<Item = Result<Bytes, String>> + Send {
        let stream = read(filename).unwrap();
        let stream = Ok(Bytes::copy_from_slice(&stream));
        let stream = vec![stream].into_iter();

        iter(stream)
    }

    pub fn load_str(s: &str) -> impl Stream<Item = Result<Bytes, String>> + Send {
        let stream = Ok(Bytes::copy_from_slice(s.as_bytes()));
        let stream = vec![stream].into_iter();

        iter(stream)
    }

    pub async fn assert_stream_task<S, E>(mut s: S)
    where
        S: Stream<Item = Result<Item, E>> + Unpin,
        E: Display + Debug,
    {
        macro_rules! assert_some_ok {
            ($item:expr) => {
                assert_eq!($item, s.next().await.unwrap().unwrap())
            };
        }

        macro_rules! assert_none {
            () => {
                assert_eq!(true, s.next().await.is_none())
            };
        }

        assert_some_ok!(Item::BeginElement {
            name: "Task".into()
        });
        assert_some_ok!(Item::BeginElement {
            name: "meta".into()
        });
        assert_some_ok!(Item::Field {
            name: "profile".into(),
            value: "https://gematik.de/fhir/StructureDefinition/erxTask".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::BeginElement {
            name: "extension".into()
        });
        assert_some_ok!(Item::Field {
            name: "url".into(),
            value: "https://gematik.de/fhir/StructureDefinition/PrescriptionType".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::BeginElement {
            name: "valueCoding".into()
        });
        assert_some_ok!(Item::Field {
            name: "system".into(),
            value: "https://gematik.de/fhir/CodeSystem/Flowtype".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::Field {
            name: "code".into(),
            value: "160".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::Field {
            name: "display".into(),
            value: "Muster 16 (Apothekenpflichtige Arzneimittel)".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::BeginElement {
            name: "extension".into()
        });
        assert_some_ok!(Item::Field {
            name: "url".into(),
            value: "https://example.org/fhir/StructureDefinition/AcceptDate".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::Field {
            name: "valueDateTime".into(),
            value: "2020-03-02T08:25:05+00:00".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::BeginElement {
            name: "extension".into()
        });
        assert_some_ok!(Item::Field {
            name: "url".into(),
            value: "https://gematik.de/fhir/StructureDefinition/ExpiryDate".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::Field {
            name: "valueDateTime".into(),
            value: "2020-05-02T08:25:05+00:00".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::BeginElement {
            name: "identifier".into()
        });
        assert_some_ok!(Item::Field {
            name: "system".into(),
            value: "https://gematik.de/fhir/Namingsystem/PrescriptionID".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::Field {
            name: "value".into(),
            value: "160.123.456.789.123.58".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::BeginElement {
            name: "identifier".into()
        });
        assert_some_ok!(Item::Field {
            name: "system".into(),
            value: "https://gematik.de/fhir/Namingsystem/AccessCode".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::Field {
            name: "value".into(),
            value: "777bea0e13cc9c42ceec14aec3ddee2263325dc2c6c699db115f58fe423607ea".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::BeginElement {
            name: "identifier".into()
        });
        assert_some_ok!(Item::Field {
            name: "system".into(),
            value: "https://gematik.de/fhir/Namingsystem/Secret".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::Field {
            name: "value".into(),
            value: "c36ca26502892b371d252c99b496e31505ff449aca9bc69e231c58148f6233cf".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::Field {
            name: "status".into(),
            value: "in-progress".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::Field {
            name: "intent".into(),
            value: "order".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::BeginElement { name: "for".into() });
        assert_some_ok!(Item::BeginElement {
            name: "identifier".into()
        });
        assert_some_ok!(Item::Field {
            name: "system".into(),
            value: "http://fhir.de/NamingSystem/gkv/kvid-10".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::Field {
            name: "value".into(),
            value: "X123456789".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::Field {
            name: "authoredOn".into(),
            value: "2020-03-02T08:25:05+00:00".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::Field {
            name: "lastModified".into(),
            value: "2020-03-02T08:45:05+00:00".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::BeginElement {
            name: "performerType".into()
        });
        assert_some_ok!(Item::BeginElement {
            name: "coding".into()
        });
        assert_some_ok!(Item::Field {
            name: "system".into(),
            value: "urn:ietf:rfc:3986".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::Field {
            name: "code".into(),
            value: "urn:oid:1.2.276.0.76.4.54".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::Field {
            name: "display".into(),
            value: "Apotheke".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::BeginElement {
            name: "input".into()
        });
        assert_some_ok!(Item::BeginElement {
            name: "type".into()
        });
        assert_some_ok!(Item::BeginElement {
            name: "coding".into()
        });
        assert_some_ok!(Item::Field {
            name: "system".into(),
            value: "https://gematik.de/fhir/CodeSystem/Documenttype".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::Field {
            name: "code".into(),
            value: "1".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::Field {
            name: "display".into(),
            value: "Health Care Provider Prescription".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::BeginElement {
            name: "valueReference".into()
        });
        assert_some_ok!(Item::Field {
            name: "reference".into(),
            value: "#Bundle/KbvPrescriptionExample".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::BeginElement {
            name: "input".into()
        });
        assert_some_ok!(Item::BeginElement {
            name: "type".into()
        });
        assert_some_ok!(Item::BeginElement {
            name: "coding".into()
        });
        assert_some_ok!(Item::Field {
            name: "system".into(),
            value: "https://gematik.de/fhir/CodeSystem/Documenttype".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::Field {
            name: "code".into(),
            value: "2".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::Field {
            name: "display".into(),
            value: "Patient Confirmation".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::BeginElement {
            name: "valueReference".into()
        });
        assert_some_ok!(Item::Field {
            name: "reference".into(),
            value: "#Bundle/KbvPatientReceiptExample".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::BeginElement {
            name: "output".into()
        });
        assert_some_ok!(Item::BeginElement {
            name: "type".into()
        });
        assert_some_ok!(Item::BeginElement {
            name: "coding".into()
        });
        assert_some_ok!(Item::Field {
            name: "system".into(),
            value: "https://gematik.de/fhir/CodeSystem/Documenttype".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::Field {
            name: "code".into(),
            value: "3".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::Field {
            name: "display".into(),
            value: "Receipt".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::BeginElement {
            name: "valueReference".into()
        });
        assert_some_ok!(Item::Field {
            name: "reference".into(),
            value: "#Bundle/KbvReceiptExample".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::EndElement);
        assert_none!();
    }

    pub async fn assert_stream_task_create_parameters<S, E>(mut s: S)
    where
        S: Stream<Item = Result<Item, E>> + Unpin,
        E: Display + Debug,
    {
        macro_rules! assert_some_ok {
            ($item:expr) => {
                assert_eq!($item, s.next().await.unwrap().unwrap())
            };
        }

        macro_rules! assert_none {
            () => {
                assert_eq!(true, s.next().await.is_none())
            };
        }

        assert_some_ok!(Item::BeginElement {
            name: "Parameters".into()
        });
        assert_some_ok!(Item::BeginElement {
            name: "parameter".into()
        });
        assert_some_ok!(Item::Field {
            name: "name".into(),
            value: "workflowType".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::BeginElement {
            name: "valueCoding".into(),
        });
        assert_some_ok!(Item::Field {
            name: "system".into(),
            value: "https://gematik.de/fhir/CodeSystem/Flowtype".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::Field {
            name: "code".into(),
            value: "160".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::Field {
            name: "display".into(),
            value: "Muster 16 (Apothekenpflichtige Arzneimittel)".into(),
            extension: Vec::new(),
        });
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::EndElement);
        assert_none!();
    }

    pub async fn assert_stream_resource<S, E>(mut s: S)
    where
        S: Stream<Item = Result<Item, E>> + Unpin,
        E: Display + Debug,
    {
        macro_rules! assert_some_ok {
            ($item:expr) => {
                assert_eq!($item, s.next().await.unwrap().unwrap())
            };
        }

        macro_rules! assert_none {
            () => {
                assert_eq!(true, s.next().await.is_none())
            };
        }

        assert_some_ok!(Item::BeginElement {
            name: "Root".into()
        });
        assert_some_ok!(Item::BeginElement {
            name: "resource".into()
        });
        assert_some_ok!(Item::BeginElement {
            name: "Resource".into()
        });
        assert_some_ok!(Item::Field {
            name: "key".into(),
            value: "value".into(),
            extension: vec![]
        });
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::EndElement);
        assert_some_ok!(Item::EndElement);
        assert_none!();
    }

    pub async fn assert_stream_extended_value<S, E>(mut s: S)
    where
        S: Stream<Item = Result<Item, E>> + Unpin,
        E: Display + Debug,
    {
        macro_rules! assert_some_ok {
            ($item:expr) => {
                assert_eq!($item, s.next().await.unwrap().unwrap())
            };
        }

        macro_rules! assert_none {
            () => {
                assert_eq!(true, s.next().await.is_none())
            };
        }

        assert_some_ok!(Item::BeginElement {
            name: "Test".into()
        });
        assert_some_ok!(Item::Field {
            name: "name".into(),
            value: "value".into(),
            extension: vec![
                Item::BeginElement {
                    name: "extension".into()
                },
                Item::Field {
                    name: "fuu".into(),
                    value: "bar".into(),
                    extension: vec![],
                },
                Item::EndElement,
            ]
        });
        assert_some_ok!(Item::EndElement);
        assert_none!();
    }

    pub async fn assert_stream_extended_array<S, E>(mut s: S)
    where
        S: Stream<Item = Result<Item, E>> + Unpin,
        E: Display + Debug,
    {
        macro_rules! assert_some_ok {
            ($item:expr) => {
                assert_eq!($item, s.next().await.unwrap().unwrap())
            };
        }

        macro_rules! assert_none {
            () => {
                assert_eq!(true, s.next().await.is_none())
            };
        }

        assert_some_ok!(Item::BeginElement {
            name: "Test".into()
        });
        assert_some_ok!(Item::Field {
            name: "name".into(),
            value: "value1".into(),
            extension: vec![
                Item::BeginElement {
                    name: "extension".into()
                },
                Item::Field {
                    name: "fuu".into(),
                    value: "bar1".into(),
                    extension: vec![],
                },
                Item::EndElement,
            ]
        });
        assert_some_ok!(Item::Field {
            name: "name".into(),
            value: "value2".into(),
            extension: vec![
                Item::BeginElement {
                    name: "extension".into()
                },
                Item::Field {
                    name: "fuu".into(),
                    value: "bar2".into(),
                    extension: vec![],
                },
                Item::EndElement,
            ]
        });
        assert_some_ok!(Item::EndElement);
        assert_none!();
    }
}
