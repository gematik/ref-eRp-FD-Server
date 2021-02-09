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
    web::{Data, Path, Payload, Query},
    HttpResponse,
};
use resources::{primitives::Id, MedicationDispense};
use serde::Deserialize;

use crate::{
    service::{
        header::{Accept, Authorization, ContentType},
        misc::{create_response, read_payload, DataType, Profession},
        AsReqErrResult, TypedRequestError, TypedRequestResult,
    },
    state::State,
};

#[derive(Deserialize)]
pub struct QueryArgs {
    secret: Option<String>,
}

pub async fn close(
    state: Data<State>,
    id: Path<Id>,
    accept: Accept,
    content_type: ContentType,
    access_token: Authorization,
    query: Query<QueryArgs>,
    payload: Payload,
) -> Result<HttpResponse, TypedRequestError> {
    let data_type = DataType::from_mime(&content_type);
    let accept = DataType::from_accept(&accept)
        .unwrap_or_default()
        .replace_any(data_type)
        .check_supported()
        .err_with_type_default()?;

    access_token
        .check_profession(|p| {
            p == Profession::OeffentlicheApotheke || p == Profession::KrankenhausApotheke
        })
        .as_req_err()
        .err_with_type(accept)?;

    let id = id.into_inner();
    let secret = query.into_inner().secret;
    let performer = access_token
        .telematik_id()
        .as_req_err()
        .err_with_type(accept)?;
    let medication_dispense = read_payload::<MedicationDispense>(data_type, payload)
        .await
        .err_with_type(accept)?;
    let agent = (&*access_token).into();

    let mut state = state.lock().await;
    let erx_bundle = state
        .task_close(id, secret, performer, medication_dispense, agent)
        .as_req_err()
        .err_with_type(accept)?;

    create_response(erx_bundle, accept)
}
