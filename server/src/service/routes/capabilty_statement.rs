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

use std::convert::TryInto;

use actix_web::{
    http::header::{Quality, QualityItem},
    web::Query,
    HttpResponse,
};
use resources::capability_statement::{
    CapabilityStatement, FhirVersion, Format, Mode, Rest, Status,
};
use serde::Deserialize;

#[cfg(feature = "support-json")]
use crate::{
    fhir::json::{
        definitions::CapabilityStatementRoot as JsonCapabilityStatement, to_string as to_json,
    },
    service::constants::MIME_FHIR_JSON,
};
#[cfg(feature = "support-xml")]
use crate::{
    fhir::xml::{
        definitions::CapabilityStatementRoot as XmlCapabilityStatement, to_string as to_xml,
    },
    service::constants::MIME_FHIR_XML,
};

use crate::service::{header::Accept, misc::DataType, RequestError};

#[derive(Deserialize)]
pub struct QueryArgs {
    #[serde(rename = "_format")]
    format: Option<String>,
}

pub fn create() -> CapabilityStatement {
    CapabilityStatement {
        fhir_version: FhirVersion::V4_0_0,
        name: "Gem_erxCapabilityStatement".to_owned(),
        title: "E-Rezept Workflow CapabilityStatement".to_owned(),
        description: "E-Rezept Fachdienst Server Referenzimplementierung".to_owned(),
        date: "2020-01-01T00:00:00Z".try_into().unwrap(),
        status: Status::Draft,
        format: vec![
            #[cfg(feature = "support-xml")]
            Format::XML,
            #[cfg(feature = "support-json")]
            Format::JSON,
        ],
        rest: vec![Rest {
            mode: Mode::Server,
            resource: vec![],
        }],
    }
}

pub async fn get(accept: Accept, query: Query<QueryArgs>) -> Result<HttpResponse, RequestError> {
    let mut data_types = match query.0.format {
        #[cfg(feature = "support-xml")]
        Some(format) if format == "xml" => {
            vec![QualityItem::new(MIME_FHIR_XML.clone(), Quality::default())]
        }

        #[cfg(feature = "support-json")]
        Some(format) if format == "json" => {
            vec![QualityItem::new(MIME_FHIR_JSON.clone(), Quality::default())]
        }

        Some(format) => vec![format
            .parse()
            .map_err(|_| RequestError::InvalidFormatArgument(format))?],

        None => accept.0,
    };

    let mut use_default = data_types.is_empty();
    data_types.sort_by(|a, b| a.quality.cmp(&b.quality));
    for q in data_types {
        let data_type = DataType::from_mime(&q.item);
        if data_type == DataType::Unknown {
            continue;
        }

        match q.item.get_param("fhirVersion") {
            Some(fhir_version) if fhir_version != "4.0" => continue,
            _ => (),
        }

        match data_type {
            #[cfg(feature = "support-xml")]
            DataType::Xml => {
                return Ok(HttpResponse::Ok()
                    .content_type(DataType::Xml.as_mime().to_string())
                    .body(XML.clone()))
            }

            #[cfg(feature = "support-json")]
            DataType::Json => {
                return Ok(HttpResponse::Ok()
                    .content_type(DataType::Json.as_mime().to_string())
                    .body(JSON.clone()))
            }

            DataType::Any => {
                use_default = true;

                break;
            }

            DataType::Unknown => (),
        }
    }

    if use_default {
        #[cfg(feature = "support-xml")]
        return Ok(HttpResponse::Ok()
            .content_type(DataType::Xml.as_mime().to_string())
            .body(XML.clone()));

        #[cfg(all(feature = "support-json", not(feature = "support-xml")))]
        return Ok(HttpResponse::Ok()
            .content_type(DataType::Json.as_mime().to_string())
            .body(JSON.clone()));
    }

    Ok(HttpResponse::NotFound().finish())
}

#[cfg(feature = "support-xml")]
lazy_static! {
    static ref XML: String = to_xml(&XmlCapabilityStatement::new(
        super::ROUTES.capability_statement()
    ))
    .unwrap();
}

#[cfg(feature = "support-json")]
lazy_static! {
    static ref JSON: String = to_json(&JsonCapabilityStatement::new(
        super::ROUTES.capability_statement()
    ))
    .unwrap();
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::fs::read_to_string;

    #[cfg(feature = "support-json")]
    use crate::fhir::test::trim_json_str;
    #[cfg(feature = "support-xml")]
    use crate::fhir::test::trim_xml_str;

    #[test]
    #[cfg(feature = "support-xml")]
    fn as_xml() {
        #[cfg(all(feature = "interface-patient", feature = "interface-supplier"))]
        let file = "./examples/capability_statement_both.xml";

        #[cfg(all(not(feature = "interface-patient"), feature = "interface-supplier"))]
        let file = "./examples/capability_statement_supplier.xml";

        #[cfg(all(feature = "interface-patient", not(feature = "interface-supplier")))]
        let file = "./examples/capability_statement_patient.xml";

        let actual = &*XML;
        let expected = &trim_xml_str(&read_to_string(file).unwrap());

        assert_eq!(actual, expected);
    }

    #[test]
    #[cfg(feature = "support-json")]
    fn as_json() {
        #[cfg(all(feature = "interface-patient", feature = "interface-supplier"))]
        let file = "./examples/capability_statement_both.json";

        #[cfg(all(not(feature = "interface-patient"), feature = "interface-supplier"))]
        let file = "./examples/capability_statement_supplier.json";

        #[cfg(all(feature = "interface-patient", not(feature = "interface-supplier")))]
        let file = "./examples/capability_statement_patient.json";

        let actual = &*JSON;
        let expected = &trim_json_str(&read_to_string(file).unwrap());

        assert_eq!(actual, expected);
    }
}
