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

use bytes::BytesMut;
use openssl::ec::EcKey;
use structopt::StructOpt;
use vau::Decrypter;

use super::misc::{read_input, write_output};

#[derive(StructOpt)]
/// Tool to decrypt VAU requests.
///
/// The VAU tunnel is used to send encrypted HTTP requests and content to the FD service using
/// normal HTTP requests. This tool is used to extract the HTTP request stored in the payload of a
/// VAU request.
pub struct Opts {
    /// File path of the private key.
    ///
    /// Path to the file that contains the private key in PEM format to decrypt the VAU request.
    #[structopt(short, long)]
    key: PathBuf,

    /// File path of the encrypted payload.
    ///
    /// Path to the file that contains the encrypted payload of a VAU request. If this parameter is
    /// not passed, the encrypted payload is read from stdin.
    #[structopt(short, long)]
    input: Option<PathBuf>,

    /// File path to store the decrypted request at.
    ///
    /// Path to the file the decrypted request is stored at. If this parameter is not passed, the
    /// decrypted request is written to stdout.
    #[structopt(short, long)]
    output: Option<PathBuf>,
}

pub fn execute(opts: Opts) {
    /* read cipher */
    let cipher = read_input(&opts.input);
    let cipher = BytesMut::from(&cipher[..]);

    /* read key */
    let key = read(opts.key).expect("Unable to read key");
    let key = EcKey::private_key_from_pem(&key).expect("Unable to load key");

    /* decrypt */
    let mut decrypter = Decrypter::new(key).expect("Unable to create decrypter");
    let plain = decrypter.decrypt(cipher).expect("Unable to decrypt data");

    /* write output */
    write_output(&opts.output, &plain);
}
