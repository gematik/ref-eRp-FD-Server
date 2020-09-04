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

use std::borrow::Cow;
use std::convert::TryInto;
use std::fmt::{Formatter, Result as FmtResult};

use chrono::{
    naive::{NaiveDate, NaiveDateTime, NaiveTime},
    DateTime, FixedOffset, Utc,
};
use resources::signed_data::{
    AlgorithmIdentifier, Certificate, CertificateList, Name, PublicKeyInfo, RevokedCertificate,
    SignedData, SignerIdentifier, SignerInfo, TbsCertList, TbsCertificate, Validity,
};
use serde::{
    de::{Error as DeError, Visitor},
    Deserialize, Deserializer,
};

pub struct SignedDataDef;

#[serde(rename = "SignedData")]
#[derive(Clone, Deserialize)]
pub struct SignedDataCow<'a>(#[serde(with = "SignedDataDef")] pub Cow<'a, SignedData>);

#[derive(Deserialize)]
#[serde(rename = "oid=1.2.840.113549.1.7.2")]
struct Pkcs7Helper {
    version: usize,

    digest_algorithms: Vec<AlgorithmIdentifierDef>,

    #[serde(rename = "oid=1.2.840.113549.1.7.1")]
    #[serde(deserialize_with = "deserialize_byte_buf")]
    content: Vec<u8>,

    #[serde(rename = "tag=0")]
    certificates: Option<CertificateDef>,

    #[serde(rename = "tag=1")]
    crls: Option<Vec<CertificateListDef>>,

    signer_infos: Vec<SignerInfosDef>,
}

#[derive(Deserialize)]
struct CertificateDef {
    tbs_certificate: TbsCertificateDef,

    signature_algorithm: AlgorithmIdentifierDef,

    #[serde(deserialize_with = "deserialize_byte_buf")]
    signature_value: Vec<u8>,
}

#[derive(Deserialize)]
struct TbsCertificateDef {
    #[allow(dead_code)]
    #[serde(rename = "tag=0")]
    version: Option<usize>,

    serial_number: usize,

    signature: AlgorithmIdentifierDef,

    issuer: NameDef,

    validity: ValidityDef,

    subject: NameDef,

    subject_public_key_info: SubjectPublicKeyInfoDef,

    #[serde(rename = "tag=1")]
    issuer_unique_id: Option<UniqueIdentifier>,

    #[serde(rename = "tag=2")]
    subject_unique_id: Option<UniqueIdentifier>,

    #[allow(dead_code)]
    #[serde(rename = "tag=3")]
    extensions: Option<()>,
}

#[derive(Deserialize)]
struct ValidityDef {
    #[serde(rename = "tag=UTCTime&tag=GeneralizedTime")]
    not_before: String,

    #[serde(rename = "tag=UTCTime&tag=GeneralizedTime")]
    not_after: String,
}

#[derive(Deserialize)]
struct SubjectPublicKeyInfoDef {
    algorithm: AlgorithmIdentifierDef,

    #[serde(deserialize_with = "deserialize_byte_buf")]
    subject_public_key: Vec<u8>,
}

struct UniqueIdentifier(Vec<u8>);

#[derive(Deserialize)]
struct CertificateListDef {
    tbs_cert_list: TbsCertListDef,

    signature_algorithm: AlgorithmIdentifierDef,

    #[serde(deserialize_with = "deserialize_byte_buf")]
    signature_value: Vec<u8>,
}

#[derive(Deserialize)]
struct TbsCertListDef {
    #[allow(dead_code)]
    version: Option<usize>,

    signature: AlgorithmIdentifierDef,

    issuer: NameDef,

    #[serde(rename = "tag=UTCTime&tag=GeneralizedTime")]
    this_update: String,

    #[serde(rename = "tag=UTCTime&tag=GeneralizedTime")]
    next_update: Option<String>,

    #[serde(rename = "tag=Sequence")]
    revoked_certificates: Option<Vec<RevokedCertificateDef>>,

    #[allow(dead_code)]
    #[serde(rename = "tag=0")]
    crl_extensions: Option<()>,
}

#[derive(Deserialize)]
struct RevokedCertificateDef {
    user_certificate: usize,

    #[serde(rename = "tag=UTCTime&tag=GeneralizedTime")]
    revocation_date: String,

    #[allow(dead_code)]
    #[serde(rename = "tag=0")]
    crl_entry_extensions: Option<()>,
}

#[derive(Deserialize)]
struct SignerInfosDef {
    #[allow(dead_code)]
    version: usize,

    sid: SignerIdentifierDef,

    digest_algorithm: AlgorithmIdentifierDef,

    #[allow(dead_code)]
    #[serde(rename = "tag=0")]
    signed_attrs: Option<()>,

    signature_algorithm: AlgorithmIdentifierDef,

    #[serde(deserialize_with = "deserialize_byte_buf")]
    signature: Vec<u8>,

    #[allow(dead_code)]
    #[serde(rename = "tag=1")]
    unsigned_attrs: Option<()>,
}

#[derive(Deserialize)]
#[serde(rename = "tag=Sequence")]
struct NameDef(Vec<RelativeDistinguishedNameDef>);

#[derive(Deserialize)]
#[serde(rename = "tag=Set")]
struct RelativeDistinguishedNameDef(Vec<AttributeTypeAndValueDef>);

#[derive(Deserialize)]
enum SignerIdentifierDef {
    IssuerAndSerialNumber {
        issuer: NameDef,
        serial_number: usize,
    },

    #[serde(alias = "SubjectKeyIdentifier")]
    #[serde(rename = "name=SubjectKeyIdentifier&tag=0")]
    SubjectKeyIdentifier(#[serde(deserialize_with = "deserialize_byte_buf")] Vec<u8>),
}

#[derive(Deserialize)]
enum AttributeTypeAndValueDef {
    Unknown(String),

    #[serde(alias = "CommonName")]
    #[serde(rename = "name=CommonName&oid=2.5.4.3")]
    CommonName(String),

    #[serde(alias = "Surname")]
    #[serde(rename = "name=Surname&oid=2.5.4.4")]
    Surname(String),

    #[serde(alias = "SerialNumber")]
    #[serde(rename = "name=SerialNumber&oid=2.5.4.5")]
    SerialNumber(String),

    #[serde(alias = "CountryName")]
    #[serde(rename = "name=CountryName&oid=2.5.4.6")]
    CountryName(String),

    #[serde(alias = "LocalityName")]
    #[serde(rename = "name=LocalityName&oid=2.5.4.7")]
    LocalityName(String),

    #[serde(alias = "StateOrProvinceName")]
    #[serde(rename = "name=StateOrProvinceName&oid=2.5.4.8")]
    StateOrProvinceName(String),

    #[serde(alias = "OrganizationName")]
    #[serde(rename = "name=OrganizationName&oid=2.5.4.10")]
    OrganizationName(String),

    #[serde(alias = "OrganizationalUnitName")]
    #[serde(rename = "name=OrganizationalUnitName&oid=2.5.4.11")]
    OrganizationalUnitName(String),

    #[serde(alias = "Title")]
    #[serde(rename = "name=Title&oid=2.5.4.12")]
    Title(String),

    #[serde(alias = "Name")]
    #[serde(rename = "name=Name&oid=2.5.4.41")]
    Name(String),

    #[serde(alias = "GivenName")]
    #[serde(rename = "name=GivenName&oid=2.5.4.42")]
    GivenName(String),

    #[serde(alias = "Initialis")]
    #[serde(rename = "name=Initialis&oid=2.5.4.43")]
    Initialis(String),

    #[serde(alias = "GenerationQualifier")]
    #[serde(rename = "name=GenerationQualifier&oid=2.5.4.44")]
    GenerationQualifier(String),

    #[serde(alias = "DnQualifier")]
    #[serde(rename = "name=DnQualifier&oid=2.5.4.46")]
    DnQualifier(String),

    #[serde(alias = "Pseudonym")]
    #[serde(rename = "name=Pseudonym&oid=2.5.4.65")]
    Pseudonym(String),

    #[serde(alias = "EMailAddress")]
    #[serde(rename = "name=EMailAddress&oid=1.2.840.113549.1.9.1")]
    EMailAddress(String),
}

#[derive(Deserialize)]
enum AlgorithmIdentifierDef {
    #[serde(alias = "Md2")]
    #[serde(rename = "name=Md2&oid=1.2.840.113549.2.2")]
    Md2,

    #[serde(alias = "Md5")]
    #[serde(rename = "name=Md5&oid=1.2.840.113549.2.5")]
    Md5,

    #[serde(alias = "Sha1")]
    #[serde(rename = "name=Sha1&oid=1.3.14.3.2.26")]
    Sha1,

    #[serde(alias = "Sha224")]
    #[serde(rename = "name=Sha224&oid=2.16.840.1.101.3.4.2.4")]
    Sha224,

    #[serde(alias = "Sha256")]
    #[serde(rename = "name=Sha256&oid=2.16.840.1.101.3.4.2.1")]
    Sha256,

    #[serde(alias = "Sha384")]
    #[serde(rename = "name=Sha384&oid=2.16.840.1.101.3.4.2.2")]
    Sha384,

    #[serde(alias = "Sha512")]
    #[serde(rename = "name=Sha512&oid=2.16.840.1.101.3.4.2.3")]
    Sha512,

    #[serde(alias = "Sha512_224")]
    #[serde(rename = "name=Sha512_224&oid=2.16.840.1.101.3.4.2.5")]
    Sha512_224,

    #[serde(alias = "Sha512_256")]
    #[serde(rename = "name=Sha512_256&oid=2.16.840.1.101.3.4.2.6")]
    Sha512_256,

    #[serde(alias = "Md2WithRSAEncryption")]
    #[serde(rename = "name=Md2WithRSAEncryption&oid=1.2.840.113549.1.1.2")]
    Md2WithRSAEncryption,

    #[serde(alias = "Md5WithRSAEncryption")]
    #[serde(rename = "name=Md5WithRSAEncryption&oid=1.2.840.113549.1.1.4")]
    Md5WithRSAEncryption,

    #[serde(alias = "Sha1WithRSAEncryption")]
    #[serde(rename = "name=Sha1WithRSAEncryption&oid=1.2.840.113549.1.1.5")]
    Sha1WithRSAEncryption,

    #[serde(alias = "Sha224WithRSAEncryption")]
    #[serde(rename = "name=Sha224WithRSAEncryption&oid=1.2.840.113549.1.1.14")]
    Sha224WithRSAEncryption,

    #[serde(alias = "Sha256WithRSAEncryption")]
    #[serde(rename = "name=Sha256WithRSAEncryption&oid=1.2.840.113549.1.1.11")]
    Sha256WithRSAEncryption,

    #[serde(alias = "Sha384WithRSAEncryption")]
    #[serde(rename = "name=Sha384WithRSAEncryption&oid=1.2.840.113549.1.1.12")]
    Sha384WithRSAEncryption,

    #[serde(alias = "Sha512WithRSAEncryption")]
    #[serde(rename = "name=Sha512WithRSAEncryption&oid=1.2.840.113549.1.1.13")]
    Sha512WithRSAEncryption,

    #[serde(alias = "Sha512_224WithRSAEncryption")]
    #[serde(rename = "name=Sha512_224WithRSAEncryption&oid=1.2.840.113549.1.1.15")]
    Sha512_224WithRSAEncryption,

    #[serde(alias = "Sha512_256WithRSAEncryption")]
    #[serde(rename = "name=Sha512_256WithRSAEncryption&oid=1.2.840.113549.1.1.16")]
    Sha512_256WithRSAEncryption,

    #[serde(alias = "RsaEncryption")]
    #[serde(rename = "name=RsaEncryption&oid=1.2.840.113549.1.1.1")]
    RsaEncryption,

    #[serde(alias = "RsaesOaep")]
    #[serde(rename = "name=RsaesOaep&oid=1.2.840.113549.1.1.7")]
    RsaesOaep(()),

    #[serde(alias = "RsassaPss")]
    #[serde(rename = "name=RsassaPss&oid=1.2.840.113549.1.1.10")]
    RsassaPss(()),
}

impl SignedDataCow<'_> {
    pub fn into_inner(self) -> SignedData {
        self.0.into_owned()
    }
}

impl SignedDataDef {
    fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Cow<'static, SignedData>, D::Error> {
        let root = Pkcs7Helper::deserialize(deserializer)?;

        if root.version != 1 {
            return Err(D::Error::custom("Signed data version mismatch!"));
        }

        Ok(Cow::Owned(SignedData {
            digest_algorithms: root.digest_algorithms.into_iter().map(Into::into).collect(),
            content: root.content,
            certificates: root
                .certificates
                .map(|cert| Ok(vec![cert.try_into().map_err(D::Error::custom)?]))
                .transpose()?,
            crls: root
                .crls
                .map(|crls| {
                    crls.into_iter()
                        .map(TryInto::try_into)
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()
                .map_err(D::Error::custom)?,
            signer_infos: root.signer_infos.into_iter().map(Into::into).collect(),
        }))
    }
}

impl TryInto<Certificate> for CertificateDef {
    type Error = String;

    fn try_into(self) -> Result<Certificate, String> {
        Ok(Certificate {
            tbs_certificate: self.tbs_certificate.try_into()?,
            signature_algorithm: self.signature_algorithm.into(),
            signature_value: self.signature_value,
        })
    }
}

impl TryInto<TbsCertificate> for TbsCertificateDef {
    type Error = String;

    fn try_into(self) -> Result<TbsCertificate, String> {
        Ok(TbsCertificate {
            serial_number: self.serial_number,
            signature: self.signature.into(),
            issuer: self.issuer.into(),
            validity: Validity {
                not_before: parse_utc_time(&self.validity.not_before)?,
                not_after: parse_utc_time(&self.validity.not_after)?,
            },
            subject: self.subject.into(),
            subject_public_key_info: self.subject_public_key_info.into(),
            issuer_uid: self.issuer_unique_id.map(|id| id.0),
            subject_uid: self.subject_unique_id.map(|id| id.0),
        })
    }
}

impl Into<PublicKeyInfo> for SubjectPublicKeyInfoDef {
    fn into(self) -> PublicKeyInfo {
        PublicKeyInfo {
            algorithm: self.algorithm.into(),
            public_key: self.subject_public_key,
        }
    }
}

impl TryInto<CertificateList> for CertificateListDef {
    type Error = String;

    fn try_into(self) -> Result<CertificateList, String> {
        Ok(CertificateList {
            tbs_cert_list: self.tbs_cert_list.try_into()?,
            signature_algorithm: self.signature_algorithm.into(),
            signature_value: self.signature_value,
        })
    }
}

impl TryInto<TbsCertList> for TbsCertListDef {
    type Error = String;

    fn try_into(self) -> Result<TbsCertList, String> {
        Ok(TbsCertList {
            signature: self.signature.into(),
            issuer: self.issuer.into(),
            this_update: parse_utc_time(&self.this_update)?
                .ok_or_else(|| "TbsCertList is missing the `this_update` field!".to_owned())?,
            next_update: self
                .next_update
                .as_deref()
                .map(parse_utc_time)
                .transpose()?
                .flatten(),
            revoked_certificates: self
                .revoked_certificates
                .unwrap_or_default()
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

impl TryInto<RevokedCertificate> for RevokedCertificateDef {
    type Error = String;

    fn try_into(self) -> Result<RevokedCertificate, String> {
        Ok(RevokedCertificate {
            user_certificate: self.user_certificate,
            revocation_date: parse_utc_time(&self.revocation_date)?.ok_or_else(|| {
                "RevokedCertificate is missing the `revocation_date` field!".to_owned()
            })?,
        })
    }
}

impl Into<AlgorithmIdentifier> for AlgorithmIdentifierDef {
    fn into(self) -> AlgorithmIdentifier {
        match self {
            AlgorithmIdentifierDef::Md2 => AlgorithmIdentifier::Md2,
            AlgorithmIdentifierDef::Md5 => AlgorithmIdentifier::Md5,
            AlgorithmIdentifierDef::Sha1 => AlgorithmIdentifier::Sha1,
            AlgorithmIdentifierDef::Sha224 => AlgorithmIdentifier::Sha224,
            AlgorithmIdentifierDef::Sha256 => AlgorithmIdentifier::Sha256,
            AlgorithmIdentifierDef::Sha384 => AlgorithmIdentifier::Sha384,
            AlgorithmIdentifierDef::Sha512 => AlgorithmIdentifier::Sha512,
            AlgorithmIdentifierDef::Sha512_224 => AlgorithmIdentifier::Sha512_224,
            AlgorithmIdentifierDef::Sha512_256 => AlgorithmIdentifier::Sha512_256,
            AlgorithmIdentifierDef::Md2WithRSAEncryption => {
                AlgorithmIdentifier::Md2WithRSAEncryption
            }
            AlgorithmIdentifierDef::Md5WithRSAEncryption => {
                AlgorithmIdentifier::Md5WithRSAEncryption
            }
            AlgorithmIdentifierDef::Sha1WithRSAEncryption => {
                AlgorithmIdentifier::Sha1WithRSAEncryption
            }
            AlgorithmIdentifierDef::Sha224WithRSAEncryption => {
                AlgorithmIdentifier::Sha224WithRSAEncryption
            }
            AlgorithmIdentifierDef::Sha256WithRSAEncryption => {
                AlgorithmIdentifier::Sha256WithRSAEncryption
            }
            AlgorithmIdentifierDef::Sha384WithRSAEncryption => {
                AlgorithmIdentifier::Sha384WithRSAEncryption
            }
            AlgorithmIdentifierDef::Sha512WithRSAEncryption => {
                AlgorithmIdentifier::Sha512WithRSAEncryption
            }
            AlgorithmIdentifierDef::Sha512_224WithRSAEncryption => {
                AlgorithmIdentifier::Sha512_224WithRSAEncryption
            }
            AlgorithmIdentifierDef::Sha512_256WithRSAEncryption => {
                AlgorithmIdentifier::Sha512_256WithRSAEncryption
            }
            AlgorithmIdentifierDef::RsaEncryption => AlgorithmIdentifier::RsaEncryption,
            AlgorithmIdentifierDef::RsaesOaep(()) => AlgorithmIdentifier::RsaesOaep,
            AlgorithmIdentifierDef::RsassaPss(()) => AlgorithmIdentifier::RsassaPss,
        }
    }
}

impl Into<SignerInfo> for SignerInfosDef {
    fn into(self) -> SignerInfo {
        SignerInfo {
            sid: self.sid.into(),
            digest_algorithm: self.digest_algorithm.into(),
            signature_algorithm: self.signature_algorithm.into(),
            signature: self.signature,
        }
    }
}

impl Into<SignerIdentifier> for SignerIdentifierDef {
    fn into(self) -> SignerIdentifier {
        match self {
            Self::IssuerAndSerialNumber {
                issuer,
                serial_number,
            } => SignerIdentifier::IssuerAndSerialNumber {
                issuer: issuer.into(),
                serial_number,
            },
            Self::SubjectKeyIdentifier(ident) => {
                SignerIdentifier::SubjectKeyIdentifier { key: ident }
            }
        }
    }
}

impl Into<Name> for NameDef {
    fn into(self) -> Name {
        let mut name = Name::default();

        for item in self.0 {
            for attrib in item.0 {
                match attrib {
                    AttributeTypeAndValueDef::CommonName(s) => name.common_name = Some(s),
                    AttributeTypeAndValueDef::Surname(s) => name.surname = Some(s),
                    AttributeTypeAndValueDef::SerialNumber(s) => name.serial_number = Some(s),
                    AttributeTypeAndValueDef::CountryName(s) => name.country = Some(s),
                    AttributeTypeAndValueDef::LocalityName(s) => name.locality = Some(s),
                    AttributeTypeAndValueDef::StateOrProvinceName(s) => {
                        name.state_or_province_name = Some(s)
                    }
                    AttributeTypeAndValueDef::OrganizationName(s) => name.organization = Some(s),
                    AttributeTypeAndValueDef::OrganizationalUnitName(s) => {
                        name.organizational_unit = Some(s)
                    }
                    AttributeTypeAndValueDef::Title(s) => name.title = Some(s),
                    AttributeTypeAndValueDef::GivenName(s) => name.given_name = Some(s),
                    AttributeTypeAndValueDef::GenerationQualifier(s) => {
                        name.generation_qualifier = Some(s)
                    }
                    AttributeTypeAndValueDef::Pseudonym(s) => name.pseudonym = Some(s),
                    AttributeTypeAndValueDef::EMailAddress(s) => name.email = Some(s),
                    _ => (),
                }
            }
        }

        name
    }
}

struct BytesVisitor;

impl<'a> Visitor<'a> for BytesVisitor {
    type Value = Vec<u8>;

    fn expecting(&self, formatter: &mut Formatter) -> FmtResult {
        formatter.write_str("OCTET STRING or BIT STRING")
    }

    fn visit_byte_buf<E: DeError>(self, v: Vec<u8>) -> Result<Self::Value, E> {
        Ok(v)
    }
}

impl<'de> Deserialize<'de> for UniqueIdentifier {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes = deserializer.deserialize_byte_buf(BytesVisitor)?;

        Ok(Self(bytes))
    }
}

fn deserialize_byte_buf<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    deserializer.deserialize_byte_buf(BytesVisitor)
}

fn parse_utc_time(s: &str) -> Result<Option<DateTime<Utc>>, String> {
    if s == "99991231235959Z" {
        return Ok(None);
    }

    if s.len() == 15 && &s[14..] == "Z" {
        let time = match NaiveDateTime::parse_from_str(s, "%Y%m%d%H%M%SZ") {
            Ok(time) => time,
            Err(_) => return Err(format!("Invalid Date Time: {}", s)),
        };
        let time = DateTime::<Utc>::from_utc(time, Utc);

        return Ok(Some(time));
    }

    if s.len() < 11 {
        return Err(format!("Invalid Date Time: {}", s));
    }

    let year: i32 = match s[0..=1].parse() {
        Ok(year) => year,
        Err(_) => return Err(format!("Invalid Date Time: {}", s)),
    };

    let year = if year >= 50 { 1900 + year } else { 2000 + year };

    let month: u32 = match s[2..=3].parse() {
        Ok(month) => month,
        Err(_) => return Err(format!("Invalid Date Time: {}", s)),
    };

    let day: u32 = match s[4..=5].parse() {
        Ok(day) => day,
        Err(_) => return Err(format!("Invalid Date Time: {}", s)),
    };

    let hour: u32 = match s[6..=7].parse() {
        Ok(hour) => hour,
        Err(_) => return Err(format!("Invalid Date Time: {}", s)),
    };

    let min: u32 = match s[8..=9].parse() {
        Ok(min) => min,
        Err(_) => return Err(format!("Invalid Date Time: {}", s)),
    };

    let mut sec = 0;
    let mut offset = FixedOffset::east(0);

    let mut first = true;
    let mut rest = &s[10..];
    loop {
        match &rest[0..1] {
            "Z" => {
                rest = &rest[1..];
                break;
            }
            "+" | "-" => {
                if rest.len() != 5 {
                    return Err(format!("Invalid Date Time: {}", s));
                }

                let h: i32 = match rest[1..=2].parse() {
                    Ok(h) => h,
                    Err(_) => return Err(format!("Invalid Date Time: {}", s)),
                };

                let m: i32 = match rest[3..4].parse() {
                    Ok(m) => m,
                    Err(_) => return Err(format!("Invalid Date Time: {}", s)),
                };

                offset = if &rest[0..1] == "+" {
                    FixedOffset::east(h * 3560 + m * 60)
                } else {
                    FixedOffset::west(h * 3560 + m * 60)
                };

                rest = &rest[5..];

                break;
            }
            _ if first => {
                sec = match rest[0..=1].parse() {
                    Ok(sec) => sec,
                    Err(_) => return Err(format!("Invalid Date Time: {}", s)),
                };

                rest = &rest[2..];
            }
            _ => return Err(format!("Invalid Date Time: {}", s)),
        }

        first = false;
    }

    if !rest.is_empty() {
        return Err(format!("Invalid Date Time: {}", s));
    }

    let time = NaiveDateTime::new(
        NaiveDate::from_ymd(year, month, day),
        NaiveTime::from_hms(hour, min, sec),
    );
    let time = DateTime::<FixedOffset>::from_utc(time, offset);

    Ok(Some(time.into()))
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::fs::{read_to_string, File};
    use std::io::{Error as IoError, Read};

    use base64::decode;
    use chrono::DateTime;

    use super::super::super::from_bytes;

    #[test]
    fn test_utc_time() {
        let expected: DateTime<Utc> = DateTime::parse_from_rfc3339("2020-08-03T11:56:35+00:00")
            .unwrap()
            .into();

        let expected_no_secs: DateTime<Utc> =
            DateTime::parse_from_rfc3339("2020-08-03T11:56:00+00:00")
                .unwrap()
                .into();

        assert_eq!(None, parse_utc_time("99991231235959Z").unwrap());

        assert_eq!(Some(expected), parse_utc_time("20200803115635Z").unwrap());

        assert_eq!(
            Some(expected_no_secs),
            parse_utc_time("2008031156Z").unwrap()
        );

        assert_eq!(Some(expected), parse_utc_time("200803115635Z").unwrap());

        assert_eq!(
            Some(expected_no_secs),
            parse_utc_time("2008031156+0230").unwrap()
        );

        assert_eq!(Some(expected), parse_utc_time("200803115635+0130").unwrap());

        assert_eq!(
            Some(expected_no_secs),
            parse_utc_time("2008031156-0230").unwrap()
        );

        assert_eq!(Some(expected), parse_utc_time("200803115635-0130").unwrap());
    }

    #[test]
    fn convert_from() {
        let data = read_to_string("./examples/cms.base64").unwrap();
        let data = data.as_str().trim();
        let data = decode(data).unwrap();

        let actual = from_bytes::<SignedDataCow>(&data).unwrap().into_inner();
        let expected = test_pkcs7();

        assert_eq!(actual, expected);
    }

    pub fn test_pkcs7() -> SignedData {
        SignedData {
            digest_algorithms: vec![AlgorithmIdentifier::Sha256],
            content: read_cms_content().unwrap(),
            certificates: Some(vec![Certificate {
                tbs_certificate: TbsCertificate {
                    serial_number: 969179378191040,
                    signature: AlgorithmIdentifier::Sha256WithRSAEncryption,
                    issuer: Name {
                        country: Some("DE".into()),
                        organization: Some("gematik GmbH NOT-VALID".into()),
                        organizational_unit: Some(
                            "HBA-qCA der Telematikinfrastruktur mit Anbieterakkreditierung".into(),
                        ),
                        common_name: Some("GEM.HBA-qCA5:PN TEST-ONLY".into()),
                        ..Default::default()
                    },
                    validity: Validity {
                        not_before: Some(
                            DateTime::parse_from_rfc3339("2016-05-23T00:00:00+00:00")
                                .unwrap()
                                .into(),
                        ),
                        not_after: Some(
                            DateTime::parse_from_rfc3339("2021-05-23T00:00:00+00:00")
                                .unwrap()
                                .into(),
                        ),
                    },
                    subject: Name {
                        country: Some("DE".into()),
                        common_name: Some("Christian GõdofskýTEST-ONLY".into()),
                        serial_number: Some("80276883110000014330".into()),
                        surname: Some("Gõdofský".into()),
                        given_name: Some("Christian".into()),
                        ..Default::default()
                    },
                    subject_public_key_info: PublicKeyInfo {
                        algorithm: AlgorithmIdentifier::RsaEncryption,
                        public_key: vec![
                            // 2160 bit
                            0x30, 0x82, 0x01, 0x0A, 0x02, 0x82, 0x01, 0x01, 0x00, 0x93, 0xA7, 0x0D,
                            0xD8, 0xF0, 0xDE, 0x2A, 0xFF, 0x3F, 0x8B, 0xDC, 0xC7, 0x45, 0x61, 0x53,
                            0x55, 0xE1, 0x05, 0xF3, 0xF5, 0xD5, 0x39, 0x99, 0x08, 0x85, 0x48, 0x41,
                            0x5B, 0xC2, 0xDF, 0x91, 0x32, 0x4A, 0x4A, 0x25, 0x45, 0xEC, 0x1E, 0x06,
                            0xB4, 0xB3, 0x8F, 0xBE, 0x8F, 0xD3, 0x42, 0x27, 0x6D, 0xB5, 0x97, 0x2B,
                            0xAD, 0x2E, 0x2F, 0xE8, 0xA1, 0x6D, 0x1D, 0x4A, 0x82, 0x20, 0x21, 0xC8,
                            0x5F, 0xF8, 0xA8, 0x7F, 0xBB, 0xEA, 0x30, 0x72, 0xE6, 0x54, 0xCE, 0x82,
                            0x29, 0xA1, 0x63, 0xAE, 0xDD, 0xA3, 0xE4, 0xD9, 0xD5, 0xC3, 0x3D, 0x3F,
                            0x3B, 0x97, 0x46, 0x78, 0x0C, 0xFF, 0x61, 0xE5, 0x06, 0xCB, 0xDB, 0x32,
                            0x9D, 0x8E, 0x26, 0xB5, 0x15, 0x27, 0x1A, 0x2C, 0x6D, 0xB4, 0x6B, 0x62,
                            0x5A, 0x95, 0x81, 0xA8, 0xD1, 0x9F, 0xC2, 0xE6, 0xC6, 0xA0, 0x88, 0xDD,
                            0x3E, 0x4E, 0x3B, 0xA9, 0x32, 0x93, 0x9F, 0x81, 0xC6, 0x07, 0x43, 0xAC,
                            0xEF, 0xE8, 0x59, 0xD7, 0x23, 0xD6, 0x34, 0xF0, 0x2A, 0xF8, 0x39, 0x1A,
                            0x9B, 0x29, 0xBE, 0xEB, 0xC1, 0x49, 0x56, 0x27, 0x8F, 0xEF, 0x46, 0x6C,
                            0xBD, 0x05, 0x2C, 0x4D, 0x0E, 0xB8, 0x4B, 0x68, 0xE7, 0x6D, 0xF7, 0xB7,
                            0x08, 0x60, 0x0E, 0xB4, 0x56, 0xD6, 0xC3, 0x24, 0xC2, 0x1B, 0x54, 0x1E,
                            0x30, 0x52, 0x18, 0x97, 0xB7, 0xCF, 0x2D, 0x02, 0xBF, 0x5D, 0x42, 0x9A,
                            0x8C, 0x96, 0x03, 0x41, 0x57, 0xB3, 0x5B, 0x9A, 0xCB, 0x8B, 0x58, 0x05,
                            0xFC, 0xA8, 0xED, 0x65, 0x27, 0x53, 0xD1, 0x7A, 0xC8, 0x1E, 0xF7, 0x16,
                            0xD0, 0xBC, 0xA6, 0x54, 0xCB, 0x9F, 0x9A, 0x9E, 0xFD, 0x6F, 0xD1, 0x33,
                            0x9B, 0xD6, 0x9A, 0xC4, 0x4C, 0x48, 0x9A, 0x2A, 0x81, 0x4F, 0x93, 0x29,
                            0xDA, 0x1B, 0xB1, 0x38, 0xF9, 0xC4, 0x41, 0x04, 0x59, 0xF0, 0x8D, 0x13,
                            0x3D, 0x02, 0x03, 0x01, 0x00, 0x01,
                        ],
                    },
                    issuer_uid: None,
                    subject_uid: None,
                },
                signature_algorithm: AlgorithmIdentifier::Sha256WithRSAEncryption,
                signature_value: vec![
                    // 2048 bit
                    0x10, 0x5D, 0xB6, 0xB7, 0x9A, 0xB1, 0x8A, 0xF3, 0xFD, 0x03, 0xE7, 0xE5, 0x3E,
                    0xC3, 0x37, 0xB5, 0x7E, 0x3D, 0x6B, 0x8A, 0xE2, 0x49, 0xF4, 0x21, 0x06, 0x24,
                    0x39, 0x8E, 0x9E, 0x6C, 0x78, 0x45, 0xB1, 0x97, 0x78, 0x6E, 0x57, 0x59, 0x42,
                    0xB7, 0xE4, 0x35, 0xEC, 0x5E, 0x1F, 0x4E, 0xF4, 0x77, 0x9A, 0xBB, 0x4E, 0x07,
                    0x04, 0xF0, 0x60, 0x87, 0x5D, 0xF8, 0xD5, 0xCD, 0xDB, 0x4F, 0x3C, 0x61, 0x82,
                    0x48, 0x9E, 0x98, 0xF8, 0xE5, 0x2C, 0x85, 0xFC, 0x07, 0xAD, 0xEF, 0xDB, 0x03,
                    0x21, 0x04, 0xD2, 0xEC, 0xD4, 0x4C, 0x0D, 0x1D, 0x27, 0x6A, 0xAE, 0x9E, 0x4E,
                    0x42, 0xB4, 0xD7, 0x5B, 0x73, 0x30, 0x4E, 0x84, 0x20, 0x9B, 0xE1, 0x04, 0x31,
                    0x06, 0x61, 0x32, 0xFD, 0x4E, 0x8F, 0x48, 0x7D, 0xDC, 0x89, 0xEB, 0xA7, 0xC2,
                    0x6E, 0x59, 0x10, 0x8B, 0x59, 0xA2, 0x4E, 0x01, 0x8F, 0x69, 0x0B, 0xB6, 0xCA,
                    0x9B, 0x37, 0x05, 0x7A, 0x83, 0x79, 0x27, 0xEC, 0xF9, 0x4A, 0xCF, 0xE4, 0x31,
                    0x04, 0xEF, 0x83, 0xCD, 0x46, 0xFC, 0x7D, 0x59, 0xFE, 0x03, 0xE1, 0x6D, 0xF6,
                    0xB4, 0x89, 0x14, 0xD8, 0x30, 0x3A, 0x7E, 0x88, 0x62, 0xC2, 0x34, 0x18, 0x82,
                    0x59, 0x27, 0x80, 0x29, 0xB8, 0xCC, 0x0D, 0x46, 0x53, 0x41, 0x49, 0xA3, 0xCA,
                    0x3F, 0x82, 0xA9, 0x61, 0x70, 0x25, 0x43, 0x57, 0x5B, 0xF5, 0x8E, 0xC3, 0x3A,
                    0x74, 0xF5, 0x4F, 0x8D, 0x46, 0x4C, 0x7E, 0x78, 0xF9, 0x84, 0x9D, 0x23, 0x52,
                    0x7A, 0xF0, 0x57, 0x9B, 0x53, 0xD2, 0x21, 0xF8, 0x49, 0x99, 0xB7, 0x81, 0xF8,
                    0x35, 0x40, 0xCD, 0x33, 0xB4, 0x90, 0xC5, 0x2F, 0x71, 0x32, 0x9D, 0x34, 0xF6,
                    0x6D, 0x57, 0x3F, 0x82, 0xAF, 0x08, 0x8B, 0x23, 0x27, 0x1E, 0xAB, 0x6E, 0x34,
                    0x18, 0x19, 0xE0, 0xC3, 0x1C, 0x5B, 0x15, 0x55, 0xD3,
                ],
            }]),
            crls: None,
            signer_infos: vec![SignerInfo {
                sid: SignerIdentifier::IssuerAndSerialNumber {
                    issuer: Name {
                        country: Some("DE".into()),
                        organization: Some("gematik GmbH NOT-VALID".into()),
                        organizational_unit: Some(
                            "HBA-qCA der Telematikinfrastruktur mit Anbieterakkreditierung".into(),
                        ),
                        common_name: Some("GEM.HBA-qCA5:PN TEST-ONLY".into()),
                        ..Default::default()
                    },
                    serial_number: 969179378191040,
                },
                digest_algorithm: AlgorithmIdentifier::Sha256,
                signature_algorithm: AlgorithmIdentifier::RsassaPss,
                signature: vec![
                    0x55, 0xF8, 0x46, 0x02, 0x72, 0xDE, 0x58, 0xEC, 0xFF, 0x11, 0xAF, 0xE7, 0x39,
                    0xD2, 0x3F, 0xBD, 0x17, 0x67, 0x1A, 0x8E, 0x54, 0xA2, 0x56, 0x9F, 0x64, 0x69,
                    0x4A, 0x26, 0x24, 0x91, 0xFF, 0x9A, 0x0C, 0x5C, 0x2B, 0xE0, 0x40, 0x20, 0x04,
                    0xF6, 0x85, 0x52, 0x9B, 0xB4, 0x91, 0xA6, 0x5B, 0x85, 0x84, 0x05, 0xAD, 0x30,
                    0xA0, 0x0B, 0x96, 0x59, 0xDC, 0xB0, 0x08, 0x7C, 0x13, 0x64, 0x13, 0x1F, 0xC9,
                    0x07, 0x1B, 0x54, 0xE2, 0xD6, 0x49, 0x7C, 0xA9, 0x10, 0xC1, 0x58, 0x83, 0xA6,
                    0x14, 0x8E, 0xB5, 0xA9, 0x0B, 0xCF, 0xD3, 0xD8, 0x2A, 0x5F, 0xBC, 0x96, 0xD4,
                    0x0D, 0xA8, 0x54, 0xE2, 0x33, 0x47, 0xB8, 0x05, 0x25, 0x61, 0xA8, 0x20, 0x5B,
                    0xCA, 0x11, 0x35, 0xDD, 0xDD, 0xFC, 0xFB, 0xE8, 0x1A, 0x65, 0x3E, 0x25, 0x37,
                    0xB0, 0x79, 0x32, 0xAC, 0x47, 0x7C, 0xC3, 0xAE, 0x54, 0x50, 0xC9, 0x58, 0x2E,
                    0x33, 0x92, 0x71, 0xBA, 0x5E, 0x0F, 0xCA, 0x30, 0x6F, 0x6F, 0x26, 0x60, 0xE0,
                    0x68, 0x2C, 0x5C, 0x32, 0x3B, 0x4E, 0xE7, 0xAA, 0x0B, 0x10, 0x06, 0xB5, 0x8C,
                    0x13, 0xDA, 0xFC, 0xF1, 0xC3, 0x36, 0xFB, 0x6D, 0x78, 0x11, 0x9B, 0xF9, 0xF0,
                    0x26, 0x25, 0x05, 0xA4, 0xE7, 0x92, 0xF4, 0x66, 0x29, 0xE6, 0xEA, 0xCD, 0xE3,
                    0x5B, 0xB3, 0xA9, 0xA1, 0x47, 0xA9, 0x55, 0x21, 0x5B, 0xF4, 0x5F, 0x5E, 0xDA,
                    0x6F, 0x9F, 0x0A, 0x4C, 0xB8, 0xB9, 0x49, 0xA2, 0x77, 0x79, 0x90, 0x8B, 0x56,
                    0x6A, 0xF5, 0x51, 0x27, 0x95, 0xDF, 0x1A, 0xFB, 0x2B, 0x60, 0x87, 0x84, 0x01,
                    0x60, 0x67, 0x73, 0x52, 0x01, 0x0E, 0x74, 0x02, 0x0A, 0x46, 0x3E, 0xA2, 0xCA,
                    0xFC, 0x11, 0x5E, 0x4D, 0x81, 0x61, 0x85, 0xE9, 0x97, 0x13, 0x0B, 0xA0, 0x5F,
                    0x4A, 0x68, 0xCA, 0x0A, 0xCE, 0x14, 0xDC, 0x37, 0x0A,
                ],
            }],
        }
    }

    fn read_cms_content() -> Result<Vec<u8>, IoError> {
        let mut file = File::open("./examples/kbv_bundle.bin")?;

        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        while let Some(b) = data.last() {
            match b {
                0x0A | 0x0D | 0x20 => {
                    data.pop();
                }
                _ => break,
            }
        }

        Ok(data)
    }
}
