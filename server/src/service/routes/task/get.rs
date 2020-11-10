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
    HttpRequest, HttpResponse,
};
use chrono::{DateTime, Utc};
use resources::{
    bundle::{Bundle, Entry, Relation, Type},
    primitives::Id,
    task::Status,
    KbvBundle, Task,
};
use serde::Deserialize;

#[cfg(feature = "support-json")]
use crate::fhir::encode::JsonEncode;
#[cfg(feature = "support-xml")]
use crate::fhir::encode::XmlEncode;
use crate::{
    fhir::{
        definitions::EncodeBundleResource,
        encode::{DataStorage, Encode, EncodeError, EncodeStream},
    },
    service::{
        header::{Accept, Authorization, XAccessCode},
        misc::{DataType, Profession, Search, Sort},
        state::State,
        RequestError,
    },
};

#[derive(Default, Deserialize)]
pub struct QueryArgs {
    status: Option<Search<Status>>,
    authored_on: Option<Search<DateTime<Utc>>>,
    last_modified: Option<Search<DateTime<Utc>>>,

    #[serde(rename = "_sort")]
    sort: Option<Sort<SortArgs>>,

    #[serde(rename = "_count")]
    count: Option<usize>,

    #[serde(rename = "pageId")]
    page_id: Option<usize>,
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

pub async fn get_all(
    state: Data<State>,
    accept: Accept,
    id_token: Authorization,
    access_code: Option<XAccessCode>,
    query: Query<QueryArgs>,
    request: HttpRequest,
) -> Result<HttpResponse, RequestError> {
    get(
        &state,
        None,
        accept,
        id_token,
        access_code,
        query.into_inner(),
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
) -> Result<HttpResponse, RequestError> {
    get(
        &state,
        Some(id.into_inner()),
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
    id: Option<Id>,
    accept: Accept,
    access_token: Authorization,
    access_code: Option<XAccessCode>,
    query: QueryArgs,
    query_str: &str,
) -> Result<HttpResponse, RequestError> {
    access_token.check_profession(|p| p == Profession::Versicherter)?;

    let kvnr = access_token.kvnr().ok();
    let accept = DataType::from_accept(accept)
        .and_then(DataType::ignore_any)
        .unwrap_or_default()
        .check_supported()?;

    let state = state.lock().await;

    let mut tasks = Vec::new();
    let is_collection = if let Some(id) = id {
        match state.get_task(&id, &kvnr, &access_code) {
            Some(Ok(task)) => tasks.push(task),
            Some(Err(())) => return Ok(HttpResponse::Forbidden().finish()),
            None => return Ok(HttpResponse::NotFound().finish()),
        }

        false
    } else {
        for task in state.iter_tasks(kvnr, access_code) {
            if check_query(&query, task) {
                tasks.push(task);
            }
        }

        true
    };

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

    // Create the response
    let page_id = query.page_id.unwrap_or_default();
    let (skip, take) = if let Some(count) = query.count {
        (page_id * count, count)
    } else {
        (0, usize::MAX)
    };

    let mut bundle = Bundle::new(Type::Searchset);
    for task in tasks.iter().skip(skip).take(take) {
        bundle.entries.push(Entry::new(Resource::Task(task)));

        if let Some(id) = task.input.e_prescription.as_ref() {
            if let Some(res) = state.e_prescriptions.get(id) {
                bundle.entries.push(Entry::new(Resource::Bundle(res)));
            }
        }

        if let Some(id) = task.input.patient_receipt.as_ref() {
            if let Some(res) = state.patient_receipts.get(id) {
                bundle.entries.push(Entry::new(Resource::Bundle(res)));
            }
        }
    }

    if is_collection {
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

fn check_query(query: &QueryArgs, task: &Task) -> bool {
    if let Some(status) = &query.status {
        if !status.matches(&task.status) {
            return false;
        }
    }

    if let Some(authored_on) = &query.authored_on {
        if let Some(task_authored_on) = &task.authored_on {
            if !authored_on.matches(&task_authored_on.clone().into()) {
                return false;
            }
        }
    }

    if let Some(last_modified) = &query.last_modified {
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

#[derive(Clone)]
enum Resource<'a> {
    Task(&'a Task),
    Bundle(&'a KbvBundle),
}

impl EncodeBundleResource for Resource<'_> {}

impl Encode for Resource<'_> {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        match self {
            Self::Task(v) => v.encode(stream),
            Self::Bundle(v) => v.encode(stream),
        }
    }
}
