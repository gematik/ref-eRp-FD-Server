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

use actix_web::{
    web::{Data, Path, Query},
    HttpResponse,
};
use resources::{primitives::Id, task::Status};
use serde::Deserialize;

use crate::{
    fhir::definitions::TaskContainer,
    service::{
        header::{Accept, Authorization},
        misc::{create_response, DataType, Profession},
        state::State,
        AsReqErr, AsReqErrResult, TypedRequestError, TypedRequestResult,
    },
};

use super::Error;

#[derive(Deserialize)]
pub struct QueryArgs {
    secret: Option<String>,
}

pub async fn reject(
    state: Data<State>,
    id: Path<Id>,
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
        .check_profession(|p| {
            p == Profession::OeffentlicheApotheke || p == Profession::KrankenhausApotheke
        })
        .as_req_err()
        .err_with_type(accept)?;

    let id = id.0;
    let mut state = state.lock().await;
    let mut task_meta = match state.tasks.get_mut(&id) {
        Some(task_meta) => task_meta,
        None => return Err(Error::NotFound(id).as_req_err().with_type(accept)),
    };

    let task = task_meta.history.get();
    if task.status != Status::InProgress || task.identifier.secret != query.secret {
        return Err(Error::Forbidden(id).as_req_err().with_type(accept));
    }

    task_meta.accept_timestamp = None;

    let mut task = task_meta.history.get_mut();
    task.status = Status::Ready;
    task.identifier.secret = None;

    let v = task_meta.history.get_current();
    create_response(TaskContainer(v), accept)
}
