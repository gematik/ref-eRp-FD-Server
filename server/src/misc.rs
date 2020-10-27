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

use std::env::var;

use reqwest::{Client, Error, Proxy};

pub fn create_reqwest_client() -> Result<Client, Error> {
    let mut client = Client::builder();

    if let Ok(http_proxy) = var("http_proxy") {
        client = client.proxy(Proxy::http(&http_proxy)?);
    }

    if let Ok(https_proxy) = var("https_proxy") {
        client = client.proxy(Proxy::https(&https_proxy)?);
    }

    let client = client.build()?;

    Ok(client)
}
