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

use chrono::{DateTime, Utc};

pub struct History<T>
where
    T: Clone,
{
    versions: Vec<Version<T>>,
    offset: usize,
}

pub struct Version<T>
where
    T: Clone,
{
    pub id: usize,
    pub timestamp: DateTime<Utc>,
    pub resource: T,
}

impl<T> History<T>
where
    T: Clone,
{
    pub fn new(resource: T) -> Self {
        Self {
            versions: vec![Version {
                id: 0,
                timestamp: Utc::now(),
                resource,
            }],
            offset: 0,
        }
    }

    pub fn get(&self) -> &T {
        &self.versions.last().unwrap().resource
    }

    pub fn get_mut(&mut self) -> &mut T {
        let current = self.get_current();

        let id = current.id + 1;
        let timestamp = Utc::now();
        let resource = current.resource.clone();

        self.versions.push(Version {
            id,
            timestamp,
            resource,
        });

        &mut self.versions.last_mut().unwrap().resource
    }

    pub fn get_current(&self) -> &Version<T> {
        self.versions.last().unwrap()
    }

    pub fn get_version(&self, mut id: usize) -> Option<&Version<T>> {
        if id < self.offset {
            return None;
        }

        id -= self.offset;

        self.versions.get(id)
    }

    pub fn clear(&mut self) {
        let len = self.versions.len() - 1;
        self.versions.drain(0..len);
        self.offset += len;
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn get() {
        let history = History::new(1);

        assert_eq!(history.get(), &1);
        assert_eq!(history.versions.len(), 1);
    }

    #[test]
    fn get_mut() {
        let mut history = History::new(1);

        *history.get_mut() = 2;

        assert_eq!(history.get(), &2);
        assert_eq!(history.versions.len(), 2);
        assert_eq!(history.versions[0].resource, 1);
        assert_eq!(history.versions[1].resource, 2);
    }

    #[test]
    fn get_version() {
        let mut history = History::new(1);

        *history.get_mut() = 2;

        assert_eq!(history.get_version(0).map(|v| v.resource), Some(1));
        assert_eq!(history.get_version(1).map(|v| v.resource), Some(2));
    }

    #[test]
    fn clear() {
        let mut history = History::new(1);

        *history.get_mut() = 2;
        *history.get_mut() = 3;
        history.clear();

        assert_eq!(history.versions.len(), 1);
        assert_eq!(history.get_version(0).map(|v| v.resource), None);
        assert_eq!(history.get_version(1).map(|v| v.resource), None);
        assert_eq!(history.get_version(2).map(|v| v.resource), Some(3));
    }
}
