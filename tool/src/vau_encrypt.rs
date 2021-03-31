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

use bytes::{BufMut, BytesMut};
use openssl::{
    bn::{BigNum, BigNumContext},
    derive::Deriver,
    ec::{EcGroup, EcKey},
    hash::MessageDigest,
    hkdf::{Hkdf, Mode},
    nid::Nid,
    pkey::PKey,
    symm::{encrypt_aead, Cipher},
    x509::X509,
};
use rand::{rngs::OsRng, Rng};
use structopt::StructOpt;

use super::misc::{read_input, write_output};

#[derive(StructOpt)]
/// Tool to encrypt VAU requests.
///
/// The VAU tunnel is used to send encrypted HTTP requests and content to the FD service using
/// normal HTTP requests. This tool is used to create an encrypted request that can then be send to
/// the VAU tunnel of the FD server.
pub struct Opts {
    /// Certificate of the VAU server to use for encryption.
    ///
    /// Path to the X509 certificate to use for encrypting the VAU request.
    #[structopt(short, long)]
    cert: PathBuf,

    /// Private key of the client to use for encryption.
    ///
    /// Path to the private key in PEM format that is used to create the shared secret between
    /// client and server, the VAU request is enctypted with. If no key is passed, a new one is
    /// generated.
    #[structopt(short, long)]
    key: Option<PathBuf>,

    /// File path of request to encrypt.
    ///
    /// Path to the file that should be encrypted. The file should contain the format described in
    /// the VAU specification.
    ///
    /// Example:
    ///     "1 {access_token} {request_id} {response_key} {inner_http_request}"
    ///
    /// If no file is passed, the content is read from stdin.
    #[structopt(short, long)]
    input: Option<PathBuf>,

    /// File path to store the encrypted request at.
    ///
    /// Path to the file the encrypted request is stored at. If this parameter is not passed, the
    /// encrypted request is written to stdout.
    #[structopt(short, long)]
    output: Option<PathBuf>,
}

pub fn execute(opts: Opts) {
    /* read plain text */
    let group = EcGroup::from_curve_name(Nid::from_raw(927))
        .expect("Unable to create group for BRAINPOOL_P256_R1");
    let mut context = BigNumContext::new().expect("Unable to create BigNumContext");
    let plain = read_input(&opts.input);

    /* load certificate and public key */
    let cert = read(opts.cert).expect("Unable to read certificate");
    let cert = X509::from_pem(&cert).expect("Unable to load certificate");

    let pub_key = cert
        .public_key()
        .expect("Unable to extract public key from certificate");

    /* load private key */
    let pri_key = if let Some(key) = opts.key {
        let key = read(key).expect("Unable to read private key");

        PKey::private_key_from_pem(&key).expect("Unable to load private key")
    } else {
        let key = EcKey::generate(&group).expect("Unable to generate private key");

        PKey::from_ec_key(key).expect("Unable to generate private key")
    };

    /* generate shared secret */
    let mut deriver = Deriver::new(&pri_key).expect("Unable to create deriver");
    deriver
        .set_peer(&pub_key)
        .expect("Unable to set deriver public key");
    let shared_secret = deriver
        .derive_to_vec()
        .expect("Unable to generate shared secret");
    let aes_key = Hkdf::new(MessageDigest::sha256())
        .expect("Unable to creaet HKDF context")
        .set_mode(Mode::ExtractAndExpand)
        .expect("Unable to set HDKF mode")
        .set_info(Some(b"ecies-vau-transport"))
        .expect("Unable to set HDKF info")
        .set_secret(&shared_secret)
        .expect("Unable to set HDKF secret")
        .derive(16)
        .expect("Unable to generate AES key");

    /* encrypt plain text */
    let iv: [u8; 12] = OsRng.gen();
    let mut tag = [0; 16];

    let cipher = encrypt_aead(
        Cipher::aes_128_gcm(),
        &aes_key,
        Some(&iv),
        &[],
        &plain,
        &mut tag,
    )
    .expect("Unable to encrypt plain text");

    /* assemble output */
    let mut x = BigNum::new().expect("Unable to create BigNum");
    let mut y = BigNum::new().expect("Unable to create BigNum");
    let pri_key = pri_key
        .ec_key()
        .expect("Unable to extract EC key from public key");
    let pub_ec_key = pri_key.public_key();
    pub_ec_key
        .affine_coordinates_gfp(&group, &mut x, &mut y, &mut context)
        .expect("Unable to extract coordinates from EC key");
    let x = pad(x.to_vec(), 32);
    let y = pad(y.to_vec(), 32);

    let mut data = BytesMut::new();
    data.put_u8(0x01);
    data.put(&x[..]);
    data.put(&y[..]);
    data.put(&iv[..]);
    data.put(&cipher[..]);
    data.put(&tag[..]);

    /* write output */
    write_output(&opts.output, &data);
}

fn pad(mut data: Vec<u8>, mut len: usize) -> Vec<u8> {
    len -= data.len();

    let mut ret = vec![0; len];
    ret.append(&mut data);

    ret
}
