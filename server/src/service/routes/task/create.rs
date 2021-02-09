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
    http::StatusCode,
    web::{Data, Payload},
    HttpResponse,
};
use resources::task::TaskCreateParameters;

use crate::{
    fhir::definitions::TaskContainer,
    service::{
        header::{Accept, Authorization, ContentType},
        misc::{create_response_with, read_payload, DataType, Profession},
        AsReqErrResult, State, TypedRequestError, TypedRequestResult,
    },
};

pub async fn create(
    state: Data<State>,
    accept: Accept,
    access_token: Authorization,
    content_type: ContentType,
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
            p == Profession::Arzt
                || p == Profession::Zahnarzt
                || p == Profession::PraxisArzt
                || p == Profession::ZahnarztPraxis
                || p == Profession::PraxisPsychotherapeut
                || p == Profession::Krankenhaus
        })
        .as_req_err()
        .err_with_type(accept)?;

    let args = read_payload::<TaskCreateParameters>(data_type, payload)
        .await
        .err_with_type(accept)?;

    let mut state = state.lock().await;
    let task = state.task_create(args).as_req_err().err_with_type(accept)?;

    create_response_with(TaskContainer(task), accept, StatusCode::CREATED, |_| ())
}
