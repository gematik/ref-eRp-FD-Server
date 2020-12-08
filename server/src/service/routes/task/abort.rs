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

use actix_web::{
    web::{Data, Path, Query},
    HttpResponse,
};
use chrono::Utc;
use resources::{primitives::Id, task::Status};
use serde::Deserialize;

use crate::service::{
    header::{Accept, Authorization, XAccessCode},
    misc::{create_response, DataType, Profession},
    state::State,
    AsReqErr, AsReqErrResult, TypedRequestError, TypedRequestResult,
};

use super::Error;

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
        .as_req_err()
        .err_with_type(accept)?;

    let id = id.0;
    let kvnr = access_token.kvnr().ok();
    let mut state = state.lock().await;
    let task = match state.get_task_mut(&id, &kvnr, &access_code) {
        Some(Ok(task)) => task,
        Some(Err(())) => return Err(Error::Forbidden(id).as_req_err().with_type(accept)),
        None => return Err(Error::NotFound(id).as_req_err().with_type(accept)),
    };

    let is_pharmacy = access_token.is_pharmacy();
    let is_in_progress = task.status == Status::InProgress;

    if is_pharmacy != is_in_progress {
        return Err(Error::Forbidden(id).as_req_err().with_type(accept));
    }

    if is_pharmacy && (query.secret.is_none() || task.identifier.secret != query.secret) {
        return Err(Error::Forbidden(id).as_req_err().with_type(accept));
    }

    task.for_ = None;
    task.status = Status::Cancelled;
    task.identifier.secret = None;
    task.identifier.access_code = None;
    task.last_modified = Some(Utc::now().into());

    let e_prescription = task.input.e_prescription.take();
    let patient_receipt = task.input.patient_receipt.take();
    let _receipt = task.output.receipt.take();

    let res = match create_response(&*task, accept) {
        Ok(res) => res,
        Err(err) => return Err(err),
    };

    if let Some(e_prescription) = e_prescription {
        state.e_prescriptions.remove(&e_prescription);
    }

    if let Some(patient_receipt) = patient_receipt {
        state.patient_receipts.remove(&patient_receipt);
    }

    Ok(res)
}
