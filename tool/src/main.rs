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

mod aes_decrypt;
mod aes_encrypt;
mod create_access_token;
mod misc;
mod pkcs7_sign;
mod pkcs7_verify;
mod vau_decrypt;
mod vau_encrypt;

use structopt::StructOpt;

use aes_decrypt::{execute as aes_decrypt, Opts as AesDecryptOpts};
use aes_encrypt::{execute as aes_encrypt, Opts as AesEncryptOpts};
use create_access_token::{execute as create_access_token, Opts as CreateAccessTokenOpts};
use pkcs7_sign::{execute as pkcs7_sign, Opts as Pkcs7SignOpts};
use pkcs7_verify::{execute as pkcs7_verify, Opts as Pkcs7VerifyOpts};
use vau_decrypt::{execute as vau_decrypt, Opts as VauDecryptOpts};
use vau_encrypt::{execute as vau_encrypt, Opts as VauEncryptOpts};

fn main() {
    let command = Command::from_args();

    match command {
        Command::AesDecrypt(opts) => aes_decrypt(opts),
        Command::AesEncrypt(opts) => aes_encrypt(opts),
        Command::CreateAccessToken(opts) => create_access_token(opts),
        Command::Pkcs7Sign(opts) => pkcs7_sign(opts),
        Command::Pkcs7Verify(opts) => pkcs7_verify(opts),
        Command::VauDecrypt(opts) => vau_decrypt(opts),
        Command::VauEncrypt(opts) => vau_encrypt(opts),
    }
}

#[derive(StructOpt)]
enum Command {
    AesDecrypt(AesDecryptOpts),
    AesEncrypt(AesEncryptOpts),
    CreateAccessToken(CreateAccessTokenOpts),
    Pkcs7Sign(Pkcs7SignOpts),
    Pkcs7Verify(Pkcs7VerifyOpts),
    VauDecrypt(VauDecryptOpts),
    VauEncrypt(VauEncryptOpts),
}
