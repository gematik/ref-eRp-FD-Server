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
use resources::{primitives::Id, KbvBundle, Task};

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
            Some(Ok(task)) => {
                bundle.add_task(task);

                if let Some(id) = task.input.e_prescription.as_ref() {
                    if let Some(res) = state.e_prescriptions.get(id) {
                        bundle.add_kbv_bundle(res);
                    }
                }

                if let Some(id) = task.input.patient_receipt.as_ref() {
                    if let Some(res) = state.patient_receipts.get(id) {
                        bundle.add_kbv_bundle(res);
                    }
                }
            }
            Some(Err(())) => return Ok(HttpResponse::Forbidden().finish()),
            None => return Ok(HttpResponse::NotFound().finish()),
        }
    } else {
        for task in state.iter_tasks(kvnr, access_code) {
            bundle.add_task(task);

            if let Some(id) = task.input.e_prescription.as_ref() {
                if let Some(res) = state.e_prescriptions.get(id) {
                    bundle.add_kbv_bundle(res);
                }
            }

            if let Some(id) = task.input.patient_receipt.as_ref() {
                if let Some(res) = state.patient_receipts.get(id) {
                    bundle.add_kbv_bundle(res);
                }
            }
        }
    }

    bundle.to_response()
}

trait BundleHelper<'a> {
    fn add_task(&mut self, task: &'a Task);
    fn add_kbv_bundle(&mut self, bundle: &'a KbvBundle);
    fn to_response(&self) -> Result<HttpResponse, RequestError>;
}

#[cfg(feature = "support-xml")]
mod xml {
    use std::borrow::Cow;

    use actix_web::HttpResponse;
    use resources::{
        bundle::{Bundle, Entry, Type as BundleType},
        KbvBundle, Task,
    };
    use serde::Serialize;

    use crate::{
        fhir::xml::{
            definitions::{
                BundleRoot as XmlBundle, KbvBundleCow as XmlKbvBundle, TaskCow as XmlTask,
            },
            to_string as to_xml,
        },
        service::{misc::DataType, RequestError},
    };

    use super::BundleHelper as BundleHelperTrait;

    pub struct BundleHelper<'a> {
        bundle: Bundle<Resource<'a>>,
    }

    #[derive(Clone, Serialize)]
    #[allow(clippy::large_enum_variant)]
    pub enum Resource<'a> {
        Task(XmlTask<'a>),
        Bundle(XmlKbvBundle<'a>),
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

        fn add_kbv_bundle(&mut self, bundle: &'a KbvBundle) {
            let bundle = XmlKbvBundle(Cow::Borrowed(bundle));
            let resource = Resource::Bundle(bundle);
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
        KbvBundle, Task,
    };
    use serde::Serialize;

    use crate::{
        fhir::json::{
            definitions::{
                BundleRoot as JsonBundle, KbvBundleCow as JsonKbvBundle, TaskCow as JsonTask,
            },
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
    #[allow(clippy::large_enum_variant)]
    pub enum Resource<'a> {
        Task(JsonTask<'a>),
        Bundle(JsonKbvBundle<'a>),
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

        fn add_kbv_bundle(&mut self, bundle: &'a KbvBundle) {
            let bundle = JsonKbvBundle(Cow::Borrowed(bundle));
            let resource = Resource::Bundle(bundle);
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
