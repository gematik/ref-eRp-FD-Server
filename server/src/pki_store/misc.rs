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

use std::env::var;

use chrono::{DateTime, Duration, Utc};
use glob::Pattern;
use log::warn;
use openssl::{
    asn1::{Asn1Time, Asn1TimeRef},
    hash::MessageDigest,
    ocsp::{OcspCertId, OcspFlag, OcspRequest, OcspResponse, OcspResponseStatus},
    stack::Stack,
    x509::X509Ref,
};
use reqwest::{
    header::CONTENT_TYPE, Body, Client as HttpClient, Error as ReqwestError, IntoUrl, Proxy,
    RequestBuilder, Url,
};
use rustls::ClientConfig;
use rustls_native_certs::load_native_certs;

use super::{Error, TimeCheck, Tsl};

pub struct Client {
    http_proxy: HttpClient,
    http_no_proxy: HttpClient,
    no_proxy: Vec<Pattern>,
}

impl Client {
    pub fn new() -> Result<Self, Error> {
        let no_proxy = if let Ok(no_proxy) = var("no_proxy") {
            no_proxy
                .split(',')
                .map(Pattern::new)
                .filter_map(|pattern| match pattern {
                    Ok(pattern) => Some(pattern),
                    Err(err) => {
                        warn!("Invalid pattern in NO_PROXY environment variable: {}", err);

                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        let mut tls_config = ClientConfig::new();
        tls_config.root_store = load_native_certs().map_err(|(_, err)| err)?;
        tls_config
            .root_store
            .add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);

        let mut http = HttpClient::builder()
            .use_preconfigured_tls(tls_config)
            .user_agent("ref-erx-fd-server");

        if let Ok(http_proxy) = var("http_proxy") {
            http = http.proxy(Proxy::http(&http_proxy)?);
        }

        if let Ok(https_proxy) = var("https_proxy") {
            http = http.proxy(Proxy::https(&https_proxy)?);
        }

        let http_proxy = http.build()?;
        let http_no_proxy = HttpClient::builder()
            .user_agent("ref-erx-fd-server")
            .no_proxy()
            .build()?;

        Ok(Self {
            http_proxy,
            http_no_proxy,
            no_proxy,
        })
    }

    pub fn get<U: IntoUrl>(&self, url: U) -> Result<RequestBuilder, ReqwestError> {
        let url = url.into_url()?;
        let http = self.get_client(&url);

        Ok(http.get(url))
    }

    pub fn post<U: IntoUrl>(&self, url: U) -> Result<RequestBuilder, ReqwestError> {
        let url = url.into_url()?;
        let http = self.get_client(&url);

        Ok(http.post(url))
    }

    pub async fn get_ocsp_response(
        &self,
        tsl: &Tsl,
        cert: &X509Ref,
    ) -> Result<OcspResponse, Error> {
        let issuer = tsl.verify_cert(cert, TimeCheck::None)?;

        for supply_point in &issuer.supply_points {
            match self
                .send_ocsp_req(tsl, &supply_point, &issuer.cert, &cert)
                .await
            {
                Ok(res) => return Ok(res),
                Err(err) => {
                    let key = Tsl::cert_key(cert.subject_name())
                        .unwrap_or_else(|_| "<unknown>".to_owned());

                    warn!(
                        "Unable to fetch OCSP response (cert={}, supply_point={}): {}",
                        key, &supply_point, err
                    );
                }
            }
        }

        Err(Error::FetchingOcspResponseFailed)
    }

    async fn send_ocsp_req(
        &self,
        tsl: &Tsl,
        url: &str,
        issuer: &X509Ref,
        cert: &X509Ref,
    ) -> Result<OcspResponse, Error> {
        let cert_id = OcspCertId::from_cert(MessageDigest::sha1(), &cert, &issuer)?;

        let mut req = OcspRequest::new()?;
        req.add_id(cert_id)?;
        req.add_nonce()?;

        let req = req.to_der()?;
        let req = Body::from(req);

        let res = self
            .post(url)?
            .header(CONTENT_TYPE, "application/ocsp-request")
            .body(req)
            .send()
            .await?;
        if res.status() != 200 {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();

            return Err(Error::InvalidResponse(status, text));
        }
        let res = res.bytes().await?;
        let res = OcspResponse::from_der(&res)?;

        let status = res.status();
        if status != OcspResponseStatus::SUCCESSFUL {
            return Err(Error::InvalidOcspStatus(status));
        }

        let basic = res.basic()?;
        let ret = basic.verify(
            &tsl.stack,
            &tsl.store,
            OcspFlag::NO_INTERN | OcspFlag::NO_CHAIN | OcspFlag::TRUST_OTHER,
        );

        if let Err(err) = ret {
            if let Some(contained) = basic.certs() {
                let mut certs = Stack::new()?;

                for cert in contained {
                    if tsl.verify_cert(cert, TimeCheck::Now).is_ok() {
                        certs.push(cert.to_owned())?;
                    }
                }

                basic.verify(
                    &certs,
                    &tsl.store,
                    OcspFlag::NO_INTERN | OcspFlag::NO_CHAIN | OcspFlag::TRUST_OTHER,
                )?;
            } else {
                return Err(err.into());
            }
        }

        Ok(res)
    }

    fn get_client(&self, url: &Url) -> &HttpClient {
        let domain = url.domain();
        let no_proxy = match domain {
            Some(domain) => self.no_proxy.iter().any(|p| p.matches(domain)),
            None => false,
        };

        if no_proxy {
            &self.http_no_proxy
        } else {
            &self.http_proxy
        }
    }
}

pub fn asn1_to_chrono(time: &Asn1TimeRef) -> DateTime<Utc> {
    let now = Utc::now();
    let asn1_now = Asn1Time::from_unix(now.timestamp()).unwrap();
    let diff = time.diff(&asn1_now).unwrap();

    now - Duration::days(diff.days as _)
        - Duration::seconds(diff.secs as _)
        - Duration::nanoseconds(now.timestamp_subsec_nanos() as _)
}

pub fn check_cert_time(cert: &X509Ref, time: Option<&DateTime<Utc>>) -> Result<(), Error> {
    let now = Utc::now();
    let time = time.unwrap_or(&now);
    let not_after = asn1_to_chrono(cert.not_after());
    let not_before = asn1_to_chrono(cert.not_before());

    if *time < not_before {
        return Err(Error::CertNotValidYet);
    } else if *time > not_after {
        return Err(Error::CertNotValidAnymore);
    }

    Ok(())
}
