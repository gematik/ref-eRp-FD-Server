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

use actix_web::{
    web::{Data, Payload},
    HttpResponse,
};
use chrono::Utc;
use log::debug;
use rand::{distributions::Standard, thread_rng, Rng};
use resources::{
    misc::PrescriptionId,
    primitives::Id,
    task::{Extension, Identifier, Status, Task, TaskCreateParameters},
    types::{FlowType, PerformerType},
};

#[cfg(feature = "support-json")]
use crate::fhir::decode::JsonDecode;
#[cfg(feature = "support-xml")]
use crate::fhir::decode::XmlDecode;
use crate::service::{
    error::RequestError,
    header::{Accept, Authorization, ContentType},
    misc::{DataType, Profession},
    State,
};

use super::misc::response_with_task;

pub async fn create(
    state: Data<State>,
    accept: Accept,
    access_token: Authorization,
    content_type: ContentType,
    mut payload: Payload,
) -> Result<HttpResponse, RequestError> {
    access_token.check_profession(|p| {
        p == Profession::Arzt
            || p == Profession::Zahnarzt
            || p == Profession::PraxisArzt
            || p == Profession::ZahnarztPraxis
            || p == Profession::PraxisPsychotherapeut
            || p == Profession::Krankenhaus
    })?;

    let data_type = DataType::from_mime(&content_type);
    let args: TaskCreateParameters = match data_type {
        #[cfg(feature = "support-xml")]
        DataType::Xml => payload.xml().await?,

        #[cfg(feature = "support-json")]
        DataType::Json => payload.json().await?,

        DataType::Unknown | DataType::Any => {
            return Err(RequestError::ContentTypeNotSupported(
                content_type.to_string(),
            ))
        }
    };

    let task = create_task(args.flow_type)?;
    let id = task.id.clone().unwrap();

    let mut state = state.lock().await;
    let task = match state.tasks.entry(id) {
        Entry::Occupied(entry) => entry.into_mut(),
        Entry::Vacant(entry) => entry.insert(task),
    };
    debug!(target: "ref_erx_fd_server", "Task created with id: {:?}, ac: {:?}", task.id, task.identifier.access_code);

    let data_type = match DataType::from_accept(accept).unwrap_or_default() {
        DataType::Any => data_type,
        data_type => data_type,
    };

    response_with_task(task, data_type)
}

fn create_task(flow_type: FlowType) -> Result<Task, RequestError> {
    let id =
        Some(Id::generate().map_err(|()| RequestError::Internal("Unable to generate Id".into()))?);
    let access_code: String = thread_rng()
        .sample_iter(&Standard)
        .take(32)
        .map(|x: u8| format!("{:X}", x))
        .collect::<Vec<_>>()
        .join("");
    let prescription_id = PrescriptionId::new(FlowType::PharmaceuticalDrugs, 123);

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
        authored_on: Some(Utc::now().to_rfc3339().try_into().map_err(|err| {
            RequestError::internal(format!("Unable to set Task.authored_on: {}", err))
        })?),
        last_modified: Some(Utc::now().to_rfc3339().try_into().map_err(|err| {
            RequestError::internal(format!("Unable to set Task.last_modified: {}", err))
        })?),
        performer_type: vec![PerformerType::PublicPharmacy],
        input: Default::default(),
        output: Default::default(),
    })
}
