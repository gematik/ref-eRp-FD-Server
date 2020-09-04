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

use actix_web::FromRequest;
use actix_web::{
    web::{Data, Payload},
    HttpRequest, HttpResponse,
};
use chrono::Utc;
use log::debug;
use rand::{
    distributions::{Alphanumeric, Standard},
    thread_rng, Rng,
};

#[cfg(feature = "support-json")]
use crate::fhir::json::definitions::TaskCreateParametersRoot as JsonParameters;
#[cfg(feature = "support-xml")]
use crate::fhir::xml::definitions::TaskCreateParametersRoot as XmlParameters;
use resources::{
    misc::PrescriptionId,
    task::{Extension, Identifier, Status, Task},
    types::{FlowType, PerformerType},
};

#[cfg(feature = "support-json")]
use super::super::misc::json::Data as Json;
#[cfg(feature = "support-xml")]
use super::super::misc::xml::Data as Xml;
use super::{
    super::{
        super::{
            error::Error,
            header::{Accept, ContentType},
            State,
        },
        misc::DataType,
    },
    misc::response_with_task,
};

pub async fn create(
    state: Data<State>,
    request: HttpRequest,
    accept: Accept,
    content_type: ContentType,
    payload: Payload,
) -> Result<HttpResponse, Error> {
    let content_type = content_type.0;
    let data_type = DataType::from_mime(&content_type);

    let args = match data_type {
        #[cfg(feature = "support-xml")]
        DataType::Xml => Xml::<XmlParameters>::from_request(&request, &mut payload.into_inner())
            .await?
            .0
            .into_inner(),

        #[cfg(feature = "support-json")]
        DataType::Json => Json::<JsonParameters>::from_request(&request, &mut payload.into_inner())
            .await?
            .0
            .into_inner(),

        DataType::Unknown | DataType::Any => {
            return Err(Error::ContentTypeNotSupported(content_type))
        }
    };

    let task = create_task(args.flow_type)?;
    let mut state = state.lock().await;

    let id = match &task.id {
        Some(id) => id.clone(),
        None => return Err(Error::Internal),
    };

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

fn create_task(flow_type: FlowType) -> Result<Task, Error> {
    let id: String = thread_rng().sample_iter(&Alphanumeric).take(64).collect();
    let access_code: String = thread_rng()
        .sample_iter(&Standard)
        .take(32)
        .map(|x: u8| format!("{:X}", x))
        .collect::<Vec<_>>()
        .join("");
    let prescription_id = PrescriptionId::new(FlowType::PharmaceuticalDrugs, 123);

    Ok(Task {
        id: Some(id.try_into().unwrap()),
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
        authored_on: Some(
            Utc::now()
                .to_rfc3339()
                .try_into()
                .map_err(|_| Error::Internal)?,
        ),
        last_modified: Some(
            Utc::now()
                .to_rfc3339()
                .try_into()
                .map_err(|_| Error::Internal)?,
        ),
        performer_type: vec![PerformerType::PublicPharmacy],
        input: Default::default(),
        output: Default::default(),
    })
}
