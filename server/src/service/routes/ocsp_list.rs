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
    web::{get, resource, Data, ServiceConfig},
    HttpResponse,
};

use crate::pki_store::PkiStore;

pub fn configure_routes(cfg: &mut ServiceConfig) {
    cfg.service(resource("/OCSPList").route(get().to(get_ocsp_list)));
}

async fn get_ocsp_list(pki_store: Data<PkiStore>) -> Result<HttpResponse, ActixError> {
    let cert_list = pki_store.ocsp_list().data().await;

    Ok(HttpResponse::Ok().json2(&*cert_list))
}
