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
    web::{Data, Path},
    HttpResponse,
};
use resources::primitives::Id;

use crate::service::{
    header::Authorization,
    misc::Profession,
    state::{CommunicationMatch, State},
    RequestError,
};

pub async fn delete_one(
    state: Data<State>,
    id: Path<Id>,
    access_token: Authorization,
) -> Result<HttpResponse, RequestError> {
    access_token.check_profession(|p| match p {
        Profession::Versicherter => true,
        Profession::KrankenhausApotheke => true,
        Profession::OeffentlicheApotheke => true,
        _ => false,
    })?;

    let id = id.into_inner();
    let kvnr = access_token.kvnr().ok();
    let telematik_id = access_token.telematik_id().ok();

    let mut state = state.lock().await;
    match state.get_communication(&id, &kvnr, &telematik_id) {
        CommunicationMatch::NotFound => return Ok(HttpResponse::NotFound().finish()),
        CommunicationMatch::Unauthorized | CommunicationMatch::Recipient(_) => {
            return Ok(HttpResponse::Unauthorized().finish())
        }
        CommunicationMatch::Sender(_) => (),
    }

    state.communications.remove(&id);

    Ok(HttpResponse::NoContent().finish())
}
