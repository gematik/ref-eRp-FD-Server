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
use std::ops::Deref;

use actix_web::{
    http::StatusCode,
    web::{Data, Payload},
    HttpResponse,
};
use chrono::Utc;
use resources::{primitives::Id, task::Status, Communication};

use crate::service::{
    header::{Accept, Authorization, ContentType},
    misc::{access_token::Profession, create_response_with, read_payload, DataType},
    AsReqErr, AsReqErrResult, State, TypedRequestError, TypedRequestResult,
};

use super::Error;

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
        .check_profession(|p| match p {
            Profession::Versicherter => true,
            Profession::KrankenhausApotheke => true,
            Profession::OeffentlicheApotheke => true,
            _ => false,
        })
        .as_req_err()
        .err_with_type(accept)?;

    let mut communication = read_payload::<Communication>(data_type, payload)
        .await
        .err_with_type(accept)?;

    if communication.content().as_bytes().len() > MAX_CONTENT_SIZE {
        return Err(Error::ContentSizeExceeded.as_req_err().with_type(accept));
    }

    if let Communication::DispenseReq(c) = &communication {
        if c.based_on.is_none() {
            return Err(Error::MissingFieldBasedOn.as_req_err().with_type(accept));
        }
    }

    communication.set_id(Some(Id::generate().unwrap()));
    communication.set_sent(Utc::now().into());
    match &mut communication {
        Communication::InfoReq(c) => {
            c.sender = Some(access_token.kvnr().as_req_err().err_with_type(accept)?)
        }
        Communication::Reply(c) => {
            c.sender = Some(
                access_token
                    .telematik_id()
                    .as_req_err()
                    .err_with_type(accept)?,
            )
        }
        Communication::DispenseReq(c) => {
            c.sender = Some(access_token.kvnr().as_req_err().err_with_type(accept)?)
        }
        Communication::Representative(c) => {
            c.sender = Some(access_token.kvnr().as_req_err().err_with_type(accept)?)
        }
    }

    let sender_eq_recipient = match &mut communication {
        Communication::InfoReq(c) => c.recipient.deref() == c.sender.as_deref().unwrap(),
        Communication::Reply(c) => c.recipient.deref() == c.sender.as_deref().unwrap(),
        Communication::DispenseReq(c) => c.recipient.deref() == c.sender.as_deref().unwrap(),
        Communication::Representative(c) => c.recipient.deref() == c.sender.as_deref().unwrap(),
    };

    if sender_eq_recipient {
        return Err(Error::SenderNotEqualRecipient
            .as_req_err()
            .with_type(accept));
    }

    let mut state = state.lock().await;

    if let Some(based_on) = communication.based_on() {
        let (task_id, access_code) = State::parse_task_url(&based_on).err_with_type(accept)?;

        let kvnr = access_token.kvnr().ok();

        let task = state
            .get_task(&task_id, &kvnr, &access_code)
            .ok_or_else(|| Error::UnknownTask(task_id).as_req_err().with_type(accept))?
            .map_err(|()| Error::UnauthorizedTaskAccess.as_req_err().with_type(accept))?;

        match (&communication, task.status) {
            (Communication::Representative(_), Status::Ready) => (),
            (Communication::Representative(_), Status::InProgress) => (),
            _ => return Err(Error::InvalidTaskStatus.as_req_err().with_type(accept)),
        }
    }

    let id = communication.id().clone().unwrap();
    let communication = match state.communications.entry(id) {
        Entry::Occupied(entry) => entry.into_mut(),
        Entry::Vacant(entry) => entry.insert(communication),
    };

    create_response_with(&*communication, accept, StatusCode::CREATED)
}

const MAX_CONTENT_SIZE: usize = 10 * 1024;
