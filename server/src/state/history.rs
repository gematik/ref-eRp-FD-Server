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

use std::fmt::{Formatter, Result as FmtResult};
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::ops::Deref;

use chrono::{naive::NaiveDateTime, DateTime, Utc};
use serde::{
    de::{Deserialize, Deserializer, Error as DeError, MapAccess, SeqAccess, Visitor},
    ser::{Serialize, SerializeStruct, Serializer},
};
use thiserror::Error;

pub struct History<T>
where
    T: Clone,
{
    versions: Vec<Version<T>>,
    offset: usize,
}

#[derive(Clone)]
pub struct Version<T>
where
    T: Clone,
{
    pub id: usize,
    pub timestamp: DateTime<Utc>,
    pub resource: T,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Version Ids must be continuous!")]
    VersionIdsNotContinuous,

    #[error("Empty History!")]
    EmptyHistory,
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

    pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
        (&self).into_iter()
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

impl<T> FromIterator<Version<T>> for Result<History<T>, Error>
where
    T: Clone,
{
    fn from_iter<I: IntoIterator<Item = Version<T>>>(iter: I) -> Self {
        let mut versions = iter.into_iter().collect::<Vec<_>>();
        versions.sort_by(|a, b| a.id.cmp(&b.id));

        let mut offset = None;
        let mut last_id = None;

        for version in &versions {
            last_id = match last_id {
                Some(last_id) => {
                    if last_id + 1 != version.id {
                        return Err(Error::VersionIdsNotContinuous);
                    }

                    Some(version.id)
                }
                None => {
                    offset = Some(version.id);

                    Some(version.id)
                }
            }
        }

        let offset = offset.ok_or(Error::EmptyHistory)?;

        Ok(History { versions, offset })
    }
}

impl<T> IntoIterator for History<T>
where
    T: Clone,
{
    type Item = <Vec<Version<T>> as IntoIterator>::Item;
    type IntoIter = <Vec<Version<T>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.versions.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a History<T>
where
    T: Clone,
{
    type Item = <&'a Vec<Version<T>> as IntoIterator>::Item;
    type IntoIter = <&'a Vec<Version<T>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.versions.iter()
    }
}

impl<T> Deref for Version<T>
where
    T: Clone,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.resource
    }
}

impl<T> Serialize for Version<T>
where
    T: Clone + Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("Version", 3)?;

        s.serialize_field("version_id", &self.id)?;
        s.serialize_field("timestamp", &self.timestamp.timestamp_nanos())?;
        s.serialize_field("resource", &self.resource)?;

        s.end()
    }
}

impl<'de, T> Deserialize<'de> for Version<T>
where
    T: Clone + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_struct("Version", VERSION_FIELDS, VersionVisitor::<T>::default())
    }
}

struct VersionVisitor<T>(PhantomData<T>);

impl<T> Default for VersionVisitor<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<'de, T> Visitor<'de> for VersionVisitor<T>
where
    T: Clone + Deserialize<'de>,
{
    type Value = Version<T>;

    fn expecting(&self, formatter: &mut Formatter) -> FmtResult {
        formatter.write_str("struct Duration")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Version<T>, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let id = seq
            .next_element()?
            .ok_or_else(|| DeError::invalid_length(0, &self))?;
        let timestamp: i64 = seq
            .next_element()?
            .ok_or_else(|| DeError::invalid_length(1, &self))?;
        let resource = seq
            .next_element()?
            .ok_or_else(|| DeError::invalid_length(1, &self))?;

        let timestamp = NaiveDateTime::from_timestamp(
            timestamp / 1_000_000_000,
            (timestamp % 1_000_000_000) as u32,
        );
        let timestamp = DateTime::from_utc(timestamp, Utc);

        Ok(Version {
            id,
            timestamp,
            resource,
        })
    }

    fn visit_map<V>(self, mut map: V) -> Result<Version<T>, V::Error>
    where
        V: MapAccess<'de>,
    {
        let mut id = None;
        let mut timestamp = None;
        let mut resource = None;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "version_id" => {
                    if id.is_some() {
                        return Err(DeError::duplicate_field("version_id"));
                    }
                    id = Some(map.next_value()?);
                }
                "timestamp" => {
                    if timestamp.is_some() {
                        return Err(DeError::duplicate_field("timestamp"));
                    }
                    timestamp = Some(map.next_value::<i64>()?);
                }
                "resource" => {
                    if resource.is_some() {
                        return Err(DeError::duplicate_field("resource"));
                    }
                    resource = Some(map.next_value()?);
                }
                key => return Err(DeError::unknown_field(key, VERSION_FIELDS)),
            }
        }

        let id = id.ok_or_else(|| DeError::missing_field("version_id"))?;
        let timestamp = timestamp.ok_or_else(|| DeError::missing_field("timestamp"))?;
        let timestamp = NaiveDateTime::from_timestamp(
            timestamp / 1_000_000_000,
            (timestamp % 1_000_000_000) as u32,
        );
        let timestamp = DateTime::from_utc(timestamp, Utc);
        let resource = resource.ok_or_else(|| DeError::missing_field("resource"))?;

        Ok(Version {
            id,
            timestamp,
            resource,
        })
    }
}

const VERSION_FIELDS: &[&str] = &["version_id", "timestamp", "resource"];

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
