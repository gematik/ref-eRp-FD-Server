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
    error::PayloadError,
    web::{Data, Path, Payload},
    HttpResponse,
};
use bytes::Bytes;
use futures::{future::ready, stream::once};
use resources::{primitives::Id, task::TaskActivateParameters, KbvBinary, KbvBundle};

use crate::{
    fhir::{decode::XmlDecode, definitions::TaskContainer},
    pki_store::PkiStore,
    service::{
        header::{Accept, Authorization, ContentType, XAccessCode},
        misc::{create_response, read_payload, DataType, Profession},
        IntoReqErrResult, RequestError, TypedRequestError, TypedRequestResult,
    },
    state::State,
};

#[allow(clippy::too_many_arguments)]
pub async fn activate(
    state: Data<State>,
    pki_store: Data<PkiStore>,
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
            p == Profession::PraxisArzt
                || p == Profession::ZahnarztPraxis
                || p == Profession::PraxisPsychotherapeut
                || p == Profession::Krankenhaus
        })
        .into_req_err()
        .err_with_type(accept)?;

    let id = id.into_inner();
    let args = read_payload::<TaskActivateParameters>(data_type, payload)
        .await
        .err_with_type(accept)?;
    let kbv_binary = KbvBinary {
        id: Id::generate().unwrap(),
        data: args.data,
    };
    let (kbv_bundle, signing_time) = match pki_store.verify_cms(&kbv_binary.data, true) {
        Ok((kbv_bundle, signing_time)) => (kbv_bundle, signing_time),
        Err(err) => {
            let warning = state.throttle().await;

            return Err(RequestError::CmsContainerError { err, warning }.with_type(accept));
        }
    };

    let kbv_bundle = kbv_bundle.into();
    let kbv_bundle = Result::<Bytes, PayloadError>::Ok(kbv_bundle);
    let kbv_bundle: KbvBundle = once(ready(kbv_bundle))
        .xml()
        .await
        .into_req_err()
        .err_with_type(accept)?;

    let agent = (&*access_token).into();
    let mut state = state.lock().await;
    let task = state
        .task_activate(id, access_code, signing_time, kbv_binary, kbv_bundle, agent)
        .into_req_err()
        .err_with_type(accept)?;

    create_response(TaskContainer::for_supplier(task), accept)
}
