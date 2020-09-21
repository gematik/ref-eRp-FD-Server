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
use jwt::{Error as JwtError, PKeyWithDigest, VerifyWithKey};
use openssl::{
    hash::MessageDigest,
    pkey::{PKey, Public},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use resources::misc::{Kvnr, TelematikId};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AccessToken {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub acr: String,
    pub nonce: String,

    #[serde(with = "from_timtstamp")]
    pub exp: DateTime<Utc>,
    #[serde(with = "from_timtstamp")]
    pub iat: DateTime<Utc>,
    #[serde(with = "from_timtstamp_opt")]
    pub nbf: Option<DateTime<Utc>>,

    #[serde(rename = "professionOID")]
    pub profession: Profession,
    #[serde(rename = "given_name")]
    pub given_name: String,
    #[serde(rename = "family_name")]
    pub family_name: String,
    pub organization_name: String,
    #[serde(rename = "idNummer")]
    pub id_number: String,
    pub jti: String,
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

    #[serde(rename = "1.2.276.0.76.4.54")]
    OeffentlicheApotheke,

    #[serde(rename = "1.2.276.0.76.4.55")]
    KrankenhausApotheke,

    #[serde(rename = "1.2.276.0.76.4.178")]
    Notfallsanitaeter,

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
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("JWT Error: {0}")]
    JwtError(JwtError),

    #[error("Authorization header is missing!")]
    Missing,

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

impl AccessToken {
    pub fn verify(
        access_token: &str,
        key: PKey<Public>,
        now: DateTime<Utc>,
    ) -> Result<Self, Error> {
        let key = PKeyWithDigest {
            digest: MessageDigest::sha256(),
            key,
        };

        let access_token: Self = access_token.verify_with_key(&key)?;
        let nbf = access_token.nbf.unwrap_or(access_token.iat);

        if now > access_token.exp {
            return Err(Error::Expired);
        } else if nbf > now {
            return Err(Error::NotValidYet);
        }

        if access_token.acr != "1" {
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

    use base64::decode;

    #[test]
    fn verify() {
        let pub_key = "MHYwEAYHKoZIzj0CAQYFK4EEACIDYgAEVDUmq9/Ec5Sj8mRbDUhlGp86TUbYdAvjIpFRB/BQJQxzDKQLN+HcheCCtLsYG4hHvW0Poni65escBUdMmk4r7sKMlwvknBlJ8J6Wl5onelFIMOMqW53h7GirmfSS3TAK";
        let pub_key = decode(&pub_key).unwrap();
        let pub_key = PKey::public_key_from_der(&pub_key).unwrap();

        let now = DateTime::parse_from_rfc3339("2020-03-27T19:25:00Z").unwrap();
        let access_token = r##"eyJhbGciOiJFUzI1NiIsInR5cCI6IkpXVCJ9.eyJhY3IiOiIxIiwiYXVkIjoiaHR0cHM6Ly9lcnAudGVsZW1hdGlrLmRlL2xvZ2luIiwiZXhwIjoxNTg1MzM3MjU2LCJmYW1pbHlfbmFtZSI6ImRlciBOYWNobmFtZSIsImdpdmVuX25hbWUiOiJkZXIgVm9ybmFtZSIsImlhdCI6MTU4NTMzNjk1NiwiaWROdW1tZXIiOiIzLTE1LjEuMS4xMjM0NTY3ODkiLCJpc3MiOiJodHRwczovL2lkcDEudGVsZW1hdGlrLmRlL2p3dCIsImp0aSI6IjxJRFA-XzAxMjM0NTY3ODkwMTIzNDU2Nzg5IiwibmJmIjoxNTg1MzM2OTU2LCJub25jZSI6ImZ1dSBiYXIgYmF6Iiwib3JnYW5pemF0aW9uTmFtZSI6Ikluc3RpdHV0aW9ucy0gb2RlciBPcmdhbmlzYXRpb25zLUJlemVpY2hudW5nIiwicHJvZmVzc2lvbk9JRCI6IjEuMi4yNzYuMC43Ni40LjQ5Iiwic3ViIjoiUmFiY1VTdXVXS0taRUVIbXJjTm1fa1VET1cxM3VhR1U1Wms4T29Cd2lOayJ9.e244BReFrmlY86dLWi3wAFRWnIy764BAuLDIR7Lj5qjBwuZFq9IJ9YBkLl-1alAxDjh4Td8BeP5pEtFMTVwPh20roTOvz8byjS7_ugtESG7QLrEtyZso7W6zB4aJgSc3"##;
        AccessToken::verify(access_token, pub_key, now.into()).unwrap();
    }
}
