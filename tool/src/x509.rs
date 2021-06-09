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
use std::str::FromStr;

use miscellaneous::admission::{Admission, Profession};
use openssl::{
    asn1::{Asn1Object, Asn1Time},
    bn::{BigNum, MsbOption},
    hash::MessageDigest,
    pkey::PKey,
    x509::{X509Extension, X509Req, X509},
};
use structopt::StructOpt;

use super::misc::{read_input, write_output};

#[derive(StructOpt)]
/// Tool to encode PKCS#7 file.
///
/// This tool is used to create PKCS#7 files with. These files are used, for example, as parameters
/// for the Task $activate operation.
pub struct Opts {
    /// File to read data from.
    #[structopt(short, long)]
    input: Option<PathBuf>,

    /// File to write output to.
    #[structopt(short, long)]
    output: Option<PathBuf>,

    /// Self sign cert with arg.
    #[structopt(long)]
    signkey: PathBuf,

    /// How long till expiry of a signed certificate.
    #[structopt(long, default_value = "30")]
    days: u32,

    /// Profession OIDs to add to the admission extension
    #[structopt(long)]
    profession: Vec<String>,
}

pub fn execute(opts: Opts) {
    eprintln!("CAUTION! Certificates created with this tool should not be used in productive environments. The admission extension is not fully supported!");

    let req = read_input(&opts.input);
    let req = X509Req::from_pem(&req).expect("Unable to load X509 request");

    let prikey = read(&opts.signkey).expect("Unable to read private key");
    let prikey = PKey::private_key_from_pem(&prikey).expect("Unable to load private key");

    let pubkey = req
        .public_key()
        .expect("Unable to extract public key from request");
    if !req.verify(&pubkey).expect("Error while verifying request") {
        panic!("Unable to verify request!");
    }

    let subject_name = req.subject_name();
    let not_before = Asn1Time::days_from_now(0).expect("Unable to create 'not before' time");
    let not_after = Asn1Time::days_from_now(opts.days).expect("Unable to create 'not after' time");

    let mut serial = BigNum::new().expect("Unable to create serial number");
    serial
        .rand(159, MsbOption::MAYBE_ZERO, false)
        .expect("Unable to generate random serial number");
    let serial = serial
        .to_asn1_integer()
        .expect("Unable to convert big num to asn1 integer");

    let mut x509 = X509::builder().expect("Unable to create X509 builder");
    if !opts.profession.is_empty() {
        let admission = Admission {
            professions: opts
                .profession
                .iter()
                .map(|s| Profession::from_str(s))
                .collect::<Result<_, _>>()
                .expect("Invalid profession OID"),
        };
        let admission = admission.to_der();

        let obj = Asn1Object::from_str("1.3.36.8.3.3")
            .expect("Unable to create OID for admission extension");
        let ext = X509Extension::from_obj(&obj, &admission)
            .expect("Unable to create admission extension");

        x509.append_extension(ext)
            .expect("Unable to append admission extension");
    }

    x509.set_serial_number(&serial)
        .expect("Unable to set serial number");
    x509.set_subject_name(subject_name)
        .expect("Unable to set subject name");
    x509.set_issuer_name(subject_name)
        .expect("Unable to set issuer name");
    x509.set_not_before(&not_before)
        .expect("Unable to set 'not before' time");
    x509.set_not_after(&not_after)
        .expect("Unable to set 'not after' time");
    x509.set_pubkey(&pubkey).expect("Unable to set public key");
    x509.sign(&prikey, MessageDigest::sha256())
        .expect("Unable to sign certificate");

    let x509 = x509.build();
    let x509 = x509.to_pem().expect("Unable to convert cert to PEM");
    write_output(&opts.output, &x509);
}
