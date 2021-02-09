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
    primitives::Id,
    task::Status,
    Task,
};
use serde::Deserialize;

use crate::{
    service::{
        header::{Accept, Authorization, XAccessCode},
        misc::{
            create_response, AccessToken, DataType, FromQuery, Profession, Query, QueryValue,
            Search, Sort,
        },
        AsReqErrResult, TypedRequestError, TypedRequestResult,
    },
    state::{Inner as StateInner, State, Version},
};

use super::{misc::Resource, Error};

#[derive(Default)]
pub struct QueryArgs {
    status: Vec<Search<Status>>,
    authored_on: Vec<Search<DateTime<Utc>>>,
    last_modified: Vec<Search<DateTime<Utc>>>,
    sort: Option<Sort<SortArgs>>,
    count: Option<usize>,
    page_id: Option<usize>,
}

#[derive(Deserialize)]
pub struct PathArgs {
    id: Id,
    version: usize,
}

impl FromQuery for QueryArgs {
    fn parse_key_value_pair(&mut self, key: &str, value: QueryValue) -> Result<(), String> {
        match key {
            "status" => self.status.push(value.ok()?.parse()?),
            "authoredOn" => self.authored_on.push(value.ok()?.parse()?),
            "lastModified" => self.last_modified.push(value.ok()?.parse()?),
            "_sort" => self.sort = Some(value.ok()?.parse()?),
            "_count" => self.count = Some(value.ok()?.parse::<usize>().map_err(|e| e.to_string())?),
            "pageId" => {
                self.page_id = Some(value.ok()?.parse::<usize>().map_err(|e| e.to_string())?)
            }
            _ => (),
        }

        Ok(())
    }
}

pub enum SortArgs {
    AuthoredOn,
    LastModified,
}

impl FromStr for SortArgs {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "authoredOn" => Ok(Self::AuthoredOn),
            "lastModified" => Ok(Self::LastModified),
            _ => Err(()),
        }
    }
}

enum TaskReference {
    All,
    One(Id),
    Version(Id, usize),
}

pub async fn get_all(
    state: Data<State>,
    accept: Accept,
    id_token: Authorization,
    access_code: Option<XAccessCode>,
    query: Query<QueryArgs>,
    request: HttpRequest,
) -> Result<HttpResponse, TypedRequestError> {
    get(
        &state,
        TaskReference::All,
        accept,
        id_token,
        access_code,
        query.0,
        request.query_string(),
    )
    .await
}

pub async fn get_one(
    state: Data<State>,
    id: Path<Id>,
    accept: Accept,
    id_token: Authorization,
    access_code: Option<XAccessCode>,
) -> Result<HttpResponse, TypedRequestError> {
    get(
        &state,
        TaskReference::One(id.into_inner()),
        accept,
        id_token,
        access_code,
        QueryArgs::default(),
        "",
    )
    .await
}

pub async fn get_version(
    state: Data<State>,
    args: Path<PathArgs>,
    accept: Accept,
    id_token: Authorization,
    access_code: Option<XAccessCode>,
) -> Result<HttpResponse, TypedRequestError> {
    let PathArgs { id, version } = args.into_inner();

    get(
        &state,
        TaskReference::Version(id, version),
        accept,
        id_token,
        access_code,
        QueryArgs::default(),
        "",
    )
    .await
}

#[allow(unreachable_code)]
async fn get(
    state: &State,
    reference: TaskReference,
    accept: Accept,
    access_token: Authorization,
    access_code: Option<XAccessCode>,
    query: QueryArgs,
    query_str: &str,
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

    let kvnr = Some(access_token.kvnr().as_req_err().err_with_type(accept)?);
    let agent = (&*access_token).into();
    let mut state = state.lock().await;

    match reference {
        TaskReference::One(id) => {
            let (state, task) = state
                .task_get(id, None, kvnr, access_code, agent)
                .as_req_err()
                .err_with_type(accept)?;

            let mut bundle = Bundle::new(Type::Searchset);
            add_task_to_bundle(&mut bundle, &task, &access_token, Some(&state))
                .as_req_err()
                .err_with_type(accept)?;

            create_response(&bundle, accept)
        }
        TaskReference::Version(id, version_id) => {
            let (_, task) = state
                .task_get(id, Some(version_id), kvnr, access_code, agent)
                .as_req_err()
                .err_with_type(accept)?;

            let mut bundle = Bundle::new(Type::Searchset);
            add_task_to_bundle(&mut bundle, &task, &access_token, None)
                .as_req_err()
                .err_with_type(accept)?;

            create_response(&bundle, accept)
        }
        TaskReference::All => {
            let mut tasks = state
                .task_iter(kvnr, access_code, agent, |t| check_query(&query, t))
                .collect::<Vec<_>>();

            // Sort the result
            if let Some(sort) = &query.sort {
                tasks.sort_by(|a, b| {
                    let a = a.unlogged();
                    let b = b.unlogged();

                    sort.cmp(|arg| match arg {
                        SortArgs::AuthoredOn => {
                            let a: Option<DateTime<Utc>> =
                                a.resource.authored_on.clone().map(Into::into);
                            let b: Option<DateTime<Utc>> =
                                b.resource.authored_on.clone().map(Into::into);

                            a.cmp(&b)
                        }
                        SortArgs::LastModified => {
                            let a: Option<DateTime<Utc>> =
                                a.resource.last_modified.clone().map(Into::into);
                            let b: Option<DateTime<Utc>> =
                                b.resource.last_modified.clone().map(Into::into);

                            a.cmp(&b)
                        }
                    })
                });
            }

            // Create the bundle
            let page_id = query.page_id.unwrap_or_default();
            let (skip, take) = if let Some(count) = query.count {
                (page_id * count, count)
            } else {
                (0, usize::MAX)
            };

            let mut bundle = Bundle::new(Type::Searchset);
            for task in tasks.iter().skip(skip).take(take) {
                add_task_to_bundle(&mut bundle, &task, &access_token, None)
                    .as_req_err()
                    .err_with_type(accept)?;
            }

            bundle.total = Some(tasks.len());

            if let Some(count) = query.count {
                let page_count = ((tasks.len() - 1) / count) + 1;

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
    }
}

fn check_query(query: &QueryArgs, task: &Task) -> bool {
    for status in &query.status {
        if !status.matches(&task.status) {
            return false;
        }
    }

    for authored_on in &query.authored_on {
        if let Some(task_authored_on) = &task.authored_on {
            if !authored_on.matches(&task_authored_on.clone().into()) {
                return false;
            }
        }
    }

    for last_modified in &query.last_modified {
        if let Some(task_last_modified) = &task.last_modified {
            if !last_modified.matches(&task_last_modified.clone().into()) {
                return false;
            }
        }
    }

    true
}

fn make_uri(query: &str, page_id: usize) -> String {
    if query.is_empty() {
        format!("/Task?pageId={}", page_id)
    } else {
        format!("/Task?{}&pageId={}", query, page_id)
    }
}

fn add_task_to_bundle<'b, 's>(
    bundle: &'b mut Bundle<Resource<'s>>,
    task: &'s Version<Task>,
    access_token: &AccessToken,
    state: Option<&'s StateInner>,
) -> Result<(), Error>
where
    's: 'b,
{
    bundle.entries.push(Entry::new(Resource::TaskVersion(task)));

    let state = match state {
        Some(state) => state,
        None => return Ok(()),
    };

    #[cfg(feature = "interface-supplier")]
    {
        if access_token.is_pharmacy() {
            if let Some(id) = task.resource.output.receipt.as_ref() {
                let receipt = state
                    .erx_receipts
                    .get(id)
                    .ok_or_else(|| Error::ErxReceiptNotFound(id.clone()))?;

                bundle
                    .entries
                    .push(Entry::new(Resource::ErxBundle(receipt)));
            }
        }
    }

    #[cfg(feature = "interface-patient")]
    {
        if access_token.is_patient() {
            if let Some(id) = task.resource.input.patient_receipt.as_ref() {
                let patient_receipt = state
                    .patient_receipts
                    .get(id)
                    .ok_or_else(|| Error::PatientReceiptNotFound(id.clone()))?;

                bundle
                    .entries
                    .push(Entry::new(Resource::KbvBundle(patient_receipt)));
            }

            if let Some(id) = task.resource.output.receipt.as_ref() {
                let receipt = state
                    .erx_receipts
                    .get(id)
                    .ok_or_else(|| Error::ErxReceiptNotFound(id.clone()))?;

                bundle
                    .entries
                    .push(Entry::new(Resource::ErxBundle(receipt)));
            }
        }
    }

    Ok(())
}
