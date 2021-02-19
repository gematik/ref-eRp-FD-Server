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
    HttpRequest, HttpResponse,
};
use chrono::{DateTime, Utc};
use resources::{
    audit_event::{AuditEvent, SubType},
    bundle::{Bundle, Entry, Relation, Type},
    primitives::Id,
};

use crate::{
    service::{
        header::{Accept, Authorization},
        misc::{create_response, DataType, FromQuery, Profession, Query, QueryValue, Search, Sort},
        AsReqErrResult, TypedRequestError, TypedRequestResult,
    },
    state::State,
};

#[derive(Default, Debug)]
pub struct QueryArgs {
    date: Vec<Search<DateTime<Utc>>>,
    agent: Vec<Search<String>>,
    sub_type: Vec<Search<SubType>>,
    sort: Option<Sort<SortArgs>>,
    count: Option<usize>,
    page_id: Option<usize>,
}

impl FromQuery for QueryArgs {
    fn parse_key_value_pair(&mut self, key: &str, value: QueryValue<'_>) -> Result<(), String> {
        match key {
            "date" => self.date.push(value.ok()?.parse()?),
            "agent" => self.agent.push(value.ok()?.parse()?),
            "subType" | "subtype" | "sub-type" => self.sub_type.push(value.ok()?.parse()?),
            "_sort" => self.sort = Some(value.ok()?.parse()?),
            "_count" => self.count = Some(value.ok()?.parse::<usize>().map_err(|e| e.to_string())?),
            "pageId" | "page-id" => {
                self.page_id = Some(value.ok()?.parse::<usize>().map_err(|e| e.to_string())?)
            }
            _ => (),
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum SortArgs {
    Date,
    Agent,
    SubType,
}

impl FromStr for SortArgs {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "date" => Ok(Self::Date),
            "agent" => Ok(Self::Agent),
            "subType" | "subtype" | "sub-type" => Ok(Self::SubType),
            _ => Err(()),
        }
    }
}

pub async fn get_all(
    state: Data<State>,
    request: HttpRequest,
    accept: Accept,
    access_token: Authorization,
    query: Query<QueryArgs>,
) -> Result<HttpResponse, TypedRequestError> {
    let mut query = query.0;
    let accept = DataType::from_accept(&accept)
        .and_then(DataType::ignore_any)
        .unwrap_or_default()
        .check_supported()
        .err_with_type_default()?;

    access_token
        .check_profession(|p| matches!(p, Profession::Versicherter))
        .as_req_err()
        .err_with_type(accept)?;

    let kvnr = access_token.kvnr().as_req_err().err_with_type(accept)?;

    let state = state.lock().await;
    let mut events: Vec<&AuditEvent> = state
        .audit_event_iter(&kvnr, |av| check_query(&query, av))
        .collect();

    // Sort the result
    if let Some(sort) = &query.sort {
        events.sort_by(|a, b| {
            sort.cmp(|arg| match arg {
                SortArgs::Date => {
                    let a = &a.recorded;
                    let b = &b.recorded;

                    a.cmp(&b)
                }
                SortArgs::Agent => {
                    let a = &a.agent.who;
                    let b = &b.agent.who;

                    a.cmp(&b)
                }
                SortArgs::SubType => {
                    let a = &a.sub_type;
                    let b = &b.sub_type;

                    a.cmp(&b)
                }
            })
        });
    }

    // Handle paging
    if events.len() > MAX_COUNT {
        query.count = match query.count {
            Some(c) if c < MAX_COUNT => Some(c),
            _ => Some(MAX_COUNT),
        }
    }

    let page_id = query.page_id.unwrap_or_default();
    let (skip, take) = if let Some(count) = query.count {
        (page_id * count, count)
    } else {
        (0, usize::MAX)
    };

    let result_count = events.len();
    let mut bundle = Bundle::new(Type::Searchset);
    bundle.total = Some(result_count);

    for result in events.into_iter().skip(skip).take(take) {
        let mut entry = Entry::new(result);
        entry.url = Some(format!("/AuditEvent/{}", &result.id));

        bundle.entries.push(entry);
    }

    if let Some(count) = query.count {
        let query_str = request.query_string();
        let page_count = ((result_count - 1) / count) + 1;

        bundle
            .link
            .push((Relation::Self_, make_uri(query_str, page_id)));
        bundle.link.push((Relation::First, make_uri(query_str, 0)));
        bundle
            .link
            .push((Relation::Last, make_uri(query_str, page_count - 1)));

        if page_id > 0 {
            bundle
                .link
                .push((Relation::Previous, make_uri(query_str, page_id - 1)));
        }

        if page_id + 1 < page_count {
            bundle
                .link
                .push((Relation::Next, make_uri(query_str, page_id + 1)));
        }
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
        .check_profession(|p| matches!(p, Profession::Versicherter))
        .as_req_err()
        .err_with_type(accept)?;

    let id = id.into_inner();
    let kvnr = access_token.kvnr().as_req_err().err_with_type(accept)?;

    let state = state.lock().await;
    let event = state
        .audit_event_get(id, &kvnr)
        .as_req_err()
        .err_with_type(accept)?;

    create_response(event, accept)
}

fn check_query(query: &QueryArgs, event: &AuditEvent) -> bool {
    for qsent in &query.date {
        if !qsent.matches(&event.recorded) {
            return false;
        }
    }

    for qreceived in &query.agent {
        if let Some(who) = &event.agent.who {
            if !qreceived.matches(who.as_string()) {
                return false;
            }
        } else {
            return false;
        }
    }

    for qsender in &query.sub_type {
        if !qsender.matches(&event.sub_type) {
            return false;
        }
    }

    true
}

fn make_uri(query: &str, page_id: usize) -> String {
    if query.is_empty() {
        format!("/AuditEvent?pageId={}", page_id)
    } else {
        format!("/AuditEvent?{}&pageId={}", query, page_id)
    }
}

const MAX_COUNT: usize = 50;
