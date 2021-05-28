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

pub mod access_token;
pub mod data_type;
pub mod from_query;
pub mod logging;
pub mod search;
pub mod sort;

pub use access_token::{AccessToken, Error as AccessTokenError, Profession};
pub use data_type::DataType;
pub use from_query::{FromQuery, Query, QueryValue};
pub use search::Search;
pub use sort::Sort;

use std::convert::TryInto;

use actix_web::{dev::HttpResponseBuilder, http::StatusCode, web::Payload, HttpResponse};
use regex::{Captures, Regex};
use resources::device::{Device, DeviceName, Status, Type};

use crate::fhir::{decode::Decode, encode::Encode};

use super::{IntoReqErrResult, RequestError, TypedRequestError, TypedRequestResult};

lazy_static! {
    pub static ref DEVICE: Device = Device {
        id: "ErxService".try_into().unwrap(),
        status: Status::Active,
        serial_number: None,
        device_name: DeviceName {
            name: "E-Rezept Fachdienst Referenzimplementierung".into(),
            type_: Type::UserFriendlyName,
        },
        version: format_version(),
    };
}

pub fn format_version() -> String {
    static VERSIONS: [Option<&str>; 2] = [
        option_env!("GIT_VERSION_TAG"),
        option_env!("CARGO_PKG_VERSION"),
    ];
    let version = VERSIONS
        .iter()
        .filter_map(|s| match s {
            Some(s) if !s.is_empty() => Some(s),
            _ => None,
        })
        .next()
        .unwrap_or(&"0.0.0");
    let mut ret = (*version).to_owned();

    if let Some(commits_behind) = option_env!("GIT_COMMITS_BEHIND") {
        if !commits_behind.is_empty() && commits_behind != "0" {
            ret = format!("{}+{}", ret, commits_behind);
        }
    }

    if let Some(dirty) = option_env!("GIT_DIRTY") {
        if dirty == "1" {
            ret = format!("{} dirty", ret);
        }
    }

    if let Some(hash) = option_env!("GIT_HASH") {
        if !hash.is_empty() {
            ret = format!("{} {}", ret, hash);
        }
    }

    ret
}

pub async fn read_payload<T>(data_type: DataType, mut payload: Payload) -> Result<T, RequestError>
where
    T: Decode,
{
    match data_type {
        #[cfg(feature = "support-xml")]
        DataType::Xml => {
            use crate::fhir::decode::XmlDecode;

            Ok(payload.xml().await?)
        }

        #[cfg(feature = "support-json")]
        DataType::Json => {
            use crate::fhir::decode::JsonDecode;

            Ok(payload.json().await?)
        }

        DataType::Unknown => Err(RequestError::ContentTypeNotSupported),

        DataType::Any => Err(RequestError::ContentTypeNotSupported),
    }
}

pub fn create_response<T>(
    response: T,
    data_type: DataType,
) -> Result<HttpResponse, TypedRequestError>
where
    T: Encode,
{
    create_response_with(response, data_type, StatusCode::OK, |_| ())
}

pub fn create_response_with<T, F>(
    response: T,
    data_type: DataType,
    status: StatusCode,
    f: F,
) -> Result<HttpResponse, TypedRequestError>
where
    T: Encode,
    F: FnOnce(&mut HttpResponseBuilder),
{
    match data_type {
        #[cfg(feature = "support-xml")]
        DataType::Xml => {
            use crate::fhir::encode::XmlEncode;

            let xml = response.xml().into_req_err().err_with_type(data_type)?;

            let mut res = HttpResponseBuilder::new(status);
            res.content_type(DataType::Xml.as_mime().to_string());

            f(&mut res);

            Ok(res.body(xml))
        }

        #[cfg(feature = "support-json")]
        DataType::Json => {
            use crate::fhir::encode::JsonEncode;

            let json = response.json().into_req_err().err_with_type(data_type)?;

            let mut res = HttpResponseBuilder::new(status);
            res.content_type(DataType::Json.as_mime().to_string());

            f(&mut res);

            Ok(res.body(json))
        }

        DataType::Any | DataType::Unknown => panic!("Data type of response was not specified"),
    }
}

pub fn make_page_uri(uri: &str, query: &str, page_id: usize) -> String {
    lazy_static! {
        static ref RX: Regex = Regex::new("(&?)(pageId|pageid|page-id)=[^&]").unwrap();
    }

    let mut done = false;
    let query = RX.replace(query, |c: &Captures| {
        done = true;

        format!("{}pageId={}", &c[1], page_id)
    });

    if done {
        format!("{}?{}", uri, query)
    } else if query.is_empty() {
        format!("{}?pageId={}", uri, page_id)
    } else {
        format!("{}?{}&pageId={}", uri, query, page_id)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn make_page_uri_test() {
        assert_eq!("/Task?pageId=6", make_page_uri("/Task", "", 6));
        assert_eq!(
            "/Task?fuu=bar&pageId=7",
            make_page_uri("/Task", "fuu=bar", 7)
        );
        assert_eq!(
            "/Task?pageId=8&bar=2&baz=3",
            make_page_uri("/Task", "pageId=1&bar=2&baz=3", 8)
        );
        assert_eq!(
            "/Task?bar=2&pageId=9&baz=3",
            make_page_uri("/Task", "bar=2&pageid=1&baz=3", 9)
        );
        assert_eq!(
            "/Task?bar=2&baz=3&pageId=10",
            make_page_uri("/Task", "bar=2&baz=3&page-id=1", 10)
        );
    }
}
