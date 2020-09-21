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

use actix_web::{dev::Payload, FromRequest, HttpRequest};
use bytes::BytesMut;
use futures::{
    future::{FutureExt, LocalBoxFuture},
    stream::StreamExt,
};
use serde::de::DeserializeOwned;

use crate::{fhir::json::from_slice, service::RequestError};

pub struct Data<T>(pub T);

#[derive(Clone)]
pub struct Config {
    limit: usize,
}

impl<T> FromRequest for Data<T>
where
    T: DeserializeOwned + 'static,
{
    type Error = RequestError;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;
    type Config = Config;

    #[inline]
    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let limit = req
            .app_data::<Self::Config>()
            .map(|c| c.limit)
            .unwrap_or(32768);

        let mut payload = payload.take();

        (async move {
            let mut body = BytesMut::with_capacity(8192);

            while let Some(item) = payload.next().await {
                let chunk = item?;

                if (body.len() + chunk.len()) > limit {
                    return Err(RequestError::PayloadToLarge(limit));
                } else {
                    body.extend_from_slice(&chunk);
                }
            }

            let data: T = from_slice(&body).map_err(RequestError::DeserializeJson)?;

            Ok(Data(data))
        })
        .boxed_local()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self { limit: 32768 }
    }
}
