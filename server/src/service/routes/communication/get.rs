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

use actix_web::{
    web::{Data, Path, Query},
    HttpResponse,
};
use chrono::{DateTime, Utc};
use resources::{primitives::Id, Communication};
use serde::Deserialize;

use crate::service::{
    header::{Accept, Authorization},
    misc::{DataType, Profession, Search},
    state::{CommunicationMatch, State},
    RequestError,
};

use super::misc::response_with_communication;

#[derive(Deserialize)]
pub struct QueryArgs {
    sent: Option<Search<DateTime<Utc>>>,
    received: Option<Search<DateTime<Utc>>>,
    recipient: Option<Search<String>>,
    sender: Option<Search<String>>,
}

pub async fn get_all(
    state: Data<State>,
    accept: Accept,
    access_token: Authorization,
    query: Query<QueryArgs>,
) -> Result<HttpResponse, RequestError> {
    access_token.check_profession(|p| match p {
        Profession::Versicherter => true,
        Profession::KrankenhausApotheke => true,
        Profession::OeffentlicheApotheke => true,
        _ => false,
    })?;

    let accept = DataType::from_accept(accept)
        .and_then(DataType::ignore_any)
        .unwrap_or_default()
        .check_supported()?;

    let kvnr = access_token.kvnr().ok();
    let telematik_id = access_token.telematik_id().ok();

    let mut state = state.lock().await;
    let mut bundle: Box<dyn BundleHelper> = match accept {
        #[cfg(feature = "support-xml")]
        DataType::Xml => Box::new(xml::BundleHelper::new()),

        #[cfg(feature = "support-json")]
        DataType::Json => Box::new(json::BundleHelper::new()),

        DataType::Unknown | DataType::Any => unreachable!(),
    };

    for communication in state.iter_communications(kvnr, telematik_id) {
        match communication {
            CommunicationMatch::Sender(c) => {
                if check_query(&query, c) {
                    bundle.add(c);
                }
            }
            CommunicationMatch::Recipient(c) => {
                if check_query(&query, c) {
                    if c.received().is_none() {
                        c.set_received(Utc::now().into());
                    }

                    bundle.add(c);
                }
            }
            _ => unreachable!(),
        }
    }

    bundle.to_response()
}

pub async fn get_one(
    state: Data<State>,
    id: Path<Id>,
    accept: Accept,
    access_token: Authorization,
) -> Result<HttpResponse, RequestError> {
    access_token.check_profession(|p| match p {
        Profession::Versicherter => true,
        Profession::KrankenhausApotheke => true,
        Profession::OeffentlicheApotheke => true,
        _ => false,
    })?;

    let accept = DataType::from_accept(accept)
        .and_then(DataType::ignore_any)
        .unwrap_or_default()
        .check_supported()?;

    let id = id.into_inner();
    let kvnr = access_token.kvnr().ok();
    let telematik_id = access_token.telematik_id().ok();

    let mut state = state.lock().await;
    let communication = match state.get_communication(&id, &kvnr, &telematik_id) {
        CommunicationMatch::NotFound => return Ok(HttpResponse::NotFound().finish()),
        CommunicationMatch::Unauthorized => return Ok(HttpResponse::Unauthorized().finish()),
        CommunicationMatch::Sender(c) => c,
        CommunicationMatch::Recipient(c) => {
            if c.received().is_none() {
                c.set_received(Utc::now().into());
            }

            c
        }
    };

    response_with_communication(communication, accept)
}

fn check_query(query: &Query<QueryArgs>, communication: &Communication) -> bool {
    if let Some(qsent) = &query.sent {
        if let Some(csent) = communication.sent() {
            let csent = csent.clone().into();
            if !qsent.matches(&csent) {
                return false;
            }
        } else {
            return false;
        }
    }

    if let Some(qreceived) = &query.received {
        if let Some(creceived) = communication.received() {
            let creceived = creceived.clone().into();
            if !qreceived.matches(&creceived) {
                return false;
            }
        } else {
            return false;
        }
    }

    if let Some(qsender) = &query.sender {
        if let Some(csender) = communication.sender() {
            if !qsender.matches(&csender) {
                return false;
            }
        } else {
            return false;
        }
    }

    if let Some(qrecipient) = &query.recipient {
        let crecipient = communication.recipient();
        if !qrecipient.matches(&crecipient) {
            return false;
        }
    }

    true
}

trait BundleHelper<'a> {
    fn add(&mut self, communication: &'a Communication);
    fn to_response(&self) -> Result<HttpResponse, RequestError>;
}

#[cfg(feature = "support-xml")]
mod xml {
    use super::*;

    use std::borrow::Cow;

    use resources::bundle::{Bundle, Entry, Type};
    use serde::Serialize;

    use crate::fhir::xml::{
        definitions::{BundleRoot as XmlBundle, CommunicationCow as XmlCommunication},
        to_string as to_xml,
    };

    use super::BundleHelper as BundleHelperTrait;

    pub struct BundleHelper<'a> {
        bundle: Bundle<Resource<'a>>,
    }

    #[derive(Clone, Serialize)]
    pub enum Resource<'a> {
        Communication(XmlCommunication<'a>),
    }

    impl BundleHelper<'_> {
        pub fn new() -> Self {
            Self {
                bundle: Bundle::new(Type::Collection),
            }
        }
    }

    impl<'a> BundleHelperTrait<'a> for BundleHelper<'a> {
        fn add(&mut self, communication: &'a Communication) {
            let communication = XmlCommunication(Cow::Borrowed(communication));
            let resource = Resource::Communication(communication);
            let entry = Entry::new(resource);

            self.bundle.entries.push(entry);
        }

        fn to_response(&self) -> Result<HttpResponse, RequestError> {
            let xml = to_xml(&XmlBundle::new(&self.bundle)).map_err(RequestError::SerializeXml)?;

            Ok(HttpResponse::Ok()
                .content_type(DataType::Xml.as_mime().to_string())
                .body(xml))
        }
    }
}

#[cfg(feature = "support-json")]
mod json {
    use super::*;

    use std::borrow::Cow;

    use resources::bundle::{Bundle, Entry, Type};
    use serde::Serialize;

    use crate::fhir::json::{
        definitions::{BundleRoot as JsonBundle, CommunicationCow as JsonCommunication},
        to_string as to_json,
    };

    use super::BundleHelper as BundleHelperTrait;

    pub struct BundleHelper<'a> {
        bundle: Bundle<Resource<'a>>,
    }

    #[derive(Clone, Serialize)]
    #[serde(tag = "resourceType")]
    pub enum Resource<'a> {
        Communication(JsonCommunication<'a>),
    }

    impl BundleHelper<'_> {
        pub fn new() -> Self {
            Self {
                bundle: Bundle::new(Type::Collection),
            }
        }
    }

    impl<'a> BundleHelperTrait<'a> for BundleHelper<'a> {
        fn add(&mut self, communication: &'a Communication) {
            let communication = JsonCommunication(Cow::Borrowed(communication));
            let resource = Resource::Communication(communication);
            let entry = Entry::new(resource);

            self.bundle.entries.push(entry);
        }

        fn to_response(&self) -> Result<HttpResponse, RequestError> {
            let json =
                to_json(&JsonBundle::new(&self.bundle)).map_err(RequestError::SerializeJson)?;

            Ok(HttpResponse::Ok()
                .content_type(DataType::Json.as_mime().to_string())
                .body(json))
        }
    }
}
