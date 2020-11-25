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
use std::ops::Deref;
use std::str::FromStr;

use actix_web::{
    web::{Data, Payload},
    HttpResponse,
};
use chrono::Utc;
use resources::{primitives::Id, task::Status, Communication};
use url::Url;

#[cfg(feature = "support-json")]
use crate::fhir::decode::JsonDecode;
#[cfg(feature = "support-xml")]
use crate::fhir::decode::XmlDecode;
use crate::service::{
    error::RequestError,
    header::{Accept, Authorization, ContentType, XAccessCode},
    misc::{access_token::Profession, DataType},
    State,
};

use super::misc::response_with_communication;

pub async fn create(
    state: Data<State>,
    accept: Accept,
    access_token: Authorization,
    content_type: ContentType,
    mut payload: Payload,
) -> Result<HttpResponse, RequestError> {
    access_token.check_profession(|p| match p {
        Profession::Versicherter => true,
        Profession::KrankenhausApotheke => true,
        Profession::OeffentlicheApotheke => true,
        _ => false,
    })?;

    let data_type = DataType::from_mime(&content_type);
    let mut communication: Communication = match data_type {
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

    let accept = DataType::from_accept(accept)
        .unwrap_or_default()
        .replace_any(data_type)
        .check_supported()?;

    if communication.content().as_bytes().len() > MAX_CONTENT_SIZE {
        return Err(RequestError::BadRequest("Invalid payload content!".into()));
    }

    if let Communication::DispenseReq(c) = &communication {
        if c.based_on.is_none() {
            return Err(RequestError::BadRequest(
                "Communication is missing the `basedOn` field!".into(),
            ));
        }
    }

    communication.set_id(Some(
        Id::generate().map_err(|()| RequestError::Internal("Unable to generate Id".into()))?,
    ));
    communication.set_sent(Utc::now().into());
    match &mut communication {
        Communication::InfoReq(c) => c.sender = Some(access_token.kvnr()?),
        Communication::Reply(c) => c.sender = Some(access_token.telematik_id()?),
        Communication::DispenseReq(c) => c.sender = Some(access_token.kvnr()?),
        Communication::Representative(c) => c.sender = Some(access_token.kvnr()?),
    }

    let sender_eq_recipient = match &mut communication {
        Communication::InfoReq(c) => c.recipient.deref() == c.sender.as_deref().unwrap(),
        Communication::Reply(c) => c.recipient.deref() == c.sender.as_deref().unwrap(),
        Communication::DispenseReq(c) => c.recipient.deref() == c.sender.as_deref().unwrap(),
        Communication::Representative(c) => c.recipient.deref() == c.sender.as_deref().unwrap(),
    };

    if sender_eq_recipient {
        return Err(RequestError::BadRequest(
            "Sender is equal to recipient!".into(),
        ));
    }

    let mut state = state.lock().await;

    if let Some(based_on) = communication.based_on() {
        let (task_id, access_code) = parse_task_url(&based_on).map_err(|()| {
            RequestError::BadRequest("Communication contains invalid task URI: {}!".into())
        })?;

        let kvnr = access_token.kvnr().ok();

        let task = state
            .get_task(&task_id, &kvnr, &access_code)
            .ok_or_else(|| RequestError::BadRequest("Unknown task!".into()))?
            .map_err(|()| {
                RequestError::Unauthorized("You are not allowed to access this task!".into())
            })?;

        match (&communication, task.status) {
            (Communication::Representative(_), Status::Ready) => (),
            (Communication::Representative(_), Status::InProgress) => (),
            _ => return Err(RequestError::BadRequest("Task has invalid status!".into())),
        }
    }

    let id = communication.id().clone().unwrap();
    let communication = match state.communications.entry(id) {
        Entry::Occupied(entry) => entry.into_mut(),
        Entry::Vacant(entry) => entry.insert(communication),
    };

    response_with_communication(communication, accept, true)
}

fn parse_task_url(url: &str) -> Result<(Id, Option<XAccessCode>), ()> {
    let url = format!("http://localhost/{}", url);
    let url = Url::from_str(&url).map_err(|_| ())?;

    let mut path = url.path_segments().ok_or(())?;
    if path.next() != Some("Task") {
        return Err(());
    }

    let task_id = path.next().ok_or(())?;
    let task_id = task_id.try_into().map_err(|_| ())?;

    let access_code = url.query_pairs().find_map(|(key, value)| {
        if key == "ac" {
            Some(XAccessCode(value.into_owned()))
        } else {
            None
        }
    });

    Ok((task_id, access_code))
}

const MAX_CONTENT_SIZE: usize = 10 * 1024;
