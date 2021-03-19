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

use std::collections::hash_map::Entry;
use std::convert::TryInto;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use chrono::Utc;
use resources::{
    communication::{Attachment, Content, Inner as CommunicationInner},
    misc::ParticipantId,
    primitives::{DateTime, Id},
    task::Status,
    Communication,
};
use url::Url;

use crate::{service::header::XAccessCode, state::Inner};

use super::Error;

pub enum CommunicationRefMut<'a> {
    Sender(&'a mut Communication),
    Recipient(&'a mut Communication),
}

impl<'a> Deref for CommunicationRefMut<'a> {
    type Target = Communication;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Sender(c) => &*c,
            Self::Recipient(c) => &*c,
        }
    }
}

impl<'a> DerefMut for CommunicationRefMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Sender(c) => c,
            Self::Recipient(c) => c,
        }
    }
}

impl Inner {
    pub fn communication_create(
        &mut self,
        participant_id: ParticipantId,
        mut communication: Communication,
    ) -> Result<&mut Communication, Error> {
        match communication.content() {
            Content::String(s) if s.as_bytes().len() > MAX_CONTENT_SIZE => {
                return Err(Error::ContentSizeExceeded)
            }
            Content::Attachment(Attachment {
                data: Some(data), ..
            }) if data.len() > MAX_CONTENT_SIZE => return Err(Error::ContentSizeExceeded),
            _ => (),
        }

        if let Communication::DispenseReq(c) = &communication {
            if c.based_on.is_none() {
                return Err(Error::MissingFieldBasedOn);
            }
        }

        communication.set_sent(Utc::now().into());
        match (&mut communication, &participant_id) {
            (Communication::InfoReq(c), ParticipantId::Kvnr(id)) => c.sender = Some(id.clone()),
            (Communication::Reply(c), ParticipantId::TelematikId(id)) => {
                c.sender = Some(id.clone())
            }
            (Communication::DispenseReq(c), ParticipantId::Kvnr(id)) => c.sender = Some(id.clone()),
            (Communication::Representative(c), ParticipantId::Kvnr(id)) => {
                c.sender = Some(id.clone())
            }
            (_, _) => return Err(Error::InvalidSender),
        }

        let sender_eq_recipient = match &mut communication {
            Communication::InfoReq(c) => &*c.recipient == c.sender.as_deref().unwrap(),
            Communication::Reply(c) => &*c.recipient == c.sender.as_deref().unwrap(),
            Communication::DispenseReq(c) => &*c.recipient == c.sender.as_deref().unwrap(),
            Communication::Representative(c) => &*c.recipient == c.sender.as_deref().unwrap(),
        };

        if sender_eq_recipient {
            return Err(Error::SenderEqualRecipient);
        }

        if let Some(based_on) = communication.based_on() {
            let (task_id, access_code) = Self::parse_task_url(&based_on)?;

            let kvnr = participant_id.kvnr().cloned();
            let task_meta = match self.tasks.get(&task_id) {
                Some(task_meta) => task_meta,
                None => return Err(Error::UnknownTask(task_id)),
            };

            let task = task_meta.history.get();
            if !Self::task_matches(&task, &kvnr, &access_code, &None) {
                return Err(Error::UnauthorizedTaskAccess);
            }

            match (&communication, task.status) {
                (Communication::Representative(_), Status::Ready) => (),
                (Communication::Representative(_), Status::InProgress) => (),
                (Communication::Representative(_), _) => return Err(Error::InvalidTaskStatus),
                (_, _) => (),
            }
        }

        let id = Id::generate().unwrap();
        communication.set_id(Some(id.clone()));

        let communication = match self.communications.entry(id) {
            Entry::Occupied(e) => panic!("Communication does already exists: {}", e.key()),
            Entry::Vacant(e) => e.insert(communication),
        };

        Ok(communication)
    }

    pub fn communication_get_mut(
        &mut self,
        id: Id,
        participant_id: &ParticipantId,
    ) -> Result<CommunicationRefMut<'_>, Error> {
        let c = match self.communications.get_mut(&id) {
            Some(c) => c,
            None => return Err(Error::NotFound(id)),
        };

        match communication_matches(c, participant_id) {
            Match::Sender => Ok(CommunicationRefMut::Sender(c)),
            Match::Recipient => Ok(CommunicationRefMut::Recipient(c)),
            Match::Unauthorized => Err(Error::Unauthorized(id)),
        }
    }

    pub fn communication_iter_mut<F>(
        &mut self,
        participant_id: ParticipantId,
        mut f: F,
    ) -> impl Iterator<Item = CommunicationRefMut<'_>>
    where
        F: FnMut(&Communication) -> bool,
    {
        self.communications.iter_mut().filter_map(move |(_, c)| {
            match communication_matches(c, &participant_id) {
                Match::Sender if f(c) => Some(CommunicationRefMut::Sender(c)),
                Match::Recipient if f(c) => Some(CommunicationRefMut::Recipient(c)),
                _ => None,
            }
        })
    }

    pub fn communication_delete(
        &mut self,
        id: Id,
        participant_id: &ParticipantId,
    ) -> Result<Option<DateTime>, Error> {
        let c = match self.communications.get(&id) {
            Some(c) => c,
            None => return Err(Error::NotFound(id)),
        };

        if communication_matches(c, participant_id) == Match::Unauthorized {
            return Err(Error::Unauthorized(id));
        }

        let c = self.communications.remove(&id).unwrap();

        let received = match c {
            Communication::DispenseReq(c) => c.received,
            Communication::InfoReq(c) => c.received,
            Communication::Reply(c) => c.received,
            Communication::Representative(c) => c.received,
        };

        let received = received.map(Into::into);

        Ok(received)
    }

    pub fn parse_task_url(uri: &str) -> Result<(Id, Option<XAccessCode>), Error> {
        let url = format!("http://localhost/{}", uri);
        let url = Url::from_str(&url).map_err(|_| Error::InvalidTaskUri(uri.into()))?;

        let mut path = url
            .path_segments()
            .ok_or_else(|| Error::InvalidTaskUri(uri.into()))?;
        if path.next() != Some("Task") {
            return Err(Error::InvalidTaskUri(uri.into()));
        }

        let task_id = path
            .next()
            .ok_or_else(|| Error::InvalidTaskUri(uri.into()))?;
        let task_id = task_id
            .try_into()
            .map_err(|_| Error::InvalidTaskUri(uri.into()))?;

        let access_code = url.query_pairs().find_map(|(key, value)| {
            if key == "ac" {
                Some(XAccessCode(value.into_owned()))
            } else {
                None
            }
        });

        Ok((task_id, access_code))
    }
}

#[derive(Debug, Eq, PartialEq)]
enum Match {
    Sender,
    Recipient,
    Unauthorized,
}

fn communication_matches(communication: &Communication, participant_id: &ParticipantId) -> Match {
    let kvnr = participant_id.kvnr();
    let telematik_id = participant_id.telematik_id();

    match communication {
        Communication::InfoReq(CommunicationInner {
            sender, recipient, ..
        })
        | Communication::DispenseReq(CommunicationInner {
            sender, recipient, ..
        }) => {
            match (kvnr, sender) {
                (Some(kvnr), Some(sender)) if kvnr == sender => {
                    return Match::Sender;
                }
                _ => (),
            }

            match (telematik_id, recipient) {
                (Some(telematik_id), recipient) if telematik_id == recipient => {
                    return Match::Recipient;
                }
                _ => (),
            }

            Match::Unauthorized
        }
        Communication::Reply(CommunicationInner {
            sender, recipient, ..
        }) => {
            match (telematik_id, sender) {
                (Some(telematik_id), Some(sender)) if telematik_id == sender => {
                    return Match::Sender;
                }
                _ => (),
            }

            match (kvnr, recipient) {
                (Some(kvnr), recipient) if kvnr == recipient => {
                    return Match::Recipient;
                }
                _ => (),
            }

            Match::Unauthorized
        }
        Communication::Representative(CommunicationInner {
            sender, recipient, ..
        }) => {
            match (kvnr, sender) {
                (Some(kvnr), Some(sender)) if kvnr == sender => {
                    return Match::Sender;
                }
                _ => (),
            }

            match (kvnr, recipient) {
                (Some(kvnr), recipient) if kvnr == recipient => {
                    return Match::Recipient;
                }
                _ => (),
            }

            Match::Unauthorized
        }
    }
}

const MAX_CONTENT_SIZE: usize = 10 * 1024;
