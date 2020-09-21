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

use actix_web::{
    web::{Data, Path},
    HttpResponse,
};
use resources::{primitives::Id, Task};

use crate::service::{
    header::{Accept, Authorization, XAccessCode},
    misc::{DataType, Profession},
    state::State,
    RequestError,
};

pub async fn get_all(
    state: Data<State>,
    accept: Accept,
    id_token: Authorization,
    access_code: Option<XAccessCode>,
) -> Result<HttpResponse, RequestError> {
    get(&state, None, accept, id_token, access_code).await
}

pub async fn get_one(
    state: Data<State>,
    id: Path<Id>,
    accept: Accept,
    id_token: Authorization,
    access_code: Option<XAccessCode>,
) -> Result<HttpResponse, RequestError> {
    get(&state, Some(id.into_inner()), accept, id_token, access_code).await
}

#[allow(unreachable_code)]
async fn get(
    state: &State,
    id: Option<Id>,
    accept: Accept,
    access_token: Authorization,
    access_code: Option<XAccessCode>,
) -> Result<HttpResponse, RequestError> {
    access_token.check_profession(|p| p == Profession::Versicherter)?;

    let kvnr = access_token.kvnr().ok();
    let data_type = DataType::from_accept(accept)
        .and_then(DataType::ignore_any)
        .unwrap_or_default()
        .check_supported()?;

    let state = state.lock().await;
    let mut bundle: Box<dyn BundleHelper> = match data_type {
        #[cfg(feature = "support-xml")]
        DataType::Xml => Box::new(xml::BundleHelper::new()),

        #[cfg(feature = "support-json")]
        DataType::Json => Box::new(json::BundleHelper::new()),

        DataType::Unknown | DataType::Any => unreachable!(),
    };

    if let Some(id) = id {
        match state.get_task(&id, &kvnr, &access_code) {
            Some(Ok(task)) => bundle.add_task(task),
            Some(Err(())) => return Ok(HttpResponse::Forbidden().finish()),
            None => return Ok(HttpResponse::NotFound().finish()),
        }
    } else {
        for task in state.iter_tasks(kvnr, access_code) {
            bundle.add_task(task)
        }
    }

    bundle.to_response()
}

trait BundleHelper<'a> {
    fn add_task(&mut self, task: &'a Task);
    fn to_response(&self) -> Result<HttpResponse, RequestError>;
}

#[cfg(feature = "support-xml")]
mod xml {
    use std::borrow::Cow;

    use actix_web::HttpResponse;
    use resources::{
        bundle::{Bundle, Entry, Type as BundleType},
        Task,
    };
    use serde::Serialize;

    use crate::{
        fhir::xml::{
            definitions::{BundleRoot as XmlBundle, TaskCow as XmlTask},
            to_string as to_xml,
        },
        service::{misc::DataType, RequestError},
    };

    use super::BundleHelper as BundleHelperTrait;

    pub struct BundleHelper<'a> {
        bundle: Bundle<Resource<'a>>,
    }

    #[derive(Clone, Serialize)]
    pub enum Resource<'a> {
        Task(XmlTask<'a>),
    }

    impl BundleHelper<'_> {
        pub fn new() -> Self {
            Self {
                bundle: Bundle::new(BundleType::Collection),
            }
        }
    }

    impl<'a> BundleHelperTrait<'a> for BundleHelper<'a> {
        fn add_task(&mut self, task: &'a Task) {
            let task = XmlTask(Cow::Borrowed(task));
            let resource = Resource::Task(task);
            let entry = Entry::new(resource);

            self.bundle.entries.push(entry);
        }

        fn to_response(&self) -> Result<HttpResponse, RequestError> {
            let xml = to_xml(&XmlBundle::new(&self.bundle)).map_err(RequestError::SerializeXml)?;

            Ok(HttpResponse::Ok()
                .content_type(DataType::Xml.as_mime().to_string())
                .body(xml))
        }
    }
}

#[cfg(feature = "support-json")]
mod json {
    use std::borrow::Cow;

    use actix_web::HttpResponse;
    use resources::{
        bundle::{Bundle, Entry, Type as BundleType},
        Task,
    };
    use serde::Serialize;

    use crate::{
        fhir::json::{
            definitions::{BundleRoot as JsonBundle, TaskCow as JsonTask},
            to_string as to_json,
        },
        service::{misc::DataType, RequestError},
    };

    use super::BundleHelper as BundleHelperTrait;

    pub struct BundleHelper<'a> {
        bundle: Bundle<Resource<'a>>,
    }

    #[derive(Clone, Serialize)]
    #[serde(tag = "resourceType")]
    pub enum Resource<'a> {
        Task(JsonTask<'a>),
    }

    impl BundleHelper<'_> {
        pub fn new() -> Self {
            Self {
                bundle: Bundle::new(BundleType::Collection),
            }
        }
    }

    impl<'a> BundleHelperTrait<'a> for BundleHelper<'a> {
        fn add_task(&mut self, task: &'a Task) {
            let task = JsonTask(Cow::Borrowed(task));
            let resource = Resource::Task(task);
            let entry = Entry::new(resource);

            self.bundle.entries.push(entry);
        }

        fn to_response(&self) -> Result<HttpResponse, RequestError> {
            let json =
                to_json(&JsonBundle::new(&self.bundle)).map_err(RequestError::SerializeJson)?;

            Ok(HttpResponse::Ok()
                .content_type(DataType::Json.as_mime().to_string())
                .body(json))
        }
    }
}
