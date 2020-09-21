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

use std::ops::Deref;

use actix_web::{dev::Payload, http::header::HeaderName, FromRequest, HttpRequest};
use chrono::Utc;
use futures::future::{ready, Ready};

use crate::service::{
    misc::{AccessToken, AccessTokenError, PukToken},
    RequestError,
};

lazy_static! {
    pub static ref AUTHORIZATION: HeaderName =
        HeaderName::from_lowercase(b"authorization").unwrap();
}

pub struct Authorization(pub AccessToken);

impl FromRequest for Authorization {
    type Error = RequestError;
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready(parse_authorization(req))
    }
}

impl Deref for Authorization {
    type Target = AccessToken;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn parse_authorization(req: &HttpRequest) -> Result<Authorization, RequestError> {
    let puk_token = req
        .app_data::<PukToken>()
        .ok_or_else(|| RequestError::internal("Shared data 'PukToken' is missing!"))?;

    let access_token = req
        .headers()
        .get(&*AUTHORIZATION)
        .ok_or_else(|| AccessTokenError::Missing)?
        .to_str()
        .map_err(|_| AccessTokenError::InvalidValue)?;
    let access_token = AccessToken::verify(access_token, puk_token.0.clone(), Utc::now())?;

    Ok(Authorization(access_token))
}
