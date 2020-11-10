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

use base64::{decode, DecodeError as Base64Error};
use chrono::{DateTime, ParseError as ChronoError, Utc};
use openssl::{error::ErrorStack as OpenSslError, x509::X509};
use quick_xml::{de::from_str, DeError as XmlError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("XML Error: {0}")]
    XmlError(XmlError),

    #[error("Chrono Error: {0}")]
    ChronoError(ChronoError),

    #[error("Base64 Error: {0}")]
    Base64Error(Base64Error),

    #[error("OpenSSL Error: {0}")]
    OpenSslError(OpenSslError),
}

pub fn extract(xml: &str) -> Result<Vec<X509>, Error> {
    const IDENT: &str = "http://uri.etsi.org/TrstSvc/Svctype/CA/PKC";
    const STATUS: &str = "http://uri.etsi.org/TrstSvc/Svcstatus/inaccord";
    const EXT_OID: &str = "1.2.276.0.76.4.203";
    const EXT_VALUE: &str = "oid_fd_sig";

    let now = Utc::now();
    let tsl: TrustServiceStatusList = from_str(xml)?;

    let mut certs = Vec::new();

    for provider in tsl.provider_list.provider {
        for service in provider.services.service {
            let info = &service.infos;

            if info.ident != IDENT {
                continue;
            }

            if info.status != STATUS {
                continue;
            }

            let start_time = DateTime::parse_from_rfc3339(&info.starting_time)?;
            if start_time > now {
                continue;
            }

            let has_ext = info.extensions.extension.iter().any(|ex| {
                ex.oid.as_deref() == Some(EXT_OID) && ex.value.as_deref() == Some(EXT_VALUE)
            });
            if !has_ext {
                continue;
            }

            for id in &info.identity.id {
                if let Some(cert) = &id.cert {
                    let cert = cert.trim();
                    let cert = decode(&cert)?;
                    let cert = X509::from_der(&cert)?;

                    certs.push(cert);
                }
            }
        }
    }

    Ok(certs)
}

#[derive(Debug, Serialize, Deserialize)]
struct TrustServiceStatusList {
    #[serde(rename = "TrustServiceProviderList")]
    provider_list: TrustServiceProviderList,
}

#[derive(Debug, Serialize, Deserialize)]
struct TrustServiceProviderList {
    #[serde(rename = "TrustServiceProvider")]
    provider: Vec<TrustServiceProvider>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TrustServiceProvider {
    #[serde(rename = "TSPServices")]
    services: TSPServices,
}

#[derive(Debug, Serialize, Deserialize)]
struct TSPServices {
    #[serde(rename = "TSPService")]
    service: Vec<TSPService>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TSPService {
    #[serde(rename = "ServiceInformation")]
    infos: ServiceInformation,
}

#[derive(Debug, Serialize, Deserialize)]
struct ServiceInformation {
    #[serde(rename = "ServiceTypeIdentifier")]
    ident: String,

    #[serde(rename = "ServiceStatus")]
    status: String,

    #[serde(rename = "StatusStartingTime")]
    starting_time: String,

    #[serde(rename = "ServiceInformationExtensions")]
    extensions: ServiceInformationExtensions,

    #[serde(rename = "ServiceDigitalIdentity")]
    identity: ServiceDigitalIdentity,
}

#[derive(Debug, Serialize, Deserialize)]
struct ServiceInformationExtensions {
    #[serde(rename = "Extension")]
    extension: Vec<Extension>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Extension {
    #[serde(rename = "ExtensionOID")]
    oid: Option<String>,

    #[serde(rename = "ExtensionValue")]
    value: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ServiceDigitalIdentity {
    #[serde(rename = "DigitalId")]
    id: Vec<DigitalId>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DigitalId {
    #[serde(rename = "X509Certificate")]
    cert: Option<String>,
}

impl From<XmlError> for Error {
    fn from(err: XmlError) -> Self {
        Self::XmlError(err)
    }
}

impl From<ChronoError> for Error {
    fn from(err: ChronoError) -> Self {
        Self::ChronoError(err)
    }
}

impl From<Base64Error> for Error {
    fn from(err: Base64Error) -> Self {
        Self::Base64Error(err)
    }
}

impl From<OpenSslError> for Error {
    fn from(err: OpenSslError) -> Self {
        Self::OpenSslError(err)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::fs::read_to_string;

    use base64::encode;

    #[test]
    fn extract_certs_from_tsl() {
        let xml = read_to_string("./examples/TSL.xml").unwrap();
        let actual = extract(&xml)
            .unwrap()
            .iter()
            .map(|cert| cert.to_der().unwrap())
            .map(|cert| encode(&cert))
            .collect::<Vec<_>>();

        let expected = vec!["MIIEbDCCA1SgAwIBAgIBATANBgkqhkiG9w0BAQsFADCBozELMAkGA1UEBhMCREUxKDAmBgNVBAoMH0JJVE1BUkNLIFRlY2huaWsgR21iSCBOT1QtVkFMSUQxRTBDBgNVBAsMPEVsZWt0cm9uaXNjaGUgR2VzdW5kaGVpdHNrYXJ0ZS1DQSBkZXIgVGVsZW1hdGlraW5mcmFzdHJ1a3R1cjEjMCEGA1UEAwwaQklUTUFSQ0suRUdLLUNBNiBURVNULU9OTFkwHhcNMTQwODA0MTQ1MjIwWhcNMjIwODAyMTQ1MjIwWjCBozELMAkGA1UEBhMCREUxKDAmBgNVBAoMH0JJVE1BUkNLIFRlY2huaWsgR21iSCBOT1QtVkFMSUQxRTBDBgNVBAsMPEVsZWt0cm9uaXNjaGUgR2VzdW5kaGVpdHNrYXJ0ZS1DQSBkZXIgVGVsZW1hdGlraW5mcmFzdHJ1a3R1cjEjMCEGA1UEAwwaQklUTUFSQ0suRUdLLUNBNiBURVNULU9OTFkwggEiMA0GCSqGSIb3DQEBAQUAA4IBDwAwggEKAoIBAQDVK5qcfz6G+aNLkaRS3KIKd6L2UngDTQkxceHq6OoF7wkGHtYE90r3DvL3YLRn74Y31jpLT6cXiENOpywwoWAXXRa7JrtSPSVMsHbbNAEw5alFmI6HkNKjUKuRhbzUECNaHNfsPVNt8j0UbzrTzLn6jip8kt+YSjO1Ms7qGliztHzYHjsaKaGUHnxlSN1NJHUDYo67Y0p/NLEy8cxCI6Yv1uaVzxt8EmM4kHaqe+4+ZAj2fHj3b8VNevc4R2snsOhu9aizm1QIZOGb5DdjarYCAf5RHVbPNVgT8ERe1JMIfSEffAEUJyUoaqrv/kU4xRQ8xI2Ee7BkMJqWr1crFlmzAgMBAAGjgagwgaUwHQYDVR0OBBYEFEIPA1P4Zfekc0iqNEoqXEhQ4Zq9MEkGCCsGAQUFBwEBBD0wOzA5BggrBgEFBQcwAYYtaHR0cDovL29jc3AudGVzdC1lZ2stcGtpMS5iaXRtYXJjay5kZS9lZ2stY2E2MBIGA1UdEwEB/wQIMAYBAf8CAQAwDgYDVR0PAQH/BAQDAgEGMBUGA1UdIAQOMAwwCgYIKoIUAEwEgSMwDQYJKoZIhvcNAQELBQADggEBABSAlaL2B3bHHyhW6EHU7tmNZIDYcTr78jgEwmBYw/FtBcVd36XCTDPQ+fxjqaZPHxMnlDW0xm8MeLOLc/Mi9hLeKZWe/2FCRXsC1mu4dDiuN2FEYl4KuvVhP55YSi98OKWNgtqUHGohRL9lZaL+YbHMUM9LbAaDcjYwn7rVywhw1Hw27qXofhEG+a3HIFCGYUfSPik45pl7w/d1EjMxrKzj+9eHAaR1hK8QAtpbqDQVJ4aJQ9C3fyHCE/KpWzMaw5eUGGubiA6k3dkPUsXIMs/M2tAC9i4BaQYMgm+Gh9G1/NWBRUT6ppqPzs4CTi9YncWcRa6nqlIwO8MOeDZYd5M="];

        assert_eq!(actual, expected);
    }
}
