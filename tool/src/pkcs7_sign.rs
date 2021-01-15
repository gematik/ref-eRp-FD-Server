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

use std::fs::read;
use std::path::PathBuf;

use openssl::{
    pkcs7::{Pkcs7, Pkcs7Flags},
    pkey::PKey,
    stack::Stack,
    x509::X509,
};
use structopt::StructOpt;

use super::misc::{read_input, write_output};

#[derive(StructOpt)]
/// Tool to encode PKCS#7 file.
///
/// This tool is used to create PKCS#7 files with. These files are used, for example, as parameters
/// for the Task $activate operation.
pub struct Opts {
    /// File path of the private key.
    ///
    /// Path to the file that contains the private key in PEM format to sign the PKCS#7 file with.
    #[structopt(short, long)]
    key: PathBuf,

    /// File path of the certificate to embedd.
    ///
    /// Path to the file that contains the X509 certificate that is embedded in the PKCs#7 file.
    #[structopt(short, long)]
    cert: PathBuf,

    /// Path of file to sign.
    ///
    /// Path to the file that is embedded and signed in the PKCS#7 file. The file can be of any
    /// format. If this parameter is not passed, the content is read from stdin.
    #[structopt(short, long)]
    input: Option<PathBuf>,

    /// Path of file to write PKCS#7 file to.
    ///
    /// Path of the file to write the created PKCS#7 file to. If this parameter is not passed,
    /// the output is written to stdout.
    #[structopt(short, long)]
    output: Option<PathBuf>,
}

pub fn execute(opts: Opts) {
    /* load certificate and public key */
    let cert = read(opts.cert).expect("Unable to read certificate");
    let cert = X509::from_pem(&cert).expect("Unable to load certificate");

    /* load private key */
    let pkey = read(opts.key).expect("Unable to read private key");
    let pkey = PKey::private_key_from_pem(&pkey).expect("Unable to load private key");

    /* load content of the PKCS#7 file */
    let content = read_input(&opts.input);

    /* create the PKCS#7 file */
    let certs = Stack::new().expect("Unable to create certificate stack");
    let pkcs7 = Pkcs7::sign(&cert, &pkey, &certs, &content, Pkcs7Flags::empty())
        .expect("Unable to sign PKCS#7 file");
    let pkcs7 = pkcs7.to_pem().expect("Unable to store PKCS#7 file");

    write_output(&opts.output, &pkcs7);
}
