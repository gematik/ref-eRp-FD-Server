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

use resources::{misc::Kvnr, primitives::Id, MedicationDispense};

use crate::state::Inner;

use super::Error;

impl Inner {
    pub fn medication_dispense_get(
        &self,
        id: Id,
        kvnr: &Kvnr,
    ) -> Result<&MedicationDispense, Error> {
        let value = match self.medication_dispenses.get(&id) {
            Some(value) => value,
            None => return Err(Error::NotFound(id)),
        };

        if &value.subject == kvnr {
            Ok(value)
        } else {
            Err(Error::Forbidden(id))
        }
    }

    pub fn medication_dispense_iter<'a, F>(
        &'a self,
        kvnr: &'a Kvnr,
        f: F,
    ) -> impl Iterator<Item = &'a MedicationDispense>
    where
        F: Fn(&MedicationDispense) -> bool,
    {
        self.medication_dispenses.values().filter(move |md| {
            if &md.subject != kvnr {
                return false;
            }

            f(md)
        })
    }
}
