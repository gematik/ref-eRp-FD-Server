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

use actix_web::FromRequest;
use actix_web::{
    web::{Data, Path, Payload},
    HttpRequest, HttpResponse,
};
use resources::{primitives::Id, task::Status};

#[cfg(feature = "support-json")]
use crate::fhir::json::definitions::TaskActivateParametersRoot as JsonParameters;
#[cfg(feature = "support-xml")]
use crate::fhir::xml::definitions::TaskActivateParametersRoot as XmlParameters;

#[cfg(feature = "support-json")]
use super::super::misc::json::Data as Json;
#[cfg(feature = "support-xml")]
use super::super::misc::xml::Data as Xml;
use super::{
    super::{
        super::{
            error::Error,
            header::{Accept, ContentType, XAccessCode},
            state::State,
        },
        misc::DataType,
    },
    misc::response_with_task,
};

pub async fn activate(
    state: Data<State>,
    request: HttpRequest,
    id: Path<Id>,
    accept: Accept,
    content_type: ContentType,
    access_code: XAccessCode,
    payload: Payload,
) -> Result<HttpResponse, Error> {
    let access_code = access_code.0;
    let content_type = content_type.0;
    let data_type = DataType::from_mime(&content_type);

    let accept = match DataType::from_accept(accept).unwrap_or_default() {
        DataType::Any => data_type,
        accept => accept,
    };

    match accept {
        DataType::Any | DataType::Unknown => return Err(Error::AcceptUnsupported),
        _ => (),
    }

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

    let kvnr = match args
        .kbv_bundle
        .entry
        .patient
        .as_ref()
        .and_then(|(_url, patient)| patient.identifier.as_ref())
    {
        Some(identifier) => identifier.clone().into(),
        None => return Err(Error::MissingKvnr),
    };

    /* verify the request */

    let mut state = state.lock().await;

    {
        let task = match state.tasks.get(&id) {
            Some(task) => task,
            None => return Ok(HttpResponse::NotFound().finish()),
        };

        if task.status != Status::Draft {
            return Err(Error::InvalidTaskStatus);
        }

        match &task.identifier.access_code {
            Some(s) if *s == access_code => (),
            Some(_) | None => return Ok(HttpResponse::Forbidden().finish()),
        }
    }

    /* create / update resources */

    let mut patient_receipt = args.kbv_bundle.clone();
    patient_receipt.id = Id::generate().map_err(|_| Error::Internal)?;

    let patient_receipt = match state.patient_receipts.entry(patient_receipt.id.clone()) {
        Entry::Occupied(_) => return Err(Error::Internal),
        Entry::Vacant(entry) => entry.insert(patient_receipt).id.clone(),
    };

    let e_prescription = match state.e_prescriptions.entry(args.kbv_bundle.id.clone()) {
        Entry::Occupied(_) => return Ok(HttpResponse::BadRequest().finish()),
        Entry::Vacant(entry) => entry.insert(args.kbv_bundle).id.clone(),
    };

    let mut task = match state.tasks.get_mut(&id) {
        Some(task) => task,
        None => return Err(Error::Internal),
    };

    task.for_ = Some(kvnr);
    task.status = Status::Ready;
    task.input.e_prescription = Some(e_prescription.into());
    task.input.patient_receipt = Some(patient_receipt.into());

    response_with_task(task, accept)
}
