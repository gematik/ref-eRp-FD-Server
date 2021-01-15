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

use std::path::PathBuf;

use bytes::BytesMut;
use openssl::symm::{decrypt_aead, Cipher};
use structopt::StructOpt;
use vau::hex_decode;

use super::misc::{read_input, write_output};

#[derive(StructOpt)]
/// Tool to decrypt AES-128-GCM messages.
///
/// This tool is used to decrypt AES-128-GCM messages. These messages are usually returned within
/// the response of the FD through the VAU tunnel.
pub struct Opts {
    /// Key to decrypt message with.
    ///
    /// The key used to decrypt the AES-128-GCM ciphter text with. The key should be passed in it's
    /// hexadecimal representatio (e.g. 0123456789ABCDEF0123456789ABCDEF).
    #[structopt(short, long)]
    key: String,

    /// Path of file to decrypt.
    ///
    /// Path to the file that contains the cipher text. If this parameter is not passed, the cipher
    /// text is read from stdin.
    #[structopt(short, long)]
    input: Option<PathBuf>,

    /// Path of file to write output to.
    ///
    /// Path of the file to write the decrypted output to. If this parameter is not passed, the
    /// decrypted text is written to stdout.
    #[structopt(short, long)]
    output: Option<PathBuf>,
}

pub fn execute(opts: Opts) {
    /* read cipher */
    let cipher = read_input(&opts.input);
    let cipher = BytesMut::from(&cipher[..]);

    /* read key */
    let key = hex_decode(&opts.key).expect("Invalid key");

    /* decrypt */
    if cipher.len() < 28 {
        panic!("Invalid input data!");
    }

    let cipher = cipher.as_ref();
    let (iv, cipher) = cipher.split_at(12);
    let (cipher, tag) = cipher.split_at(cipher.len() - 16);

    let data = decrypt_aead(Cipher::aes_128_gcm(), &key, Some(&iv), &[], cipher, tag)
        .expect("Unable to decrypt data");

    /* write output */
    write_output(&opts.output, &data);
}
