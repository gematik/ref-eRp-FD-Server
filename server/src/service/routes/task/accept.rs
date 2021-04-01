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
use resources::{
    bundle::{Bundle, Entry, Type},
    primitives::Id,
};

use crate::{
    service::{
        header::{Accept, Authorization, XAccessCode},
        misc::{create_response, DataType, Profession},
        IntoReqErrResult, TypedRequestError, TypedRequestResult,
    },
    state::State,
};

use super::misc::Resource;

pub async fn accept(
    state: Data<State>,
    id: Path<Id>,
    accept: Accept,
    access_token: Authorization,
    access_code: XAccessCode,
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
        .into_req_err()
        .err_with_type(accept)?;

    let id = id.into_inner();
    let agent = (&*access_token).into();
    let mut state = state.lock().await;
    let (task, e_prescription) = state
        .task_accept(id, access_code, agent)
        .into_req_err()
        .err_with_type(accept)?;

    let mut bundle = Bundle::new(Type::Collection);
    bundle
        .entries
        .push(Entry::new(Resource::TaskForSupplier(task)));
    bundle
        .entries
        .push(Entry::new(Resource::KbvBinary(e_prescription)));

    create_response(&bundle, accept)
}
