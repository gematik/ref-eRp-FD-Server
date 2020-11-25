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
use resources::Task;

#[cfg(feature = "support-json")]
use crate::fhir::encode::JsonEncode;
#[cfg(feature = "support-xml")]
use crate::fhir::encode::XmlEncode;
use crate::service::{misc::DataType, RequestError};

pub fn response_with_task(
    task: &Task,
    data_type: DataType,
    created: bool,
) -> Result<HttpResponse, RequestError> {
    let (body, content_type) = match data_type {
        #[cfg(feature = "support-xml")]
        DataType::Xml => {
            let xml = task.xml()?;

            (xml, DataType::Xml.as_mime().to_string())
        }

        #[cfg(feature = "support-json")]
        DataType::Json => {
            let json = task.json()?;

            (json, DataType::Json.as_mime().to_string())
        }

        DataType::Any | DataType::Unknown => panic!("Data type of response was not specified"),
    };

    let mut res = if created {
        HttpResponse::Created()
    } else {
        HttpResponse::Ok()
    };

    Ok(res.content_type(content_type).body(body))
}
