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

use actix_web::HttpResponse;
use resources::Communication;

#[cfg(feature = "support-json")]
use crate::fhir::json::{
    definitions::CommunicationRoot as JsonCommunication, to_string as to_json,
};
#[cfg(feature = "support-xml")]
use crate::fhir::xml::{definitions::CommunicationRoot as XmlCommunication, to_string as to_xml};
use crate::service::{misc::DataType, RequestError};

pub fn response_with_communication(
    communication: &Communication,
    data_type: DataType,
) -> Result<HttpResponse, RequestError> {
    match data_type {
        #[cfg(feature = "support-xml")]
        DataType::Xml => {
            let xml = to_xml(&XmlCommunication::new(&communication))
                .map_err(RequestError::SerializeXml)?;

            Ok(HttpResponse::Ok()
                .content_type(DataType::Xml.as_mime().to_string())
                .body(xml))
        }

        #[cfg(feature = "support-json")]
        DataType::Json => {
            let json = to_json(&JsonCommunication::new(&communication))
                .map_err(RequestError::SerializeJson)?;

            Ok(HttpResponse::Ok()
                .content_type(DataType::Json.as_mime().to_string())
                .body(json))
        }

        DataType::Any | DataType::Unknown => panic!("Data type of response was not specified"),
    }
}
