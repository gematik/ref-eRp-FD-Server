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

use std::sync::Arc;

use actix_web::{
    error::Error as ActixError,
    http::header::{ContentType, IntoHeaderValue},
    web::{get, resource, Data, ServiceConfig},
    HttpResponse,
};
use arc_swap::ArcSwapOption;

use crate::tsl::Tsl;

pub fn configure_routes(cfg: &mut ServiceConfig) {
    cfg.service(resource("/TSL.xml").route(get().to(get_xml)));
    cfg.service(resource("/TSL.sha2").route(get().to(get_sha2)));
}

async fn get_xml(tsl: Data<Arc<ArcSwapOption<Tsl>>>) -> Result<HttpResponse, ActixError> {
    match tsl.load_full() {
        Some(tsl) => Ok(HttpResponse::Ok()
            .content_type(ContentType::xml().try_into()?)
            .body(&tsl.xml)),
        None => Ok(HttpResponse::NotFound().finish()),
    }
}

async fn get_sha2(tsl: Data<Arc<ArcSwapOption<Tsl>>>) -> Result<HttpResponse, ActixError> {
    match tsl.load_full() {
        Some(tsl) => Ok(HttpResponse::Ok()
            .content_type(ContentType::plaintext().try_into()?)
            .body(&tsl.sha2)),
        None => Ok(HttpResponse::NotFound().finish()),
    }
}
