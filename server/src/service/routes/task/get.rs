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

use std::borrow::ToOwned;
use std::str::FromStr;

use actix_web::{
    web::{Data, Path},
    HttpRequest, HttpResponse,
};
use chrono::{DateTime, Utc};
use resources::{
    audit_event::Language,
    bundle::{Bundle, Entry, Relation, Type},
    primitives::Id,
    task::Status,
    Task,
};

use crate::{
    service::{
        header::{Accept, AcceptLanguage, Authorization, XAccessCode},
        misc::{
            create_response, make_page_uri, AccessToken, DataType, FromQuery, Profession, Query,
            QueryValue, Search, Sort,
        },
        IntoReqErr, IntoReqErrResult, TypedRequestError, TypedRequestResult,
    },
    state::{Inner as StateInner, State},
};

use super::{misc::Resource, Error};

#[derive(Default)]
pub struct GetOneQueryArgs {
    secret: Option<String>,
    rev_include: Vec<IncludeArgs>,
}

#[derive(Default)]
pub struct GetAllQueryArgs {
    status: Vec<Search<Status>>,
    authored_on: Vec<Search<DateTime<Utc>>>,
    last_modified: Vec<Search<DateTime<Utc>>>,
    sort: Option<Sort<SortArgs>>,
    count: Option<usize>,
    page_id: Option<usize>,
}

#[derive(Default, Debug)]
pub struct IncludeArgs {
    source: String,
    path: String,
    target: Option<String>,
}

impl FromQuery for GetOneQueryArgs {
    #[allow(clippy::single_match)]
    fn parse_key_value_pair(&mut self, key: &str, value: QueryValue) -> Result<(), String> {
        match key {
            "secret" => self.secret = Some(value.ok()?.to_owned()),
            "_revinclude" => {
                let mut it = value.ok()?.split(':');

                let source = it.next().ok_or("Invalid _revinclude")?.to_owned();
                let path = it.next().ok_or("Invalid _revinclude")?.to_owned();
                let target = it.next().map(ToOwned::to_owned);

                if it.next().is_some() {
                    return Err("Invalid _revinclude".into());
                }

                self.rev_include.push(IncludeArgs {
                    source,
                    path,
                    target,
                });
            }
            _ => (),
        }

        Ok(())
    }
}

impl FromQuery for GetAllQueryArgs {
    fn parse_key_value_pair(&mut self, key: &str, value: QueryValue) -> Result<(), String> {
        match key {
            "status" => self.status.push(value.ok()?.parse()?),
            "authoredOn" | "authored-on" | "authoredon" => {
                self.authored_on.push(value.ok()?.parse()?)
            }
            "lastModified" | "last-modified" | "modified" => {
                self.last_modified.push(value.ok()?.parse()?)
            }
            "_sort" => self.sort = Some(value.ok()?.parse()?),
            "_count" => self.count = Some(value.ok()?.parse::<usize>().map_err(|e| e.to_string())?),
            "pageId" | "page-id" | "pageid" => {
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
            "authoredOn" | "authored-on" | "authoredon" => Ok(Self::AuthoredOn),
            "lastModified" | "last-modified" | "modified" => Ok(Self::LastModified),
            _ => Err(()),
        }
    }
}

enum TaskReference {
    All(GetAllQueryArgs),
    One(Id, GetOneQueryArgs),
}

pub async fn get_all(
    state: Data<State>,
    accept: Accept,
    accept_language: AcceptLanguage,
    access_token: Authorization,
    access_code: Option<XAccessCode>,
    query: Query<GetAllQueryArgs>,
    request: HttpRequest,
) -> Result<HttpResponse, TypedRequestError> {
    if let Err(err) = access_token.check_profession(|p| p == Profession::Versicherter) {
        let accept = DataType::from_accept(&accept)
            .and_then(DataType::ignore_any)
            .unwrap_or_default()
            .check_supported()
            .err_with_type_default()?;

        return Err(err.into_req_err().with_type(accept));
    }

    get(
        &state,
        TaskReference::All(query.0),
        accept,
        accept_language,
        access_token,
        access_code,
        request.query_string(),
    )
    .await
}

pub async fn get_one(
    state: Data<State>,
    id: Path<Id>,
    accept: Accept,
    accept_language: AcceptLanguage,
    access_token: Authorization,
    access_code: Option<XAccessCode>,
    query: Query<GetOneQueryArgs>,
) -> Result<HttpResponse, TypedRequestError> {
    get(
        &state,
        TaskReference::One(id.into_inner(), query.0),
        accept,
        accept_language,
        access_token,
        access_code,
        "",
    )
    .await
}

async fn get(
    state: &State,
    reference: TaskReference,
    accept: Accept,
    accept_language: AcceptLanguage,
    access_token: Authorization,
    mut access_code: Option<XAccessCode>,
    query_str: &str,
) -> Result<HttpResponse, TypedRequestError> {
    let accept = DataType::from_accept(&accept)
        .and_then(DataType::ignore_any)
        .unwrap_or_default()
        .check_supported()
        .err_with_type_default()?;

    access_token
        .check_profession(|p| {
            p == Profession::Versicherter
                || p == Profession::OeffentlicheApotheke
                || p == Profession::KrankenhausApotheke
        })
        .into_req_err()
        .err_with_type(accept)?;

    let mut kvnr = access_token.kvnr().ok();
    let agent = (&*access_token).into();
    let lang: Language = accept_language.into();

    #[cfg(feature = "interface-supplier")]
    {
        if access_token.is_pharmacy() {
            kvnr = None;
            access_code = None;
        }
    }

    let mut state = state.lock().await;

    match reference {
        TaskReference::One(id, query) => {
            let (state, task) = state
                .task_get(id.clone(), kvnr, access_code, query.secret, agent)
                .into_req_err()
                .err_with_type(accept)?;

            let mut bundle = Bundle::new(Type::Collection);
            add_to_bundle(&mut bundle, &task, &access_token, Some(&state))
                .into_req_err()
                .err_with_type(accept)?;

            for inc in &query.rev_include {
                let inc_audit_event = matches!(
                    inc,
                    IncludeArgs {
                        ref source,
                        ref path,
                        ref target,
                    } if source == "AuditEvent"
                        && path == "entity.what"
                        && (target.is_none() || target.as_deref() == Some("AuditEvent")));

                if inc_audit_event {
                    if let Some(events) = state.audit_event_iter_by_task(&id) {
                        for event in events {
                            bundle
                                .entries
                                .push(Entry::new(Resource::AuditEvent(event, lang)));
                        }
                    }
                }
            }

            create_response(&bundle, accept)
        }
        TaskReference::All(query) => {
            let mut tasks = state
                .task_iter(kvnr, access_code, agent, |t| check_query(&query, t))
                .collect::<Vec<_>>();

            // Sort the result
            if let Some(sort) = &query.sort {
                tasks.sort_by(|a, b| {
                    sort.cmp(|arg| match arg {
                        SortArgs::AuthoredOn => {
                            let a: Option<DateTime<Utc>> = a.authored_on.clone().map(Into::into);
                            let b: Option<DateTime<Utc>> = b.authored_on.clone().map(Into::into);

                            a.cmp(&b)
                        }
                        SortArgs::LastModified => {
                            let a: Option<DateTime<Utc>> = a.last_modified.clone().map(Into::into);
                            let b: Option<DateTime<Utc>> = b.last_modified.clone().map(Into::into);

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
                add_to_bundle(&mut bundle, &task, &access_token, None)
                    .into_req_err()
                    .err_with_type(accept)?;
            }

            bundle.total = Some(tasks.len());

            if let Some(count) = query.count {
                let page_count = ((tasks.len() - 1) / count) + 1;

                bundle
                    .link
                    .push((Relation::Self_, make_page_uri("/Task", query_str, page_id)));
                bundle
                    .link
                    .push((Relation::First, make_page_uri("/Task", query_str, 0)));
                bundle.link.push((
                    Relation::Last,
                    make_page_uri("/Task", query_str, page_count - 1),
                ));

                if page_id > 0 {
                    bundle.link.push((
                        Relation::Previous,
                        make_page_uri("/Task", query_str, page_id - 1),
                    ));
                }

                if page_id + 1 < page_count {
                    bundle.link.push((
                        Relation::Next,
                        make_page_uri("/Task", query_str, page_id + 1),
                    ));
                }
            }

            create_response(&bundle, accept)
        }
    }
}

fn check_query(query: &GetAllQueryArgs, task: &Task) -> bool {
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

fn add_to_bundle<'b, 's>(
    bundle: &'b mut Bundle<Resource<'s>>,
    task: &'s Task,
    access_token: &AccessToken,
    state: Option<&'s StateInner>,
) -> Result<(), Error>
where
    's: 'b,
{
    #[cfg(feature = "interface-supplier")]
    {
        if access_token.is_pharmacy() {
            bundle
                .entries
                .push(Entry::new(Resource::TaskForSupplier(task)));
        }
    }

    #[cfg(feature = "interface-patient")]
    {
        if access_token.is_patient() {
            bundle
                .entries
                .push(Entry::new(Resource::TaskForPatient(task)));
        }
    }

    let state = match state {
        Some(state) => state,
        None => return Ok(()),
    };

    #[cfg(feature = "interface-supplier")]
    {
        if access_token.is_pharmacy() {
            if let Some(id) = task.output.receipt.as_ref() {
                let receipt = state
                    .erx_receipts
                    .get_by_id(id)
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
            if let Some(id) = task.input.patient_receipt.as_ref() {
                let patient_receipt = state
                    .patient_receipts
                    .get_by_id(id)
                    .ok_or_else(|| Error::PatientReceiptNotFound(id.clone()))?;

                bundle
                    .entries
                    .push(Entry::new(Resource::KbvBundle(patient_receipt)));
            }
        }
    }

    Ok(())
}
