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

use crate::{
    service::{
        header::{Accept, Authorization},
        misc::{create_response, DataType, FromQuery, Profession, Query, QueryValue, Search, Sort},
        IntoReqErrResult, TypedRequestError, TypedRequestResult,
    },
    state::State,
};

use super::CommunicationRefMut;

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
            "received" => self.received.push(value.ok()?.parse()?),
            "sender" => self.sender.push(value.ok()?.parse()?),
            "recipient" => self.recipient.push(value.ok()?.parse()?),
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

#[allow(clippy::match_like_matches_macro)]
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
        .into_req_err()
        .err_with_type(accept)?;

    let query = query.0;
    let participant_id = access_token.id().into_req_err().err_with_type(accept)?;

    // Find all communications
    let mut state = state.lock().await;
    let mut communications = state
        .communication_iter_mut(participant_id, |c| check_query(&query, c))
        .collect::<Vec<_>>();

    // Sort the result
    if let Some(sort) = &query.sort {
        communications.sort_by(|a, b| {
            let a = &*a;
            let b = &*b;

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
        let c = match c {
            CommunicationRefMut::Sender(c) => &*c,
            CommunicationRefMut::Recipient(c) => {
                if c.received().is_none() {
                    c.set_received(Utc::now().into());
                }

                &*c
            }
        };

        bundle.entries.push(Entry::new(c));
    }

    create_response(&bundle, accept)
}

#[allow(clippy::match_like_matches_macro)]
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
        .into_req_err()
        .err_with_type(accept)?;

    let id = id.into_inner();
    let participant_id = access_token.id().into_req_err().err_with_type(accept)?;

    let mut state = state.lock().await;
    let communication = state
        .communication_get_mut(id, &participant_id)
        .into_req_err()
        .err_with_type(accept)?;

    let communication = match communication {
        CommunicationRefMut::Sender(c) => &*c,
        CommunicationRefMut::Recipient(c) => {
            if c.received().is_none() {
                c.set_received(Utc::now().into());
            }

            &*c
        }
    };

    create_response(communication, accept)
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
