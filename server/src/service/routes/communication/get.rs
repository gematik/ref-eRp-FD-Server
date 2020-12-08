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
    web::{Data, Path},
    HttpResponse,
};
use chrono::{DateTime, Utc};
use resources::{
    bundle::{Bundle, Entry, Type},
    primitives::Id,
    Communication,
};

use crate::service::{
    header::{Accept, Authorization},
    misc::{create_response, DataType, FromQuery, Profession, Query, QueryValue, Search, Sort},
    state::{CommunicationMatch, State},
    AsReqErr, AsReqErrResult, TypedRequestError, TypedRequestResult,
};

use super::Error;

#[derive(Default, Debug)]
pub struct QueryArgs {
    sent: Vec<Search<DateTime<Utc>>>,
    received: Vec<Search<Option<DateTime<Utc>>>>,
    sender: Vec<Search<String>>,
    recipient: Vec<Search<String>>,
    sort: Option<Sort<SortArgs>>,
}

impl FromQuery for QueryArgs {
    fn parse_key_value_pair(&mut self, key: &str, value: QueryValue<'_>) -> Result<(), String> {
        match key {
            "sent" => self.sent.push(value.ok()?.parse()?),
            "received" => self.sent.push(value.ok()?.parse()?),
            "sender" => self.sent.push(value.ok()?.parse()?),
            "recipient" => self.sent.push(value.ok()?.parse()?),
            "_sort" => self.sort = Some(value.ok()?.parse()?),
            _ => (),
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum SortArgs {
    Sent,
    Received,
    Sender,
    Recipient,
}

impl FromStr for SortArgs {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sent" => Ok(Self::Sent),
            "received" => Ok(Self::Received),
            "sender" => Ok(Self::Sender),
            "recipient" => Ok(Self::Recipient),
            _ => Err(()),
        }
    }
}

pub async fn get_all(
    state: Data<State>,
    accept: Accept,
    access_token: Authorization,
    query: Query<QueryArgs>,
) -> Result<HttpResponse, TypedRequestError> {
    let accept = DataType::from_accept(&accept)
        .and_then(DataType::ignore_any)
        .unwrap_or_default()
        .check_supported()
        .err_with_type_default()?;

    access_token
        .check_profession(|p| match p {
            Profession::Versicherter => true,
            Profession::KrankenhausApotheke => true,
            Profession::OeffentlicheApotheke => true,
            _ => false,
        })
        .as_req_err()
        .err_with_type(accept)?;

    let query = query.0;
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
                SortArgs::Sender => {
                    let a = a.sender();
                    let b = b.sender();

                    a.cmp(&b)
                }
                SortArgs::Recipient => {
                    let a = a.recipient();
                    let b = b.recipient();

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

    create_response(&bundle, accept)
}

pub async fn get_one(
    state: Data<State>,
    id: Path<Id>,
    accept: Accept,
    access_token: Authorization,
) -> Result<HttpResponse, TypedRequestError> {
    let accept = DataType::from_accept(&accept)
        .and_then(DataType::ignore_any)
        .unwrap_or_default()
        .check_supported()
        .err_with_type_default()?;

    access_token
        .check_profession(|p| match p {
            Profession::Versicherter => true,
            Profession::KrankenhausApotheke => true,
            Profession::OeffentlicheApotheke => true,
            _ => false,
        })
        .as_req_err()
        .err_with_type(accept)?;

    let id = id.into_inner();
    let kvnr = access_token.kvnr().ok();
    let telematik_id = access_token.telematik_id().ok();

    let mut state = state.lock().await;
    let communication = match state.get_communication(&id, &kvnr, &telematik_id) {
        CommunicationMatch::NotFound => {
            return Err(Error::NotFound(id).as_req_err().with_type(accept))
        }
        CommunicationMatch::Unauthorized => {
            return Err(Error::Unauthorized(id).as_req_err().with_type(accept))
        }
        CommunicationMatch::Sender(c) => c,
        CommunicationMatch::Recipient(c) => {
            if c.received().is_none() {
                c.set_received(Utc::now().into());
            }

            c
        }
    };

    create_response(&*communication, accept)
}

fn check_query(query: &QueryArgs, communication: &Communication) -> bool {
    for qsent in &query.sent {
        if let Some(csent) = communication.sent() {
            let csent = csent.clone().into();
            if !qsent.matches(&csent) {
                return false;
            }
        } else {
            return false;
        }
    }

    for qreceived in &query.received {
        let creceived = communication.received().clone().map(Into::into);
        if !qreceived.matches(&creceived) {
            return false;
        }
    }

    for qsender in &query.sender {
        if let Some(csender) = communication.sender() {
            if !qsender.matches(&csender) {
                return false;
            }
        } else {
            return false;
        }
    }

    for qrecipient in &query.recipient {
        let crecipient = communication.recipient();
        if !qrecipient.matches(&crecipient) {
            return false;
        }
    }

    true
}
