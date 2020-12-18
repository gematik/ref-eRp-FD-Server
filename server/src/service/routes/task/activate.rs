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
use chrono::{Duration, Utc};
use futures::{future::ready, stream::once};
use resources::{
    primitives::DateTime,
    primitives::Id,
    task::{Status, TaskActivateParameters},
    types::FlowType,
    KbvBinary, KbvBundle, SignatureType,
};

use crate::{
    fhir::{decode::XmlDecode, security::Signed},
    service::{
        header::{Accept, Authorization, ContentType, XAccessCode},
        misc::{create_response, read_payload, Cms, DataType, Profession, SigCert, SigKey},
        state::State,
        AsReqErr, AsReqErrResult, TypedRequestError, TypedRequestResult,
    },
};

use super::Error;
use std::ops::Add;

#[allow(clippy::too_many_arguments)]
pub async fn activate(
    state: Data<State>,
    cms: Data<Cms>,
    sig_key: Data<SigKey>,
    sig_cert: Data<SigCert>,
    id: Path<Id>,
    accept: Accept,
    access_token: Authorization,
    content_type: ContentType,
    access_code: XAccessCode,
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

    let id = id.0;
    let args = read_payload::<TaskActivateParameters>(data_type, payload)
        .await
        .err_with_type(accept)?;
    let kbv_binary = KbvBinary(args.data);
    let kbv_bundle = cms.verify(&kbv_binary.0).err_with_type(accept)?;
    let kbv_bundle = kbv_bundle.into();
    let kbv_bundle = Result::<Bytes, PayloadError>::Ok(kbv_bundle);
    let kbv_bundle: KbvBundle = once(ready(kbv_bundle))
        .xml()
        .await
        .as_req_err()
        .err_with_type(accept)?;

    let kvnr = match kbv_bundle
        .entry
        .patient
        .as_ref()
        .and_then(|(_url, patient)| patient.identifier.as_ref())
        .map(Clone::clone)
        .map(TryInto::try_into)
    {
        Some(Ok(kvnr)) => kvnr,
        Some(Err(())) => return Err(Error::KvnrInvalid.as_req_err().with_type(accept)),
        None => return Err(Error::KvnrMissing.as_req_err().with_type(accept)),
    };

    /* verify the request */

    let mut state = state.lock().await;

    {
        let task = match state.tasks.get(&id) {
            Some(task) => task,
            None => return Err(Error::NotFound(id).as_req_err().with_type(accept)),
        };

        if Status::Draft != task.status {
            return Err(Error::InvalidStatus.as_req_err().with_type(accept));
        }

        match &task.identifier.access_code {
            Some(s) if *s == access_code => (),
            Some(_) | None => return Err(Error::Forbidden(id).as_req_err().with_type(accept)),
        }
    }

    /* create / update resources */

    let mut patient_receipt = kbv_bundle.clone();
    patient_receipt.id = Id::generate().unwrap();

    let patient_receipt = match state.patient_receipts.entry(patient_receipt.id.clone()) {
        Entry::Occupied(_) => {
            panic!(
                "Patient receipt with this ID ({}) already exists!",
                patient_receipt.id
            );
        }
        Entry::Vacant(entry) => {
            let mut patient_receipt = Signed::new(patient_receipt);
            patient_receipt
                .sign_json(
                    SignatureType::AuthorsSignature,
                    "Device/software".into(),
                    &sig_key.0,
                    &sig_cert.0,
                )
                .as_req_err()
                .err_with_type(accept)?;

            entry.insert(patient_receipt).id.clone()
        }
    };

    let e_prescription = match state.e_prescriptions.entry(kbv_bundle.id.clone()) {
        Entry::Occupied(_) => {
            panic!(
                "ePrescription with this ID ({}) does already exist!",
                kbv_bundle.id
            );
        }
        Entry::Vacant(entry) => entry.insert((kbv_binary, kbv_bundle)).1.id.clone(),
    };

    let task = match state.tasks.get_mut(&id) {
        Some(task) => task,
        None => return Err(Error::NotFound(id).as_req_err().with_type(accept)),
    };

    task.for_ = Some(kvnr);
    task.status = Status::Ready;
    task.input.e_prescription = Some(e_prescription);
    task.input.patient_receipt = Some(patient_receipt);

    let acpt_exp_dur = match task.extension.flow_type {
        FlowType::PharmaceuticalDrugs => (Duration::days(30), Duration::days(92)),
    };
    let now = Utc::now(); //TODO: take date from QES signature in CMS/pkcs7
    task.extension.accept_date = Some(DateTime::from(now.add(acpt_exp_dur.0)));
    task.extension.expiry_date = Some(DateTime::from(now.add(acpt_exp_dur.1)));

    create_response(&**task, accept)
}
