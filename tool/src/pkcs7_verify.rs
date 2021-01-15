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
    stack::Stack,
    x509::{store::X509StoreBuilder, X509},
};
use structopt::StructOpt;

use super::misc::{read_input, write_output};

#[derive(StructOpt)]
/// Tool to decode PKCS#7 file.
///
/// This tool is used to decode the content of a PKCS#7 file. These files are used, for example,
/// as parameters for the Task $activate operation.
pub struct Opts {
    /// File path of the certificate to verify the PKCS#7 file against.
    ///
    /// Path to the file that contains the X509 certificate that is used to verify the content of
    /// the PKCS#7 file. This could be the certificate that is embedded in the PCKS#7 file or a
    /// chain of certificates that validates the embedded certificate.
    #[structopt(short, long)]
    cert: PathBuf,

    /// Path of the PKCS#7 file to decode.
    ///
    /// Path to the PKCS#7 file that needs to be verified and decoded. If this parameter is not
    /// passed, the PKCS#7 file is read from stdin.
    #[structopt(short, long)]
    input: Option<PathBuf>,

    /// Path of file to write the output to.
    ///
    /// Path of the file to write the verified content of the PKCS#7 file to. If this parameter is
    /// not passed, the content is written to stdout.
    #[structopt(short, long)]
    output: Option<PathBuf>,
}

pub fn execute(opts: Opts) {
    /* load certificate and public key */
    let cert = read(opts.cert).expect("Unable to read certificate");
    let cert = X509::from_pem(&cert).expect("Unable to load certificate");

    let mut certs = Stack::new().expect("Unable to create certificate stack");
    certs
        .push(cert.clone())
        .expect("Unable to push certificate to stack");

    let mut store = X509StoreBuilder::new().expect("Unable to create certificate store");
    store
        .add_cert(cert)
        .expect("Unable to add certificate to chain");
    let store = store.build();

    /* load content of the PKCS#7 file */
    let pkcs7 = read_input(&opts.input);
    let pkcs7 = Pkcs7::from_pem(&pkcs7).expect("Unable to load PKCS#7 file");

    let mut content = Vec::new();
    pkcs7
        .verify(
            &certs,
            &store,
            None,
            Some(&mut content),
            Pkcs7Flags::empty(),
        )
        .expect("Unable to verify PKCS#7 file");

    write_output(&opts.output, &content);
}
