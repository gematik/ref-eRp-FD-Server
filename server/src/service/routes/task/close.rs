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

use std::collections::hash_map::Entry;

use actix_web::{
    web::{Data, Path, Payload, Query},
    HttpResponse,
};
use chrono::Utc;
use resources::{
    erx_bundle::{Entry as ErxEntry, ErxBundle},
    primitives::Id,
    task::Status,
    ErxComposition, MedicationDispense,
};
use serde::Deserialize;

use crate::service::{
    header::{Accept, Authorization, ContentType},
    misc::{create_response, read_payload, DataType, Profession, DEVICE},
    state::State,
    AsReqErr, AsReqErrResult, TypedRequestError, TypedRequestResult,
};

use super::Error;

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

    let id = id.0;
    let mut medication_dispense = read_payload::<MedicationDispense>(data_type, payload)
        .await
        .err_with_type(accept)?;
    let mut state = state.lock().await;
    let task_meta = match state.tasks.get_mut(&id) {
        Some(task_meta) => task_meta,
        None => return Err(Error::NotFound(id).as_req_err().with_type(accept)),
    };

    let task = task_meta.history.get();
    let prescription_id = task
        .identifier
        .prescription_id
        .as_ref()
        .ok_or(Error::EPrescriptionMissing)
        .as_req_err()
        .err_with_type(accept)?;
    if &medication_dispense.prescription_id != prescription_id {
        return Err(Error::EPrescriptionMismatch.as_req_err().with_type(accept));
    }

    let subject = task
        .for_
        .as_ref()
        .ok_or(Error::SubjectMissing)
        .as_req_err()
        .err_with_type(accept)?;
    if &medication_dispense.subject != subject {
        return Err(Error::SubjectMismatch.as_req_err().with_type(accept));
    }

    let performer = access_token
        .telematik_id()
        .as_req_err()
        .err_with_type(accept)?;
    if medication_dispense.performer != performer {
        return Err(Error::PerformerMismatch.as_req_err().with_type(accept));
    }

    if task.status != Status::InProgress || task.identifier.secret != query.secret {
        return Err(Error::Forbidden(id).as_req_err().with_type(accept));
    }

    let now = Utc::now();
    let erx_bundle = ErxBundle {
        id: Id::generate().unwrap(),
        identifier: prescription_id.clone(),
        timestamp: Utc::now().into(),
        entry: ErxEntry {
            composition: Some(ErxComposition {
                beneficiary: performer,
                date: now.clone().into(),
                author: DEVICE.id.clone().into(),
                event_start: task_meta
                    .accept_timestamp
                    .ok_or(Error::AcceptTimestampMissing)
                    .as_req_err()
                    .err_with_type(accept)?
                    .into(),
                event_end: now.into(),
            }),
            device: Some(DEVICE.clone()),
        },
        signature: vec![],
    };

    medication_dispense.id = Some(Id::generate().unwrap());
    medication_dispense.supporting_information = Some(format!("/Task/{}", id));

    let task = task_meta.history.get_mut();
    task.status = Status::Completed;
    task.output.receipt = Some(erx_bundle.id.clone());

    state.remove_communications(&id);

    match state
        .medication_dispense
        .entry(medication_dispense.id.as_ref().unwrap().clone())
    {
        Entry::Occupied(_) => {
            panic!(
                "Medication dispense with this ID ({}) already exists!",
                medication_dispense.id.unwrap()
            );
        }
        Entry::Vacant(entry) => {
            entry.insert(medication_dispense);
        }
    };

    let erx_bundle = match state.erx_receipts.entry(erx_bundle.id.clone()) {
        Entry::Occupied(_) => {
            panic!("ErxBundle with this ID ({}) already exists!", erx_bundle.id);
        }
        Entry::Vacant(entry) => entry.insert(erx_bundle),
    };

    create_response(&*erx_bundle, accept)
}
