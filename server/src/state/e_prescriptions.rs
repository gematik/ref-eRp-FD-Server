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

use std::collections::hash_map::{Entry, HashMap};

use resources::{primitives::Id, KbvBinary};

#[derive(Default)]
pub struct EPrescriptions {
    by_id: HashMap<Id, KbvBinary>,
}

impl EPrescriptions {
    pub fn insert(&mut self, id: Id, binary: KbvBinary) {
        match self.by_id.entry(id) {
            Entry::Occupied(e) => {
                panic!(
                    "ePrescription with this ID ({}) does already exist!",
                    e.key()
                );
            }
            Entry::Vacant(entry) => {
                entry.insert(binary);
            }
        }
    }

    pub fn contains(&self, id: &Id) -> bool {
        self.by_id.contains_key(id)
    }

    pub fn get_by_id(&self, id: &Id) -> Option<&KbvBinary> {
        self.by_id.get(id)
    }

    pub fn remove_by_id(&mut self, id: &Id) {
        self.by_id.remove(id).expect("ePrescription not found!");
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Id, &KbvBinary)> {
        self.by_id.iter()
    }
}
