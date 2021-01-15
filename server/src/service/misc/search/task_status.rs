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

use resources::task::Status;

use super::{Comperator, Parameter};

impl Parameter for Status {
    type Storage = Status;

    fn parse(s: &str) -> Result<Self::Storage, String> {
        match s {
            "draft" => Ok(Status::Draft),
            "requested" => Ok(Status::Requested),
            "received" => Ok(Status::Received),
            "accepted" => Ok(Status::Accepted),
            "rejected" => Ok(Status::Rejected),
            "ready" => Ok(Status::Ready),
            "cancelled" => Ok(Status::Cancelled),
            "in-progress" => Ok(Status::InProgress),
            "on-hold" => Ok(Status::OnHold),
            "failed" => Ok(Status::Failed),
            "completed" => Ok(Status::Completed),
            "entered-in-error" => Ok(Status::EnteredInError),
            s => Err(format!("Invalid status: {}", s)),
        }
    }

    fn compare(&self, comperator: Comperator, param: &Self::Storage) -> bool {
        match comperator {
            Comperator::Equal => self == param,
            Comperator::NotEqual => self != param,
            _ => false,
        }
    }
}
