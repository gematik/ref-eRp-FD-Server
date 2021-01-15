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
use chrono::Utc;
use resources::{
    bundle::{Bundle, Entry, Type},
    primitives::Id,
    task::Status,
};

use crate::service::{
    header::{Accept, Authorization, XAccessCode},
    misc::{create_response, DataType, Profession},
    state::State,
    AsReqErr, AsReqErrResult, TypedRequestError, TypedRequestResult,
};

use super::{
    misc::{random_id, Resource},
    Error,
};

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
        .as_req_err()
        .err_with_type(accept)?;

    let id = id.0;
    let mut state = state.lock().await;
    let mut task = match state.tasks.get_mut(&id) {
        Some(task) => task,
        None => return Err(Error::NotFound(id).as_req_err().with_type(accept)),
    };

    match &task.identifier.access_code {
        Some(ac) if ac == &access_code.0 => (),
        Some(_) => return Err(Error::Forbidden(id).as_req_err().with_type(accept)),
        None => return Err(Error::Gone(id).as_req_err().with_type(accept)),
    }

    match task.status {
        Status::Completed | Status::InProgress | Status::Draft => {
            return Err(Error::Conflict(id).as_req_err().with_type(accept))
        }
        Status::Cancelled => return Err(Error::Gone(id).as_req_err().with_type(accept)),
        _ => (),
    }

    task.accept_timestamp = Some(Utc::now());
    task.status = Status::InProgress;
    task.identifier.secret = Some(random_id());

    let e_prescription = task
        .input
        .e_prescription
        .as_ref()
        .ok_or(Error::EPrescriptionMissing)
        .as_req_err()
        .err_with_type(accept)?
        .clone();
    let e_prescription = state
        .e_prescriptions
        .get(&e_prescription)
        .ok_or(Error::EPrescriptionNotFound(e_prescription))
        .as_req_err()
        .err_with_type(accept)?;

    let task = state.tasks.get(&id).unwrap();
    let mut bundle = Bundle::new(Type::Collection);
    bundle.entries.push(Entry::new(Resource::Task(task)));
    bundle
        .entries
        .push(Entry::new(Resource::Binary(&e_prescription.0)));

    create_response(&bundle, accept)
}
