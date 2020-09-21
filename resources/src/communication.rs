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

use super::{
    misc::{Decode, Encode, Kvnr, TelematikId},
    primitives::{DateTime, Id},
    types::FlowType,
    Medication,
};

#[derive(Clone, PartialEq, Debug)]
pub enum Communication {
    InfoReq(Inner<InfoReqExtensions, TelematikId, Kvnr>),
    Reply(Inner<ReplyExtensions, Kvnr, TelematikId>),
    DispenseReq(Inner<DispenseReqExtensions, TelematikId, Kvnr>),
    Representative(Inner<RepresentativeExtensions, Kvnr, Kvnr>),
}

#[derive(Clone, PartialEq, Debug)]
pub struct Inner<E, R, S>
where
    E: Clone + PartialEq,
    R: Clone + PartialEq,
    S: Clone + PartialEq,
{
    pub id: Option<Id>,
    pub based_on: Option<String>,
    pub about: Vec<Medication>,
    pub sent: Option<DateTime>,
    pub received: Option<DateTime>,
    pub recipient: R,
    pub sender: Option<S>,
    pub payload: Payload<E>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Payload<E: Clone + PartialEq> {
    pub content: String,
    pub extensions: Option<E>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct InfoReqExtensions {
    pub insurance_provider: String,
    pub substitution_allowed: bool,
    pub prescription_type: FlowType,
    pub preferred_supply_options: Option<SupplyOptions>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ReplyExtensions {
    pub availability: Option<Availability>,
    pub offered_supply_options: Option<SupplyOptions>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct DispenseReqExtensions {
    pub insurance_provider: Option<String>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct RepresentativeExtensions;

#[derive(Clone, PartialEq, Debug)]
pub struct SupplyOptions {
    pub on_premise: bool,
    pub delivery: bool,
    pub shipment: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Availability {
    Now,
    Today,
    MorningNextDay,
    AfternoonNextDay,
    Unavailable,
    Unknown,
}

impl Communication {
    pub fn id(&self) -> &Option<Id> {
        match self {
            Communication::InfoReq(inner) => &inner.id,
            Communication::Reply(inner) => &inner.id,
            Communication::DispenseReq(inner) => &inner.id,
            Communication::Representative(inner) => &inner.id,
        }
    }

    pub fn based_on(&self) -> &Option<String> {
        match self {
            Communication::InfoReq(inner) => &inner.based_on,
            Communication::Reply(inner) => &inner.based_on,
            Communication::DispenseReq(inner) => &inner.based_on,
            Communication::Representative(inner) => &inner.based_on,
        }
    }

    pub fn sent(&self) -> &Option<DateTime> {
        match self {
            Communication::InfoReq(inner) => &inner.sent,
            Communication::Reply(inner) => &inner.sent,
            Communication::DispenseReq(inner) => &inner.sent,
            Communication::Representative(inner) => &inner.sent,
        }
    }

    pub fn received(&self) -> &Option<DateTime> {
        match self {
            Communication::InfoReq(inner) => &inner.received,
            Communication::Reply(inner) => &inner.received,
            Communication::DispenseReq(inner) => &inner.received,
            Communication::Representative(inner) => &inner.received,
        }
    }

    pub fn sender(&self) -> Option<String> {
        match self {
            Communication::InfoReq(inner) => {
                inner.sender.as_ref().map(Clone::clone).map(Into::into)
            }
            Communication::Reply(inner) => inner.sender.as_ref().map(Clone::clone).map(Into::into),
            Communication::DispenseReq(inner) => {
                inner.sender.as_ref().map(Clone::clone).map(Into::into)
            }
            Communication::Representative(inner) => {
                inner.sender.as_ref().map(Clone::clone).map(Into::into)
            }
        }
    }

    pub fn recipient(&self) -> String {
        match self {
            Communication::InfoReq(inner) => inner.recipient.clone().into(),
            Communication::Reply(inner) => inner.recipient.clone().into(),
            Communication::DispenseReq(inner) => inner.recipient.clone().into(),
            Communication::Representative(inner) => inner.recipient.clone().into(),
        }
    }

    pub fn content(&self) -> &String {
        match self {
            Communication::InfoReq(inner) => &inner.payload.content,
            Communication::Reply(inner) => &inner.payload.content,
            Communication::DispenseReq(inner) => &inner.payload.content,
            Communication::Representative(inner) => &inner.payload.content,
        }
    }

    pub fn set_id(&mut self, id: Option<Id>) {
        match self {
            Communication::InfoReq(inner) => inner.id = id,
            Communication::Reply(inner) => inner.id = id,
            Communication::DispenseReq(inner) => inner.id = id,
            Communication::Representative(inner) => inner.id = id,
        }
    }

    pub fn set_sent(&mut self, value: DateTime) {
        match self {
            Communication::InfoReq(inner) => inner.sent = Some(value),
            Communication::Reply(inner) => inner.sent = Some(value),
            Communication::DispenseReq(inner) => inner.sent = Some(value),
            Communication::Representative(inner) => inner.sent = Some(value),
        }
    }

    pub fn set_received(&mut self, value: DateTime) {
        match self {
            Communication::InfoReq(inner) => inner.received = Some(value),
            Communication::Reply(inner) => inner.received = Some(value),
            Communication::DispenseReq(inner) => inner.received = Some(value),
            Communication::Representative(inner) => inner.received = Some(value),
        }
    }
}

impl Decode for Availability {
    type Code = usize;
    type Auto = ();

    fn decode(code: Self::Code) -> Result<Self, Self::Code> {
        match code {
            10 => Ok(Self::Now),
            20 => Ok(Self::Today),
            30 => Ok(Self::MorningNextDay),
            40 => Ok(Self::AfternoonNextDay),
            50 => Ok(Self::Unavailable),
            90 => Ok(Self::Unknown),
            _ => Err(code),
        }
    }
}

impl Encode for Availability {
    type Code = usize;
    type Auto = ();

    fn encode(&self) -> Self::Code {
        match self {
            Self::Now => 10,
            Self::Today => 20,
            Self::MorningNextDay => 30,
            Self::AfternoonNextDay => 40,
            Self::Unavailable => 50,
            Self::Unknown => 90,
        }
    }
}
