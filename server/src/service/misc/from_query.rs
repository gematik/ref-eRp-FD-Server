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

use std::ops::Deref;

use actix_web::{dev::Payload, FromRequest, HttpRequest};
use futures::future::{ready, Ready};

use crate::service::{RequestError, TypedRequestError};

pub struct Query<T: FromQuery>(pub T);

impl<T> Deref for Query<T>
where
    T: FromQuery,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct QueryValue<'a>(pub Option<&'a str>);

impl QueryValue<'_> {
    pub fn ok(&self) -> Result<&str, String> {
        Ok(self.0.ok_or("Query key is missing a value!")?)
    }
}

pub trait FromQuery: Default {
    fn parse_key_value_pair(&mut self, key: &str, value: QueryValue) -> Result<(), String>;
}

impl<T> FromRequest for Query<T>
where
    T: FromQuery,
{
    type Error = TypedRequestError;
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let q = req.query_string();
        let mut ret = T::default();

        let iter = q.split('&');
        for kvp in iter {
            let (key, value) = if let Some(p) = kvp.find('=') {
                (&kvp[..p], Some(&kvp[(p + 1)..]))
            } else {
                (kvp, None)
            };

            if let Err(err) = ret.parse_key_value_pair(key, QueryValue(value)) {
                let err = format!("Invalid query parameter ({}): {}", key, err);

                return ready(Err(RequestError::QueryInvalid(err).with_type_from(req)));
            }
        }

        ready(Ok(Query(ret)))
    }
}
