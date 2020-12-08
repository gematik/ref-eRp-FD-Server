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

use actix_web::{web::Path, HttpResponse};
use resources::{
    bundle::{Bundle, Entry, Type},
    primitives::Id,
};

use crate::service::{
    header::{Accept, Authorization},
    misc::{create_response, DataType, DEVICE},
    TypedRequestError, TypedRequestResult,
};

pub async fn get_all(
    accept: Accept,
    _access_token: Authorization,
) -> Result<HttpResponse, TypedRequestError> {
    let accept = DataType::from_accept(&accept)
        .and_then(DataType::ignore_any)
        .unwrap_or_default()
        .check_supported()
        .err_with_type_default()?;

    let device = &*DEVICE;

    let mut entry = Entry::new(device);
    entry.url = Some(format!("/Device/{}", device.id));

    let mut bundle = Bundle::new(Type::Searchset);
    bundle.entries.push(entry);

    create_response(&bundle, accept)
}

pub async fn get_one(
    id: Path<Id>,
    accept: Accept,
    _access_token: Authorization,
) -> Result<HttpResponse, TypedRequestError> {
    let accept = DataType::from_accept(&accept)
        .and_then(DataType::ignore_any)
        .unwrap_or_default()
        .check_supported()
        .err_with_type_default()?;

    if id.0 != DEVICE.id {
        Ok(HttpResponse::NotFound().finish())
    } else {
        create_response(&*DEVICE, accept)
    }
}
