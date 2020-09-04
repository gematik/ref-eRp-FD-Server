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

use std::collections::HashMap;
use std::sync::Arc;

use resources::{primitives::Id, KbvBundle, Task};
use tokio::sync::{Mutex, MutexGuard};

use super::idp_client::IdpClient;

#[derive(Default, Clone)]
pub struct State(Arc<Mutex<Inner>>);

#[derive(Default)]
pub struct Inner {
    pub idp_client: IdpClient,
    pub tasks: HashMap<Id, Task>,
    pub e_prescriptions: HashMap<Id, KbvBundle>,
    pub patient_receipts: HashMap<Id, KbvBundle>,
}

impl State {
    pub async fn lock(&self) -> MutexGuard<'_, Inner> {
        self.0.lock().await
    }
}
