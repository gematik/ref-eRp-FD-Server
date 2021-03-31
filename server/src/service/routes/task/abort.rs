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
use resources::primitives::Id;
use serde::Deserialize;

use crate::{
    service::{
        header::{Accept, Authorization, XAccessCode},
        misc::{DataType, Profession},
        IntoReqErrResult, TypedRequestError, TypedRequestResult,
    },
    state::State,
};

#[derive(Deserialize)]
pub struct QueryArgs {
    secret: Option<String>,
}

pub async fn abort(
    state: Data<State>,
    id: Path<Id>,
    query: Query<QueryArgs>,
    accept: Accept,
    access_token: Authorization,
    access_code: Option<XAccessCode>,
) -> Result<HttpResponse, TypedRequestError> {
    let accept = DataType::from_accept(&accept)
        .and_then(DataType::ignore_any)
        .unwrap_or_default()
        .check_supported()
        .err_with_type_default()?;

    access_token
        .check_profession(|p| {
            p == Profession::Versicherter
                || p == Profession::Arzt
                || p == Profession::Zahnarzt
                || p == Profession::PraxisArzt
                || p == Profession::ZahnarztPraxis
                || p == Profession::PraxisPsychotherapeut
                || p == Profession::Krankenhaus
                || p == Profession::OeffentlicheApotheke
                || p == Profession::KrankenhausApotheke
        })
        .into_req_err()
        .err_with_type(accept)?;

    let id = id.into_inner();
    let kvnr = access_token.kvnr().ok();
    let is_pharmacy = access_token.is_pharmacy();
    let secret = query.into_inner().secret;
    let agent = (&*access_token).into();

    state
        .lock()
        .await
        .task_abort(id, kvnr, access_code, is_pharmacy, secret, agent)
        .into_req_err()
        .err_with_type(accept)?;

    Ok(HttpResponse::NoContent().finish())
}
