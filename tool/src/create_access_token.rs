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

use std::fs::read;
use std::path::PathBuf;
use std::str::from_utf8;

use miscellaneous::jwt::sign;
use openssl::{pkey::PKey, x509::X509};
use serde_json::{from_str, Value};
use structopt::StructOpt;

use super::misc::read_input;

#[derive(StructOpt)]
/// Tool to create ACCESS_TOKEN with.pkcs7_sign
///
/// This tool is used to create ACCESS_TOKEN to authenticate users against the FD server.
pub struct Opts {
    /// File path of the private key.
    ///
    /// Path to the file that contians the private key in PEM format that is used to sign the
    /// ACCESS_TOKEN with.
    #[structopt(short, long)]
    key: PathBuf,

    /// Certificate to embedd in the header of the JWS document.
    ///
    /// Path to the file that contians the cerificate PEM format that is added to the header of
    /// the JWS document.
    #[structopt(short = "z", long)]
    cert: Option<PathBuf>,

    /// File path of the claims to encode within the ACCESS_TOKEN.
    ///
    /// The claims that are encoded within the ACCESS_TOKEN can be any valid JSON file. The content
    /// of the file is not validated, so you can encode any information in the ACCESS_TOKEN.
    #[structopt(short, long)]
    claims: Option<PathBuf>,
}

pub fn execute(opts: Opts) {
    let key = read(opts.key).expect("Unable to read private key!");
    let key = PKey::private_key_from_pem(&key).expect("Unable to interpret private key!");

    let claims = read_input(&opts.claims);
    let claims = from_utf8(&claims).expect("Unable to interpret claims: Invalid UTF-8 string!");
    let claims =
        from_str::<Value>(&claims).expect("Unable to interpret claims: Invalid JSON format!");

    let cert = opts.cert.map(|cert| {
        let cert = read(cert).expect("Unable to read certificate!");

        X509::from_pem(&cert).expect("Unable to load certificate!")
    });

    let access_token =
        sign(&claims, key, cert.as_ref(), false).expect("Unable to crate ACCESS_TOKEN");

    println!("{}", access_token);
}
