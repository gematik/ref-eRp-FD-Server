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

use bytes::{BufMut, BytesMut};
use openssl::symm::{encrypt_aead, Cipher};
use rand::random;
use structopt::StructOpt;
use vau::hex_decode;

use super::misc::{read_input, write_output};

#[derive(StructOpt)]
/// Tool to encrypt AES-128-GCM messages.
///
/// This tool is used to encrypt AES-128-GCM messages. These messages are usually returned within
/// the response of the FD through the VAU tunnel.
pub struct Opts {
    /// Key to encrypt message with.
    ///
    /// The key used to encrypt the plain text with. The key should be passed in it's hexadecimal
    /// representatio (e.g. 0123456789ABCDEF0123456789ABCDEF).
    #[structopt(short, long)]
    key: String,

    /// Path of file to encrypt.
    ///
    /// Path to the file that contains the plain text. If this parameter is not passed, the plain
    /// text is read from stdin.
    #[structopt(short, long)]
    input: Option<PathBuf>,

    /// Path of file to write cipher text to.
    ///
    /// Path of the file to write the cipher text to. If this parameter is not passed, the cipher
    /// text is written to stdout.
    #[structopt(short, long)]
    output: Option<PathBuf>,
}

pub fn execute(opts: Opts) {
    /* read plain */
    let plain = read_input(&opts.input);
    let plain = BytesMut::from(&plain[..]);

    /* read key */
    let key = hex_decode(&opts.key).expect("Invalid key");

    /* encrypt data */
    let iv: [u8; 12] = random();
    let mut tag = [0; 16];

    let cipher = encrypt_aead(
        Cipher::aes_128_gcm(),
        &key,
        Some(&iv),
        &[],
        &plain,
        &mut tag,
    )
    .expect("Unable to encrypt data");

    /* assemble output */
    let mut data = BytesMut::new();
    data.put(&iv[..]);
    data.put(&cipher[..]);
    data.put(&tag[..]);

    /* write output */
    write_output(&opts.output, &data);
}
