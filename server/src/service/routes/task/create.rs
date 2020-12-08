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

use std::collections::hash_map::Entry;
use std::convert::TryInto;
use std::sync::atomic::{AtomicUsize, Ordering};

use actix_web::{
    http::StatusCode,
    web::{Data, Payload},
    HttpResponse,
};
use chrono::Utc;
use resources::{
    misc::PrescriptionId,
    primitives::Id,
    task::{Extension, Identifier, Status, Task, TaskCreateParameters},
    types::{FlowType, PerformerType},
};

use crate::service::{
    header::{Accept, Authorization, ContentType},
    misc::{create_response_with, read_payload, DataType, Profession},
    AsReqErrResult, RequestError, State, TypedRequestError, TypedRequestResult,
};

use super::misc::random_id;

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
    let task = create_task(args.flow_type).err_with_type(accept)?;
    let id = task.id.clone().unwrap();

    let mut state = state.lock().await;
    let task = match state.tasks.entry(id) {
        Entry::Occupied(entry) => entry.into_mut(),
        Entry::Vacant(entry) => entry.insert(task.into()),
    };

    create_response_with(&**task, accept, StatusCode::CREATED)
}

fn create_task(flow_type: FlowType) -> Result<Task, RequestError> {
    let id = Some(Id::generate().unwrap());
    let access_code = random_id();
    let prescription_id = generate_prescription_id(FlowType::PharmaceuticalDrugs);

    Ok(Task {
        id,
        extension: Extension {
            accept_date: None,
            expiry_date: None,
            flow_type,
        },
        identifier: Identifier {
            access_code: Some(access_code),
            prescription_id: Some(prescription_id),
            ..Default::default()
        },
        status: Status::Draft,
        for_: None,
        authored_on: Some(Utc::now().to_rfc3339().try_into().unwrap()),
        last_modified: Some(Utc::now().to_rfc3339().try_into().unwrap()),
        performer_type: vec![PerformerType::PublicPharmacy],
        input: Default::default(),
        output: Default::default(),
    })
}

fn generate_prescription_id(flow_type: FlowType) -> PrescriptionId {
    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

    let number = NEXT_ID.fetch_add(1, Ordering::SeqCst);

    PrescriptionId::new(flow_type, number)
}
