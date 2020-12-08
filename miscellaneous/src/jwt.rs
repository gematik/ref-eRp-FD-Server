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

use base64::encode;
use jwt::{FromBase64, PKeyWithDigest, SigningAlgorithm, ToBase64, VerifyingAlgorithm};
use openssl::{
    hash::MessageDigest,
    pkey::{PKey, Private, Public},
    x509::X509,
};
use serde::{Deserialize, Serialize};

pub use jwt::Error;

#[derive(Serialize, Deserialize)]
struct Header {
    alg: Algorithm,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    x5c: Vec<String>,
}

#[derive(Serialize, Deserialize)]
enum Algorithm {
    BP256R1,
}

pub fn sign<T: ToBase64>(
    claims: &T,
    key: PKey<Private>,
    cert: Option<&X509>,
    detached: bool,
) -> Result<String, Error> {
    let key = PKeyWithDigest {
        digest: MessageDigest::sha256(),
        key,
    };

    let header = Header {
        alg: Algorithm::BP256R1,
        x5c: cert
            .into_iter()
            .map(cert_to_str)
            .collect::<Result<Vec<_>, _>>()?,
    };

    let header = header.to_base64()?;
    let claims = claims.to_base64()?;
    let signature = key.sign(&header, &claims)?;

    let claims = if detached { "" } else { &*claims };

    let jwt = [&*header, claims, &signature].join(".");

    Ok(jwt)
}

pub fn verify<T>(jwt: &str, key: Option<PKey<Public>>) -> Result<T, Error>
where
    T: FromBase64,
{
    let mut access_token = jwt.split('.');
    let header_str = access_token.next().ok_or(Error::NoHeaderComponent)?;
    let claims_str = access_token.next().ok_or(Error::NoClaimsComponent)?;
    let signature_str = access_token.next().ok_or(Error::NoSignatureComponent)?;

    if access_token.next().is_some() {
        return Err(Error::TooManyComponents);
    }

    let header = Header::from_base64(header_str)?;
    let claims = T::from_base64(claims_str);

    let keys = header.x5c.into_iter().map(cert_to_key).chain(key.map(Ok));

    let mut is_verified = false;
    for key in keys {
        match header.alg {
            Algorithm::BP256R1 => {
                let key = PKeyWithDigest {
                    digest: MessageDigest::sha256(),
                    key: key?,
                };

                if key.verify(header_str, claims_str, signature_str)? {
                    is_verified = true;

                    break;
                }
            }
        }
    }

    if !is_verified {
        return Err(Error::InvalidSignature);
    }

    claims
}

fn cert_to_key(cert: String) -> Result<PKey<Public>, Error> {
    let cert = format!(
        "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----",
        cert
    );
    let cert = X509::from_pem(cert.as_bytes())?;

    let pub_key = cert.public_key()?;

    Ok(pub_key)
}

fn cert_to_str(cert: &X509) -> Result<String, Error> {
    let bytes = cert.to_der()?;
    let ret = encode(&bytes);

    Ok(ret)
}
