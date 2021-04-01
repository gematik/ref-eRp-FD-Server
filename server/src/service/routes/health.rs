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

use actix_web::{
    error::Error as ActixError,
    web::{get, resource, ServiceConfig},
    HttpResponse,
};

pub fn configure_routes(cfg: &mut ServiceConfig) {
    cfg.service(resource("/Health").route(get().to(get_health)));
}

async fn get_health() -> Result<HttpResponse, ActixError> {
    Ok(HttpResponse::Ok().finish())
}
