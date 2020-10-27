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

use chrono::{DateTime, Utc};
use jwt::{Error as JwtError, FromBase64, PKeyWithDigest, VerifyingAlgorithm};
use openssl::{
    hash::MessageDigest,
    pkey::{PKey, Public},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use resources::misc::{Kvnr, TelematikId};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessToken {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub acr: String,
    pub nonce: Option<String>,

    #[serde(with = "from_timtstamp")]
    pub exp: DateTime<Utc>,

    #[serde(with = "from_timtstamp")]
    pub iat: DateTime<Utc>,

    #[serde(default, with = "from_timtstamp_opt")]
    pub nbf: Option<DateTime<Utc>>,

    #[serde(rename = "professionOID")]
    pub profession: Profession,

    #[serde(rename = "given_name")]
    pub given_name: String,

    #[serde(rename = "family_name")]
    pub family_name: String,

    #[serde(rename = "organizationName")]
    pub organization_name: String,

    #[serde(rename = "idNummer")]
    pub id_number: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Eq, PartialEq)]
pub enum Profession {
    #[serde(rename = "1.2.276.0.76.4.30")]
    Arzt,

    #[serde(rename = "1.2.276.0.76.4.31")]
    Zahnarzt,

    #[serde(rename = "1.2.276.0.76.4.32")]
    Apotheker,

    #[serde(rename = "1.2.276.0.76.4.33")]
    ApothekerAssistent,

    #[serde(rename = "1.2.276.0.76.4.34")]
    PharmazieIngenieur,

    #[serde(rename = "1.2.276.0.76.4.35")]
    PharmTechnAssistent,

    #[serde(rename = "1.2.276.0.76.4.36")]
    PharmKaufmAngestellter,

    #[serde(rename = "1.2.276.0.76.4.37")]
    ApothekenHelfer,

    #[serde(rename = "1.2.276.0.76.4.38")]
    ApothekenAssistent,

    #[serde(rename = "1.2.276.0.76.4.39")]
    PharmAssistent,

    #[serde(rename = "1.2.276.0.76.4.40")]
    ApothekenFacharbeiter,

    #[serde(rename = "1.2.276.0.76.4.41")]
    PharmaziePraktikant,

    #[serde(rename = "1.2.276.0.76.4.42")]
    Famulant,

    #[serde(rename = "1.2.276.0.76.4.43")]
    PtaPraktikant,

    #[serde(rename = "1.2.276.0.76.4.44")]
    PkaAuszubildender,

    #[serde(rename = "1.2.276.0.76.4.46")]
    Psychotherapeut,

    #[serde(rename = "1.2.276.0.76.4.47")]
    KujPsychotherapeut,

    #[serde(rename = "1.2.276.0.76.4.48")]
    Rettungsassistent,

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

    #[serde(rename = "1.2.276.0.76.4.56")]
    BundeswehrApotheke,

    #[serde(rename = "1.2.276.0.76.4.57")]
    MobileEinrichtungRettungsdienst,

    #[serde(rename = "1.2.276.0.76.4.58")]
    Gematik,

    #[serde(rename = "1.2.276.0.76.4.59")]
    Kostentraeger,

    #[serde(rename = "1.2.276.0.76.4.178")]
    Notfallsanitaeter,

    #[serde(rename = "1.2.276.0.76.4.190")]
    AdvKtr,

    #[serde(rename = "1.2.276.0.76.4.210")]
    LeoKassenaerztlicheVereinigung,

    #[serde(rename = "1.2.276.0.76.4.223")]
    GkvSpitzenverband,

    #[serde(rename = "1.2.276.0.76.4.224")]
    LeoApothekerverband,

    #[serde(rename = "1.2.276.0.76.4.225")]
    LeoDav,

    #[serde(rename = "1.2.276.0.76.4.226")]
    LeoKrankenhausverband,

    #[serde(rename = "1.2.276.0.76.4.227")]
    LeoDktig,

    #[serde(rename = "1.2.276.0.76.4.228")]
    LeoDkg,

    #[serde(rename = "1.2.276.0.76.4.229")]
    LeoBaek,

    #[serde(rename = "1.2.276.0.76.4.230")]
    LeoAerztekammer,

    #[serde(rename = "1.2.276.0.76.4.231")]
    LeoZahnaerztekammer,

    #[serde(rename = "1.2.276.0.76.4.232")]
    PflegerHpc,

    #[serde(rename = "1.2.276.0.76.4.233")]
    AltenpflegerHpc,

    #[serde(rename = "1.2.276.0.76.4.234")]
    PflegefachkraftHpc,

    #[serde(rename = "1.2.276.0.76.4.235")]
    HebammeHpc,

    #[serde(rename = "1.2.276.0.76.4.236")]
    PhysiotherapeutHpc,

    #[serde(rename = "1.2.276.0.76.4.237")]
    AugenoptikerHpc,

    #[serde(rename = "1.2.276.0.76.4.238")]
    HoerakustikerHpc,

    #[serde(rename = "1.2.276.0.76.4.239")]
    OrthopaedieSchuhmacherHpc,

    #[serde(rename = "1.2.276.0.76.4.240")]
    OrthopaedieTechnikerHpc,

    #[serde(rename = "1.2.276.0.76.4.241")]
    ZahnTechnikerHpc,

    #[serde(rename = "1.2.276.0.76.4.242")]
    LeoKbv,

    #[serde(rename = "1.2.276.0.76.4.243")]
    LeoBzaek,

    #[serde(rename = "1.2.276.0.76.4.244")]
    LeoKzbv,

    #[serde(rename = "1.2.276.0.76.4.245")]
    InstitutionPflege,

    #[serde(rename = "1.2.276.0.76.4.246")]
    InstitutionGeburtshilfe,

    #[serde(rename = "1.2.276.0.76.4.247")]
    PraxisPhysiotherapeut,

    #[serde(rename = "1.2.276.0.76.4.248")]
    InstitutionAugenoptiker,

    #[serde(rename = "1.2.276.0.76.4.249")]
    InstitutionHoerakustiker,

    #[serde(rename = "1.2.276.0.76.4.250")]
    InstitutionOrthopaedieschuhmacher,

    #[serde(rename = "1.2.276.0.76.4.251")]
    InstitutionOrthopaedietechniker,

    #[serde(rename = "1.2.276.0.76.4.252")]
    InstitutionZahntechniker,

    #[serde(rename = "1.2.276.0.76.4.253")]
    InstitutionRettungsleitstellen,

    #[serde(rename = "1.2.276.0.76.4.254")]
    SanitaetsdienstBundeswehr,

    #[serde(rename = "1.2.276.0.76.4.255")]
    InstitutionOegd,

    #[serde(rename = "1.2.276.0.76.4.256")]
    InstitutionArbeitsmedizin,

    #[serde(rename = "1.2.276.0.76.4.257")]
    InstitutionVorsorgeReha,

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

    #[error("Invalid authentication level!")]
    InvalidAcr,

    #[error("Access Token does not contain a valid KV-Nr.!")]
    NoKvnr,

    #[error("Access Token does not contain a valid Telematik ID!")]
    NoTelematikId,
}

#[derive(Deserialize)]
struct Header {
    alg: Algorithm,
}

#[derive(Deserialize)]
enum Algorithm {
    BP256R1,
}

impl AccessToken {
    pub fn verify(
        access_token: &str,
        key: PKey<Public>,
        now: DateTime<Utc>,
    ) -> Result<Self, Error> {
        let mut access_token = access_token.split('.');
        let header_str = access_token.next().ok_or(JwtError::NoHeaderComponent)?;
        let claims_str = access_token.next().ok_or(JwtError::NoClaimsComponent)?;
        let signature_str = access_token.next().ok_or(JwtError::NoSignatureComponent)?;

        if access_token.next().is_some() {
            return Err(JwtError::TooManyComponents.into());
        }

        let header = Header::from_base64(header_str)?;

        match header.alg {
            Algorithm::BP256R1 => {
                let key = PKeyWithDigest {
                    digest: MessageDigest::sha256(),
                    key,
                };

                if !key.verify(header_str, claims_str, signature_str)? {
                    return Err(JwtError::InvalidSignature.into());
                }
            }
        }

        let access_token = AccessToken::from_base64(claims_str)?;
        let nbf = access_token.nbf.unwrap_or(access_token.iat);

        if now > access_token.exp {
            return Err(Error::Expired);
        } else if nbf > now {
            return Err(Error::NotValidYet);
        }

        if access_token.acr != "eidas-loa-high" {
            return Err(Error::InvalidAcr);
        }

        Ok(access_token)
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
