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
    bundle::{Bundle, Entry, Relation, Type},
    misc::TelematikId,
    primitives::Id,
    MedicationDispense,
};

use crate::{
    service::{
        header::{Accept, Authorization},
        misc::{create_response, DataType, FromQuery, Profession, Query, QueryValue, Search, Sort},
        AsReqErrResult, TypedRequestError, TypedRequestResult,
    },
    state::State,
};

#[derive(Default)]
pub struct QueryArgs {
    when_handed_over: Vec<Search<DateTime<Utc>>>,
    when_prepared: Vec<Search<DateTime<Utc>>>,
    performer: Vec<Search<TelematikId>>,
    sort: Option<Sort<SortArgs>>,
    count: Option<usize>,
    page_id: Option<usize>,
}

impl FromQuery for QueryArgs {
    fn parse_key_value_pair(&mut self, key: &str, value: QueryValue) -> Result<(), String> {
        match key {
            "whenHandedOver" | "whenhandedover" | "when-handed-over" => {
                self.when_handed_over.push(value.ok()?.parse()?)
            }
            "whenPrepared" | "whenprepared" | "when-prepared" => {
                self.when_prepared.push(value.ok()?.parse()?)
            }
            "performer" => self.performer.push(value.ok()?.parse()?),
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

pub enum SortArgs {
    WhenHandedOver,
    WhenPrepared,
}

impl FromStr for SortArgs {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "whenHandedOver" | "whenhandedover" | "when-handed-over" => Ok(Self::WhenHandedOver),
            "whenPrepared" | "whenprepared" | "when-prepared" => Ok(Self::WhenPrepared),
            _ => Err(()),
        }
    }
}

pub async fn get_all(
    state: Data<State>,
    accept: Accept,
    access_token: Authorization,
    query: Query<QueryArgs>,
    request: HttpRequest,
) -> Result<HttpResponse, TypedRequestError> {
    let accept = DataType::from_accept(&accept)
        .and_then(DataType::ignore_any)
        .unwrap_or_default()
        .check_supported()
        .err_with_type_default()?;

    access_token
        .check_profession(|p| p == Profession::Versicherter)
        .as_req_err()
        .err_with_type(accept)?;

    let kvnr = access_token.kvnr().as_req_err().err_with_type(accept)?;
    let state = state.lock().await;

    // Collect results
    let mut results = state
        .medication_dispense_iter(&kvnr, |md| check_query(&query, md))
        .collect::<Vec<_>>();

    // Sort the result
    if let Some(sort) = &query.sort {
        results.sort_by(|a, b| {
            sort.cmp(|arg| match arg {
                SortArgs::WhenHandedOver => {
                    let a: DateTime<Utc> = a.when_handed_over.clone().into();
                    let b: DateTime<Utc> = b.when_handed_over.clone().into();

                    a.cmp(&b)
                }
                SortArgs::WhenPrepared => {
                    let a: Option<DateTime<Utc>> = a.when_prepared.clone().map(Into::into);
                    let b: Option<DateTime<Utc>> = b.when_prepared.clone().map(Into::into);

                    a.cmp(&b)
                }
            })
        });
    }

    // Handle paging
    let page_id = query.page_id.unwrap_or_default();
    let (skip, take) = if let Some(count) = query.count {
        (page_id * count, count)
    } else {
        (0, usize::MAX)
    };

    let result_count = results.len();
    let mut bundle = Bundle::new(Type::Searchset);
    bundle.total = Some(result_count);

    for result in results.into_iter().skip(skip).take(take) {
        let mut entry = Entry::new(result);
        entry.url = result
            .id
            .as_ref()
            .map(|id| format!("/MedicationDispense/{}", id));

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
        .check_profession(|p| p == Profession::Versicherter)
        .as_req_err()
        .err_with_type(accept)?;

    let id = id.0;
    let kvnr = access_token.kvnr().as_req_err().err_with_type(accept)?;
    let state = state.lock().await;
    let medication_dispense = state
        .medication_dispense_get(id, &kvnr)
        .as_req_err()
        .err_with_type(accept)?;

    create_response(medication_dispense, accept)
}

fn check_query(query: &QueryArgs, value: &MedicationDispense) -> bool {
    for expected in &query.when_handed_over {
        if !expected.matches(&value.when_handed_over.clone().into()) {
            return false;
        }
    }

    for expected in &query.when_prepared {
        match &value.when_prepared {
            Some(actual) if !expected.matches(&actual.clone().into()) => return false,
            _ => (),
        }
    }

    for expected in &query.performer {
        if !expected.matches(&value.performer) {
            return false;
        }
    }

    true
}

fn make_uri(query: &str, page_id: usize) -> String {
    if query.is_empty() {
        format!("/MedicationDispense?pageId={}", page_id)
    } else {
        format!("/MedicationDispense?{}&pageId={}", query, page_id)
    }
}
