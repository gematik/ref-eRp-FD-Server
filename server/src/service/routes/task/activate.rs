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
    error::PayloadError,
    web::{Data, Path, Payload},
    HttpResponse,
};
use bytes::Bytes;
use futures::{future::ready, stream::once};
use resources::{
    primitives::Id,
    task::{Status, TaskActivateParameters},
    KbvBundle,
};

#[cfg(feature = "support-json")]
use crate::fhir::decode::JsonDecode;
#[cfg(feature = "support-xml")]
use crate::fhir::decode::XmlDecode;

use crate::service::{
    error::RequestError,
    header::{Accept, Authorization, ContentType, XAccessCode},
    misc::{Cms, DataType, Profession},
    state::State,
};

use super::misc::response_with_task;

#[allow(clippy::too_many_arguments)]
pub async fn activate(
    state: Data<State>,
    cms: Data<Cms>,
    id: Path<Id>,
    accept: Accept,
    access_token: Authorization,
    content_type: ContentType,
    access_code: XAccessCode,
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
    let accept = DataType::from_accept(accept)
        .unwrap_or_default()
        .replace_any(data_type)
        .check_supported()?;

    let args: TaskActivateParameters = match data_type {
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

    let kbv_bundle = cms.verify(&args.data)?;
    let kbv_bundle = kbv_bundle.into();
    let kbv_bundle = Result::<Bytes, PayloadError>::Ok(kbv_bundle);
    let kbv_bundle: KbvBundle = once(ready(kbv_bundle)).xml().await?;

    let kvnr = match kbv_bundle
        .entry
        .patient
        .as_ref()
        .and_then(|(_url, patient)| patient.identifier.as_ref())
        .map(Clone::clone)
        .map(TryInto::try_into)
    {
        Some(Ok(kvnr)) => kvnr,
        Some(Err(())) => {
            return Err(RequestError::BadRequest(
                "KBV Bundle does not contain a valid KV-Nr.!".into(),
            ))
        }
        None => {
            return Err(RequestError::BadRequest(
                "KBV Bundle is missing the KV-Nr.!".into(),
            ))
        }
    };

    /* verify the request */

    let mut state = state.lock().await;

    {
        let task = match state.tasks.get(&id) {
            Some(task) => task,
            None => return Ok(HttpResponse::NotFound().finish()),
        };

        if Status::Draft != task.status {
            return Err(RequestError::BadRequest(format!(
                "Invalid task status (expected={:?}, actual={:?})",
                Status::Draft,
                task.status
            )));
        }

        match &task.identifier.access_code {
            Some(s) if *s == access_code => (),
            Some(_) | None => return Ok(HttpResponse::Forbidden().finish()),
        }
    }

    /* create / update resources */

    let mut patient_receipt = kbv_bundle.clone();
    patient_receipt.id =
        Id::generate().map_err(|_| RequestError::internal("Unable to generate ID"))?;

    let patient_receipt = match state.patient_receipts.entry(patient_receipt.id.clone()) {
        Entry::Occupied(_) => {
            return Err(RequestError::internal(format!(
                "Patient receipt with this ID ({}) already exists!",
                patient_receipt.id
            )))
        }
        Entry::Vacant(entry) => entry.insert(patient_receipt).id.clone(),
    };

    let e_prescription = match state.e_prescriptions.entry(kbv_bundle.id.clone()) {
        Entry::Occupied(_) => return Ok(HttpResponse::BadRequest().finish()),
        Entry::Vacant(entry) => entry.insert(kbv_bundle).id.clone(),
    };

    let mut task = match state.tasks.get_mut(&id) {
        Some(task) => task,
        None => return Err(RequestError::internal("Unable to get task from database!")),
    };

    task.for_ = Some(kvnr);
    task.status = Status::Ready;
    task.input.e_prescription = Some(e_prescription);
    task.input.patient_receipt = Some(patient_receipt);

    response_with_task(task, accept)
}
