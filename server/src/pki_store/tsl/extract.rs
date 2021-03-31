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

use std::collections::HashMap;

use base64::decode;
use openssl::x509::X509;
use quick_xml::de::from_str;
use serde::{Deserialize, Serialize};

use super::{super::Error, Item, Tsl};

pub fn extract<F>(xml: &str, prepare: &F) -> Result<HashMap<String, Vec<Item>>, Error>
where
    F: Fn(&mut TrustServiceStatusList) -> Result<(), Error> + Send + Sync,
{
    let mut tsl: TrustServiceStatusList = from_str(xml)?;

    prepare(&mut tsl)?;

    let mut certs: HashMap<String, Vec<Item>> = Default::default();
    for provider in tsl.provider_list.provider {
        for service in provider.services.service {
            let info = service.infos;
            let supply_points = info.supply_points.unwrap_or_default().supply_point;

            for id in &info.identity.id {
                if let Some(cert) = &id.cert {
                    let cert = cert.trim();
                    let cert = decode(&cert)?;
                    let cert = X509::from_der(&cert)?;

                    let key = Tsl::cert_key(cert.subject_name())?;

                    let item = Item {
                        cert,
                        supply_points: supply_points.clone(),
                    };

                    certs.entry(key).or_default().push(item);
                }
            }
        }
    }

    Ok(certs)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrustServiceStatusList {
    #[serde(rename = "TrustServiceProviderList")]
    pub provider_list: TrustServiceProviderList,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrustServiceProviderList {
    #[serde(rename = "TrustServiceProvider")]
    pub provider: Vec<TrustServiceProvider>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrustServiceProvider {
    #[serde(rename = "TSPServices")]
    pub services: TSPServices,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TSPServices {
    #[serde(rename = "TSPService")]
    pub service: Vec<TSPService>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TSPService {
    #[serde(rename = "ServiceInformation")]
    pub infos: ServiceInformation,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceInformation {
    #[serde(rename = "ServiceTypeIdentifier")]
    pub ident: String,

    #[serde(rename = "ServiceStatus")]
    pub status: String,

    #[serde(rename = "StatusStartingTime")]
    pub starting_time: String,

    #[serde(rename = "ServiceInformationExtensions")]
    pub extensions: Option<ServiceInformationExtensions>,

    #[serde(rename = "ServiceDigitalIdentity")]
    pub identity: ServiceDigitalIdentity,

    #[serde(rename = "ServiceSupplyPoints")]
    pub supply_points: Option<ServiceSupplyPoints>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceInformationExtensions {
    #[serde(rename = "Extension")]
    pub extension: Vec<Extension>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ServiceSupplyPoints {
    #[serde(rename = "ServiceSupplyPoint")]
    pub supply_point: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Extension {
    #[serde(rename = "ExtensionOID")]
    pub oid: Option<String>,

    #[serde(rename = "ExtensionValue")]
    pub value: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceDigitalIdentity {
    #[serde(rename = "DigitalId")]
    pub id: Vec<DigitalId>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DigitalId {
    #[serde(rename = "X509Certificate")]
    pub cert: Option<String>,
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::fs::read_to_string;

    use base64::encode;

    #[test]
    fn extract_certs_from_tsl() {
        let xml = read_to_string("./examples/TSL.xml").unwrap();
        let actual = extract(&xml, &|_| Ok(()))
            .unwrap()
            .values()
            .flatten()
            .map(|item| {
                let der = item.cert.to_der().unwrap();

                encode(&der)
            })
            .collect::<Vec<_>>();

        let expected = vec!["MIIEbDCCA1SgAwIBAgIBATANBgkqhkiG9w0BAQsFADCBozELMAkGA1UEBhMCREUxKDAmBgNVBAoMH0JJVE1BUkNLIFRlY2huaWsgR21iSCBOT1QtVkFMSUQxRTBDBgNVBAsMPEVsZWt0cm9uaXNjaGUgR2VzdW5kaGVpdHNrYXJ0ZS1DQSBkZXIgVGVsZW1hdGlraW5mcmFzdHJ1a3R1cjEjMCEGA1UEAwwaQklUTUFSQ0suRUdLLUNBNiBURVNULU9OTFkwHhcNMTQwODA0MTQ1MjIwWhcNMjIwODAyMTQ1MjIwWjCBozELMAkGA1UEBhMCREUxKDAmBgNVBAoMH0JJVE1BUkNLIFRlY2huaWsgR21iSCBOT1QtVkFMSUQxRTBDBgNVBAsMPEVsZWt0cm9uaXNjaGUgR2VzdW5kaGVpdHNrYXJ0ZS1DQSBkZXIgVGVsZW1hdGlraW5mcmFzdHJ1a3R1cjEjMCEGA1UEAwwaQklUTUFSQ0suRUdLLUNBNiBURVNULU9OTFkwggEiMA0GCSqGSIb3DQEBAQUAA4IBDwAwggEKAoIBAQDVK5qcfz6G+aNLkaRS3KIKd6L2UngDTQkxceHq6OoF7wkGHtYE90r3DvL3YLRn74Y31jpLT6cXiENOpywwoWAXXRa7JrtSPSVMsHbbNAEw5alFmI6HkNKjUKuRhbzUECNaHNfsPVNt8j0UbzrTzLn6jip8kt+YSjO1Ms7qGliztHzYHjsaKaGUHnxlSN1NJHUDYo67Y0p/NLEy8cxCI6Yv1uaVzxt8EmM4kHaqe+4+ZAj2fHj3b8VNevc4R2snsOhu9aizm1QIZOGb5DdjarYCAf5RHVbPNVgT8ERe1JMIfSEffAEUJyUoaqrv/kU4xRQ8xI2Ee7BkMJqWr1crFlmzAgMBAAGjgagwgaUwHQYDVR0OBBYEFEIPA1P4Zfekc0iqNEoqXEhQ4Zq9MEkGCCsGAQUFBwEBBD0wOzA5BggrBgEFBQcwAYYtaHR0cDovL29jc3AudGVzdC1lZ2stcGtpMS5iaXRtYXJjay5kZS9lZ2stY2E2MBIGA1UdEwEB/wQIMAYBAf8CAQAwDgYDVR0PAQH/BAQDAgEGMBUGA1UdIAQOMAwwCgYIKoIUAEwEgSMwDQYJKoZIhvcNAQELBQADggEBABSAlaL2B3bHHyhW6EHU7tmNZIDYcTr78jgEwmBYw/FtBcVd36XCTDPQ+fxjqaZPHxMnlDW0xm8MeLOLc/Mi9hLeKZWe/2FCRXsC1mu4dDiuN2FEYl4KuvVhP55YSi98OKWNgtqUHGohRL9lZaL+YbHMUM9LbAaDcjYwn7rVywhw1Hw27qXofhEG+a3HIFCGYUfSPik45pl7w/d1EjMxrKzj+9eHAaR1hK8QAtpbqDQVJ4aJQ9C3fyHCE/KpWzMaw5eUGGubiA6k3dkPUsXIMs/M2tAC9i4BaQYMgm+Gh9G1/NWBRUT6ppqPzs4CTi9YncWcRa6nqlIwO8MOeDZYd5M="];

        assert_eq!(actual, expected);
    }
}
