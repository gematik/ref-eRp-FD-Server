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
use resources::{primitives::Id, types::Profession, Task};

use super::super::{
    super::{
        error::Error,
        header::{Accept, Authorization, XAccessCode},
        state::State,
    },
    misc::DataType,
};

pub async fn get_all(
    state: Data<State>,
    accept: Accept,
    id_token: Authorization,
    access_code: Option<XAccessCode>,
) -> Result<HttpResponse, Error> {
    get(&state, None, accept, id_token, access_code).await
}

pub async fn get_one(
    state: Data<State>,
    id: Path<Id>,
    accept: Accept,
    id_token: Authorization,
    access_code: Option<XAccessCode>,
) -> Result<HttpResponse, Error> {
    get(&state, Some(id.into_inner()), accept, id_token, access_code).await
}

#[allow(unreachable_code)]
async fn get(
    state: &State,
    id: Option<Id>,
    accept: Accept,
    id_token: Authorization,
    access_code: Option<XAccessCode>,
) -> Result<HttpResponse, Error> {
    let id_token = id_token.0;
    let access_code = access_code.map(|access_code| access_code.0);
    let state = state.lock().await;

    match state
        .idp_client
        .get_profession(&id_token)
        .map_err(Error::IdpClientError)?
    {
        Profession::Insured | Profession::PublicPharmacy | Profession::HospitalPharmacy => (),
        profession => return Err(Error::InvalidProfession(profession)),
    }

    let kvnr = state
        .idp_client
        .get_kvnr(&id_token)
        .map_err(Error::IdpClientError)?;

    let data_type = DataType::from_accept(accept)
        .and_then(DataType::ignore_any)
        .unwrap_or_default();

    let mut bundle: Box<dyn BundleHelper> = match data_type {
        #[cfg(feature = "support-xml")]
        DataType::Xml => Box::new(xml::BundleHelper::new()),

        #[cfg(feature = "support-json")]
        DataType::Json => Box::new(json::BundleHelper::new()),

        DataType::Unknown | DataType::Any => return Err(Error::AcceptUnsupported),
    };

    if let Some(id) = id {
        let task = match state.tasks.get(&id) {
            Some(task) => task,
            None => return Ok(HttpResponse::NotFound().finish()),
        };

        if !task_matches(task, kvnr.as_ref(), access_code.as_ref()) {
            return Ok(HttpResponse::Forbidden().finish());
        }

        bundle.add_task(task);
    } else {
        for (_, task) in state.tasks.iter() {
            if !task_matches(task, kvnr.as_ref(), access_code.as_ref()) {
                continue;
            }

            bundle.add_task(task);
        }
    }

    bundle.to_response()
}

fn task_matches(task: &Task, kvnr: Option<&String>, access_code: Option<&String>) -> bool {
    match (task.for_.as_ref(), kvnr) {
        (Some(task_kvnr), Some(kvnr)) if task_kvnr == kvnr => return true,
        _ => (),
    }

    match (task.identifier.access_code.as_ref(), access_code) {
        (Some(task_ac), Some(ac)) if task_ac == ac => return true,
        _ => (),
    }

    false
}

trait BundleHelper<'a> {
    fn add_task(&mut self, task: &'a Task);
    fn to_response(&self) -> Result<HttpResponse, Error>;
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

    use crate::fhir::xml::{
        definitions::{BundleRoot as XmlBundle, TaskCow as XmlTask},
        to_string as to_xml,
    };

    use super::{
        super::super::{super::error::Error, misc::DataType},
        BundleHelper as BundleHelperTrait,
    };

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

        fn to_response(&self) -> Result<HttpResponse, Error> {
            let xml = to_xml(&XmlBundle::new(&self.bundle)).map_err(Error::SerializeXml)?;

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

    use crate::fhir::json::{
        definitions::{BundleRoot as JsonBundle, TaskCow as JsonTask},
        to_string as to_json,
    };

    use super::{
        super::super::{super::error::Error, misc::DataType},
        BundleHelper as BundleHelperTrait,
    };

    pub struct BundleHelper<'a> {
        bundle: Bundle<Resource<'a>>,
    }

    #[derive(Clone, Serialize)]
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

        fn to_response(&self) -> Result<HttpResponse, Error> {
            let json = to_json(&JsonBundle::new(&self.bundle)).map_err(Error::SerializeJson)?;

            Ok(HttpResponse::Ok()
                .content_type(DataType::Json.as_mime().to_string())
                .body(json))
        }
    }
}
