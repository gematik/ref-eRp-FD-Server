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

use std::str::FromStr;

use actix_web::{
    web::{Data, Path, Query},
    HttpResponse,
};
use chrono::{DateTime, Utc};
use resources::{
    bundle::{Bundle, Entry, Type},
    primitives::Id,
    Communication,
};
use serde::Deserialize;

use crate::{
    fhir::encode::{JsonEncode, XmlEncode},
    service::{
        header::{Accept, Authorization},
        misc::{DataType, Profession, Search, Sort},
        state::{CommunicationMatch, State},
        RequestError,
    },
};

use super::misc::response_with_communication;

#[derive(Deserialize)]
pub struct QueryArgs {
    sent: Option<Search<DateTime<Utc>>>,
    received: Option<Search<Option<DateTime<Utc>>>>,
    recipient: Option<Search<String>>,
    sender: Option<Search<String>>,

    #[serde(rename = "_sort")]
    sort: Option<Sort<SortArgs>>,
}

pub enum SortArgs {
    Sent,
    Received,
}

impl FromStr for SortArgs {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sent" => Ok(Self::Sent),
            "received" => Ok(Self::Received),
            _ => Err(()),
        }
    }
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

    // Find all communications
    let mut communications: Vec<&Communication> = Vec::new();
    for communication in state.iter_communications(kvnr, telematik_id) {
        match communication {
            CommunicationMatch::Sender(c) => {
                if check_query(&query, c) {
                    communications.push(c);
                }
            }
            CommunicationMatch::Recipient(c) => {
                if check_query(&query, c) {
                    if c.received().is_none() {
                        c.set_received(Utc::now().into());
                    }

                    communications.push(c);
                }
            }
            _ => unreachable!(),
        }
    }

    // Sort the result
    if let Some(sort) = &query.sort {
        communications.sort_by(|a, b| {
            sort.cmp(|arg| match arg {
                SortArgs::Sent => {
                    let a: Option<DateTime<Utc>> = a.sent().clone().map(Into::into);
                    let b: Option<DateTime<Utc>> = b.sent().clone().map(Into::into);

                    a.cmp(&b)
                }
                SortArgs::Received => {
                    let a: Option<DateTime<Utc>> = a.received().clone().map(Into::into);
                    let b: Option<DateTime<Utc>> = b.received().clone().map(Into::into);

                    a.cmp(&b)
                }
            })
        });
    }

    // Generate the response
    let mut bundle = Bundle::new(Type::Searchset);
    for c in communications {
        bundle.entries.push(Entry::new(c));
    }

    match accept {
        #[cfg(feature = "support-xml")]
        DataType::Xml => {
            let xml = bundle.xml()?;

            Ok(HttpResponse::Ok()
                .content_type(DataType::Xml.as_mime().to_string())
                .body(xml))
        }

        #[cfg(feature = "support-json")]
        DataType::Json => {
            let json = bundle.json()?;

            Ok(HttpResponse::Ok()
                .content_type(DataType::Json.as_mime().to_string())
                .body(json))
        }

        DataType::Any | DataType::Unknown => panic!("Data type of response was not specified"),
    }
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

    response_with_communication(communication, accept, false)
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
        let creceived = communication.received().clone().map(Into::into);
        if !qreceived.matches(&creceived) {
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
