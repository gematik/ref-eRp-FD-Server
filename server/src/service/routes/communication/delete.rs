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
    web::{Data, Path},
    HttpResponse,
};
use resources::primitives::Id;

use crate::{
    service::{
        header::{Accept, Authorization},
        misc::{DataType, Profession},
        AsReqErrResult, TypedRequestError, TypedRequestResult,
    },
    state::State,
};

pub async fn delete_one(
    state: Data<State>,
    id: Path<Id>,
    accept: Accept,
    access_token: Authorization,
) -> Result<HttpResponse, TypedRequestError> {
    let accept = DataType::from_accept(&accept)
        .unwrap_or_default()
        .replace_any_default()
        .check_supported()
        .err_with_type_default()?;

    access_token
        .check_profession(|p| match p {
            Profession::Versicherter => true,
            Profession::KrankenhausApotheke => true,
            Profession::OeffentlicheApotheke => true,
            _ => false,
        })
        .as_req_err()
        .err_with_type(accept)?;

    let id = id.into_inner();
    let participant_id = access_token.id().as_req_err().err_with_type(accept)?;

    let mut state = state.lock().await;
    state
        .communication_delete(id, &participant_id)
        .as_req_err()
        .err_with_type(accept)?;

    Ok(HttpResponse::NoContent().finish())
}
