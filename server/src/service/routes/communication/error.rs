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

use resources::primitives::Id;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Content Size Exceeded!")]
    ContentSizeExceeded,

    #[error("Missing Field: 'basedOn'!")]
    MissingFieldBasedOn,

    #[error("Sender is equal to recipient!")]
    SenderEqualRecipient,

    #[error("Invalid Sender!")]
    InvalidSender,

    #[error("Unknown Task: {0}!")]
    UnknownTask(Id),

    #[error("Access to Task not authorized!")]
    UnauthorizedTaskAccess,

    #[error("Invalid Task Status!")]
    InvalidTaskStatus,

    #[error("Invalid Task Uri: {0}!")]
    InvalidTaskUri(String),

    #[error("Not Found: /Communication/{0}!")]
    NotFound(Id),

    #[error("Unauthorized: /Communication/{0}!")]
    Unauthorized(Id),

    #[error("Communications Exceeded!")]
    CommunicationsExceeded,
}
