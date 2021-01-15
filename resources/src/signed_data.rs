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

#[derive(Clone, Debug, PartialEq)]
pub struct SignedData {
    pub digest_algorithms: Vec<AlgorithmIdentifier>,
    pub content: Vec<u8>,
    pub certificates: Option<Vec<Certificate>>,
    pub crls: Option<Vec<CertificateList>>,
    pub signer_infos: Vec<SignerInfo>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Certificate {
    pub tbs_certificate: TbsCertificate,
    pub signature_algorithm: AlgorithmIdentifier,
    pub signature_value: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TbsCertificate {
    pub serial_number: usize,
    pub signature: AlgorithmIdentifier,
    pub issuer: Name,
    pub validity: Validity,
    pub subject: Name,
    pub subject_public_key_info: PublicKeyInfo,
    pub issuer_uid: Option<Vec<u8>>,
    pub subject_uid: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CertificateList {
    pub tbs_cert_list: TbsCertList,
    pub signature_algorithm: AlgorithmIdentifier,
    pub signature_value: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TbsCertList {
    pub signature: AlgorithmIdentifier,
    pub issuer: Name,
    pub this_update: DateTime<Utc>,
    pub next_update: Option<DateTime<Utc>>,
    pub revoked_certificates: Vec<RevokedCertificate>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RevokedCertificate {
    pub user_certificate: usize,
    pub revocation_date: DateTime<Utc>,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Name {
    pub country: Option<String>,
    pub organization: Option<String>,
    pub organizational_unit: Option<String>,
    pub distinguished_name_qualifier: Option<String>,
    pub state_or_province_name: Option<String>,
    pub common_name: Option<String>,
    pub serial_number: Option<String>,
    pub locality: Option<String>,
    pub title: Option<String>,
    pub surname: Option<String>,
    pub given_name: Option<String>,
    pub initials: Option<String>,
    pub pseudonym: Option<String>,
    pub generation_qualifier: Option<String>,
    pub email: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Validity {
    pub not_before: Option<DateTime<Utc>>,
    pub not_after: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SignerInfo {
    pub sid: SignerIdentifier,
    pub digest_algorithm: AlgorithmIdentifier,
    pub signature_algorithm: AlgorithmIdentifier,
    pub signature: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub enum SignerIdentifier {
    IssuerAndSerialNumber { issuer: Name, serial_number: usize },
    SubjectKeyIdentifier { key: Vec<u8> },
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub enum AlgorithmIdentifier {
    Md2,
    Md5,
    Sha1,
    Sha224,
    Sha256,
    Sha384,
    Sha512,
    Sha512_224,
    Sha512_256,
    Md2WithRSAEncryption,
    Md5WithRSAEncryption,
    Sha1WithRSAEncryption,
    Sha224WithRSAEncryption,
    Sha256WithRSAEncryption,
    Sha384WithRSAEncryption,
    Sha512WithRSAEncryption,
    Sha512_224WithRSAEncryption,
    Sha512_256WithRSAEncryption,
    RsaEncryption,
    RsaesOaep,
    RsassaPss,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PublicKeyInfo {
    pub algorithm: AlgorithmIdentifier,
    pub public_key: Vec<u8>,
}
