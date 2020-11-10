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
use openssl::pkey::PKey;
use serde_json::{from_str, to_string_pretty, Value};
use structopt::StructOpt;

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

    /// File path of the claims to encode within the ACCESS_TOKEN.
    ///
    /// The claims that are encoded within the ACCESS_TOKEN can be any valid JSON file. The content
    /// of the file is not validated, so you can encode any information in the ACCESS_TOKEN.
    #[structopt(short, long)]
    claims: PathBuf,
}

pub fn execute(opts: Opts) {
    let key = read(opts.key).expect("Unable to read private key!");
    let key = PKey::private_key_from_pem(&key).expect("Unable to interpret private key!");

    let claims = read(opts.claims).expect("Unable to read claims!");
    let claims = from_utf8(&claims).expect("Unable to interpret claims: Invalid UTF-8 string!");
    let claims =
        from_str::<Value>(&claims).expect("Unable to interpret claims: Invalid JSON format!");

    println!(
        "\nRead the following claims:\n{}",
        to_string_pretty(&claims).unwrap()
    );

    let access_token = sign(&claims, key).expect("Unable to crate ACCESS_TOKEN");

    println!("\nGenerated the following access token:\n{}", access_token);
}
