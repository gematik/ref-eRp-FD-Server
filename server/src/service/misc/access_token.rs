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

use chrono::{DateTime, Utc};
use openssl::pkey::{PKey, Public};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use miscellaneous::jwt::{verify, Error as JwtError, VerifyMode};
use resources::{
    audit_event::{Agent, ParticipationRoleType},
    misc::{Kvnr, ParticipantId, TelematikId},
};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessToken {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub nonce: Option<String>,

    #[serde(with = "from_timtstamp")]
    pub exp: DateTime<Utc>,

    #[serde(with = "from_timtstamp")]
    pub iat: DateTime<Utc>,

    #[serde(default, with = "from_timtstamp_opt")]
    pub nbf: Option<DateTime<Utc>>,

    #[serde(rename = "professionOID")]
    pub profession: Profession,

    #[serde(rename = "idNummer")]
    pub id_number: String,

    #[serde(rename = "given_name")]
    pub given_name: Option<String>,

    #[serde(rename = "family_name")]
    pub family_name: Option<String>,

    #[serde(rename = "organizationName")]
    pub organization_name: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Eq, PartialEq)]
pub enum Profession {
    #[serde(rename = "1.2.276.0.76.4.49")]
    Versicherter,

    #[serde(rename = "1.2.276.0.76.4.50")]
    PraxisArzt,

    #[serde(rename = "1.2.276.0.76.4.51")]
    ZahnarztPraxis,

    #[serde(rename = "1.2.276.0.76.4.52")]
    PraxisPsychotherapeut,

    #[serde(rename = "1.2.276.0.76.4.53")]
    Krankenhaus,

    #[serde(rename = "1.2.276.0.76.4.54")]
    OeffentlicheApotheke,

    #[serde(rename = "1.2.276.0.76.4.55")]
    KrankenhausApotheke,

    #[serde(other)]
    Unknown,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("JWT Error: {0}")]
    JwtError(JwtError),

    #[error("Authorization header is missing!")]
    Missing,

    #[error("PUK_TOKEN was not fetched yet!")]
    NoPukToken,

    #[error("Authorization header has invalid value!")]
    InvalidValue,

    #[error("Invalid profession!")]
    InvalidProfession,

    #[error("Expired!")]
    Expired,

    #[error("Not valid yet!")]
    NotValidYet,

    #[error("Access Token does not contain a valid KV-Nr.!")]
    NoKvnr,

    #[error("Access Token does not contain a valid Telematik ID!")]
    NoTelematikId,

    #[error("Access Token from VAU request and inner HTTP request does not match!")]
    Mismatch,
}

impl AccessToken {
    pub fn verify(
        access_token: &str,
        key: PKey<Public>,
        now: DateTime<Utc>,
    ) -> Result<Self, Error> {
        let access_token = verify::<Self>(access_token, VerifyMode::KeyIn(key))?;
        let nbf = access_token.nbf.unwrap_or(access_token.iat);

        if now > access_token.exp {
            return Err(Error::Expired);
        } else if nbf > now {
            return Err(Error::NotValidYet);
        }

        Ok(access_token)
    }

    pub fn id(&self) -> Result<ParticipantId, Error> {
        match self.profession {
            Profession::Versicherter => {
                let kvnr = Kvnr::new(self.id_number.clone()).map_err(|_| Error::NoKvnr)?;

                Ok(ParticipantId::Kvnr(kvnr))
            }
            _ => {
                let telematik_id = TelematikId::new(self.id_number.clone());

                Ok(ParticipantId::TelematikId(telematik_id))
            }
        }
    }

    pub fn kvnr(&self) -> Result<Kvnr, Error> {
        match self.profession {
            Profession::Versicherter => {
                Ok(Kvnr::new(self.id_number.clone()).map_err(|_| Error::NoKvnr)?)
            }
            _ => Err(Error::NoKvnr),
        }
    }

    pub fn telematik_id(&self) -> Result<TelematikId, Error> {
        match self.profession {
            Profession::Versicherter => Err(Error::NoTelematikId),
            _ => Ok(TelematikId::new(self.id_number.clone())),
        }
    }

    pub fn check_profession<F>(&self, f: F) -> Result<(), Error>
    where
        F: FnOnce(Profession) -> bool,
    {
        if f(self.profession) {
            Ok(())
        } else {
            Err(Error::InvalidProfession)
        }
    }

    pub fn is_patient(&self) -> bool {
        self.profession == Profession::Versicherter
    }

    pub fn is_pharmacy(&self) -> bool {
        self.profession == Profession::OeffentlicheApotheke
            || self.profession == Profession::KrankenhausApotheke
    }
}

impl From<&AccessToken> for Agent {
    fn from(v: &AccessToken) -> Self {
        let mut name = String::default();

        if let Some(v) = &v.given_name {
            name = format!("{}{} ", name, v);
        }

        if let Some(v) = &v.family_name {
            name = format!("{}{} ", name, v);
        }

        if let Some(v) = &v.organization_name {
            name = format!("{}{} ", name, v);
        }

        name = name.trim_end().to_owned();

        Self {
            type_: ParticipationRoleType::HumanUser,
            who: v.id().ok(),
            name,
            requestor: false,
        }
    }
}

impl From<JwtError> for Error {
    fn from(err: JwtError) -> Self {
        Self::JwtError(err)
    }
}

mod from_timtstamp {
    use chrono::{naive::NaiveDateTime, DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(
        value: &DateTime<Utc>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Helper(i64);

        Helper(value.timestamp()).serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<DateTime<Utc>, D::Error> {
        #[derive(Deserialize)]
        struct Helper(i64);

        let timestamp = Helper::deserialize(deserializer)?;
        let timestamp = NaiveDateTime::from_timestamp(timestamp.0, 0);
        let timestamp = DateTime::from_utc(timestamp, Utc);

        Ok(timestamp)
    }
}

mod from_timtstamp_opt {
    use chrono::{naive::NaiveDateTime, DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(
        value: &Option<DateTime<Utc>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        value
            .as_ref()
            .map(DateTime::timestamp)
            .serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<DateTime<Utc>>, D::Error> {
        Ok(Option::<i64>::deserialize(deserializer)?
            .map(|t| DateTime::from_utc(NaiveDateTime::from_timestamp(t, 0), Utc)))
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use openssl::x509::X509;

    #[test]
    fn verify() {
        let cert = r##"
-----BEGIN CERTIFICATE-----
MIIC+TCCAqCgAwIBAgIGQ2mWV7L/MAoGCCqGSM49BAMCMIGWMQswCQYDVQQGEwJE
RTEfMB0GA1UECgwWZ2VtYXRpayBHbWJIIE5PVC1WQUxJRDFFMEMGA1UECww8RWxl
a3Ryb25pc2NoZSBHZXN1bmRoZWl0c2thcnRlLUNBIGRlciBUZWxlbWF0aWtpbmZy
YXN0cnVrdHVyMR8wHQYDVQQDDBZHRU0uRUdLLUNBMTAgVEVTVC1PTkxZMB4XDTE5
MDUwNTIyMDAwMFoXDTI0MDUwNTIxNTk1OVowfTELMAkGA1UEBhMCREUxETAPBgNV
BAoMCEFPSyBQbHVzMRIwEAYDVQQLDAkxMDk1MDA5NjkxEzARBgNVBAsMClgxMTQ0
Mjg1MzAxDjAMBgNVBAQMBUZ1Y2hzMQ0wCwYDVQQqDARKdW5hMRMwEQYDVQQDDApK
dW5hIEZ1Y2hzMFowFAYHKoZIzj0CAQYJKyQDAwIIAQEHA0IABHXNcEP/nIswh2yt
iIbp7ac2ra9nJPaaMsdVGt+TCFQOnjLZrQbGwH8AHMr4d18UYoHSFYQunen2dCIo
w3d7MgKjgfAwge0wIQYDVR0gBBowGDAKBggqghQATASBVDAKBggqghQATASBIzA4
BggrBgEFBQcBAQQsMCowKAYIKwYBBQUHMAGGHGh0dHA6Ly9laGNhLmdlbWF0aWsu
ZGUvb2NzcC8wHQYDVR0OBBYEFKreQaZyz1VjlRbEXW4kftivxwRmMDAGBSskCAMD
BCcwJTAjMCEwHzAdMBAMDlZlcnNpY2hlcnRlLy1yMAkGByqCFABMBDEwDAYDVR0T
AQH/BAIwADAOBgNVHQ8BAf8EBAMCB4AwHwYDVR0jBBgwFoAURLFMAVhUHtzZN77k
sj8qbqRciR0wCgYIKoZIzj0EAwIDRwAwRAIgGJrZ8jQKSQST5SSl7O8uN9vLoI/n
bTruoO+7I/dqnloCIAtzL2Vk1W3dHT+3Z5Qiaa3vWnAuaBELd6wj9oY9W5aA
-----END CERTIFICATE-----"##;
        let cert = X509::from_pem(cert.as_bytes()).unwrap();

        let pub_key = cert.public_key().unwrap();

        let now = DateTime::parse_from_rfc3339("2020-10-20T12:39:59Z").unwrap();
        let access_token = r##"eyJhbGciOiJCUDI1NlIxIn0.eyJzdWIiOiJzdWJqZWN0Iiwib3JnYW5pemF0aW9uTmFtZSI6ImdlbWF0aWsgR21iSCBOT1QtVkFMSUQiLCJwcm9mZXNzaW9uT0lEIjoiMS4yLjI3Ni4wLjc2LjQuNDkiLCJpZE51bW1lciI6IlgxMTQ0Mjg1MzAiLCJpc3MiOiJzZW5kZXIiLCJyZXNwb25zZV90eXBlIjoiY29kZSIsImNvZGVfY2hhbGxlbmdlX21ldGhvZCI6IlMyNTYiLCJnaXZlbl9uYW1lIjoiSnVuYSIsImNsaWVudF9pZCI6bnVsbCwiYXVkIjoiZXJwLnplbnRyYWwuZXJwLnRpLWRpZW5zdGUuZGUiLCJhY3IiOiJlaWRhcy1sb2EtaGlnaCIsInNjb3BlIjoib3BlbmlkIGUtcmV6ZXB0Iiwic3RhdGUiOiJhZjBpZmpzbGRraiIsInJlZGlyZWN0X3VyaSI6bnVsbCwiZXhwIjoxNjAzMTk3NjUyLCJmYW1pbHlfbmFtZSI6IkZ1Y2hzIiwiY29kZV9jaGFsbGVuZ2UiOm51bGwsImlhdCI6MTYwMzE5NzM1MiwiYXV0aF90aW1lIjoxNjAzMTk3MzUyfQ.XqPmrlF-6elvj6sAU0mH2GmBoggef-RYpTdJ3Ae9KiB3n7yvc3W27wH9hcTm4gSbdddNZ1_oZfP_Rc-U2Jb9Sg"##;
        AccessToken::verify(access_token, pub_key, now.into()).unwrap();
    }
}
