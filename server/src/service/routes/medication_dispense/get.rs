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
    misc::TelematikId,
    primitives::Id,
};
use serde::Deserialize;

#[cfg(feature = "support-json")]
use crate::fhir::encode::JsonEncode;
#[cfg(feature = "support-xml")]
use crate::fhir::encode::XmlEncode;
use crate::service::{
    header::{Accept, Authorization},
    misc::{DataType, Profession, Search, Sort},
    state::State,
    RequestError,
};

#[derive(Default, Deserialize)]
pub struct QueryArgs {
    when_handed_over: Option<Search<DateTime<Utc>>>,
    when_prepared: Option<Search<DateTime<Utc>>>,
    performer: Option<Search<TelematikId>>,

    #[serde(rename = "_sort")]
    sort: Option<Sort<SortArgs>>,

    #[serde(rename = "_count")]
    count: Option<usize>,

    #[serde(rename = "pageId")]
    page_id: Option<usize>,
}

pub enum SortArgs {
    WhenHandedOver,
    WhenPrepared,
}

impl FromStr for SortArgs {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "whenHandedOver" => Ok(Self::WhenHandedOver),
            "whenPrepared" => Ok(Self::WhenPrepared),
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
) -> Result<HttpResponse, RequestError> {
    access_token.check_profession(|p| p == Profession::Versicherter)?;

    let kvnr = access_token.kvnr()?;
    let accept = DataType::from_accept(accept)
        .and_then(DataType::ignore_any)
        .unwrap_or_default()
        .check_supported()?;

    let state = state.lock().await;

    // Collect results
    let mut results = Vec::new();
    for medication_dispense in state.medication_dispense.values() {
        if medication_dispense.subject != kvnr {
            continue;
        }

        match &query.when_handed_over {
            Some(when_handed_over)
                if when_handed_over
                    .matches(&medication_dispense.when_handed_over.clone().into()) => {}
            Some(_) => continue,
            None => (),
        }

        match (&query.when_prepared, &medication_dispense.when_prepared) {
            (Some(expected), Some(actual)) if expected.matches(&actual.clone().into()) => (),
            (Some(_), _) => continue,
            (None, _) => (),
        }

        match &query.performer {
            Some(performer) if performer.matches(&medication_dispense.performer) => (),
            Some(_) => continue,
            None => (),
        }

        results.push(medication_dispense);
    }

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

    // Create the response
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
    access_token.check_profession(|p| p == Profession::Versicherter)?;

    let kvnr = access_token.kvnr()?;
    let accept = DataType::from_accept(accept)
        .and_then(DataType::ignore_any)
        .unwrap_or_default()
        .check_supported()?;

    let state = state.lock().await;
    let medication_dispense = match state.medication_dispense.get(&id) {
        Some(medication_dispense) => medication_dispense,
        None => return Ok(HttpResponse::NotFound().finish()),
    };

    if medication_dispense.subject != kvnr {
        return Ok(HttpResponse::Forbidden().finish());
    }

    match accept {
        #[cfg(feature = "support-xml")]
        DataType::Xml => {
            let xml = medication_dispense.xml()?;

            Ok(HttpResponse::Ok()
                .content_type(DataType::Xml.as_mime().to_string())
                .body(xml))
        }

        #[cfg(feature = "support-json")]
        DataType::Json => {
            let json = medication_dispense.json()?;

            Ok(HttpResponse::Ok()
                .content_type(DataType::Json.as_mime().to_string())
                .body(json))
        }

        DataType::Any | DataType::Unknown => panic!("Data type of response was not specified"),
    }
}

fn make_uri(query: &str, page_id: usize) -> String {
    if query.is_empty() {
        format!("/MedicationDispense?pageId={}", page_id)
    } else {
        format!("/MedicationDispense?{}&pageId={}", query, page_id)
    }
}
