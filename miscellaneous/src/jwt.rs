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

use std::iter::{empty, once};

use base64::{encode, encode_config, URL_SAFE_NO_PAD};
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

pub fn sign<H: ToBase64>(
    key: PKey<Private>,
    cert: Option<&X509>,
    header: Option<H>,
    data: &[u8],
    detached: bool,
) -> Result<String, Error> {
    let key = PKeyWithDigest {
        digest: MessageDigest::sha256(),
        key,
    };

    let default_header = Header {
        alg: Algorithm::BP256R1,
        x5c: cert
            .into_iter()
            .map(cert_to_str)
            .collect::<Result<Vec<_>, _>>()?,
    };

    let header = header
        .as_ref()
        .map(|h| h.to_base64())
        .unwrap_or_else(|| default_header.to_base64())?;

    let claims = encode_config(&data, URL_SAFE_NO_PAD);
    let signature = key.sign(&header, &claims)?;

    let claims = if detached { "" } else { &*claims };

    let jwt = [&*header, claims, &signature].join(".");

    Ok(jwt)
}

pub enum VerifyMode<'a> {
    None,
    KeyIn(PKey<Public>),
    CertOut(&'a mut Option<X509>),
}

struct VerifyParts<'a> {
    keys: BoxedKeyIter<'a>,
    cert_out: Option<&'a mut Option<X509>>,
    is_verified: bool,
}

type BoxedKeyIter<'a> = Box<dyn Iterator<Item = Result<(PKey<Public>, Option<X509>), Error>>>;

impl<'a> VerifyMode<'a> {
    fn into_parts(self, x5c: Vec<String>) -> VerifyParts<'a> {
        match self {
            VerifyMode::None => VerifyParts {
                keys: Box::new(empty()),
                cert_out: None,
                is_verified: true,
            },
            VerifyMode::KeyIn(key) => VerifyParts {
                keys: Box::new(once(Ok((key, None)))),
                cert_out: None,
                is_verified: false,
            },
            VerifyMode::CertOut(cert_out) => VerifyParts {
                keys: Box::new(x5c.into_iter().map(cert_to_key)),
                cert_out: Some(cert_out),
                is_verified: false,
            },
        }
    }
}

pub fn verify<T>(jwt: &str, mode: VerifyMode) -> Result<T, Error>
where
    T: FromBase64,
{
    let (header_str, claims_str, signature_str) = split(jwt)?;

    let header = Header::from_base64(header_str)?;
    let claims = T::from_base64(claims_str);

    let VerifyParts {
        keys,
        cert_out,
        mut is_verified,
    } = mode.into_parts(header.x5c);

    for key_and_cert in keys {
        match header.alg {
            Algorithm::BP256R1 => {
                let (key, cert) = key_and_cert?;
                let key = PKeyWithDigest {
                    digest: MessageDigest::sha256(),
                    key,
                };

                if key.verify(header_str, claims_str, signature_str)? {
                    is_verified = true;

                    if let (Some(cert_out), Some(cert)) = (cert_out, cert) {
                        *cert_out = Some(cert);
                    }

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

fn split(jwt: &str) -> Result<(&str, &str, &str), Error> {
    let mut access_token = jwt.split('.');

    let header_str = access_token.next().ok_or(Error::NoHeaderComponent)?;
    let claims_str = access_token.next().ok_or(Error::NoClaimsComponent)?;
    let signature_str = access_token.next().ok_or(Error::NoSignatureComponent)?;

    if access_token.next().is_some() {
        return Err(Error::TooManyComponents);
    }

    Ok((header_str, claims_str, signature_str))
}

fn cert_to_key(cert: String) -> Result<(PKey<Public>, Option<X509>), Error> {
    let cert = format!(
        "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----",
        cert
    );
    let cert = X509::from_pem(cert.as_bytes())?;
    let key = cert.public_key()?;

    Ok((key, Some(cert)))
}

fn cert_to_str(cert: &X509) -> Result<String, Error> {
    let bytes = cert.to_der()?;
    let ret = encode(&bytes);

    Ok(ret)
}
