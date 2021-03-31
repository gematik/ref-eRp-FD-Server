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
use rand::{distributions::Standard, rngs::OsRng, Rng};

pub fn configure_routes(cfg: &mut ServiceConfig) {
    cfg.service(resource("/Random").route(get().to(get_random)));
}

async fn get_random() -> Result<HttpResponse, ActixError> {
    let random = OsRng
        .sample_iter(&Standard)
        .take(128)
        .map(|x: u8| format!("{:02x}", x))
        .collect::<Vec<_>>()
        .join("");
    let random = format!("\"{}\"", random);

    let res = HttpResponse::Ok()
        .content_type("application/json")
        .body(random);

    Ok(res)
}
