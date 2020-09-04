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

use bytes::{BufMut, BytesMut};
use openssl::symm::{encrypt_aead, Cipher};
use rand::random;

use crate::Error;

#[derive(Default)]
pub struct Encrypter;

impl Encrypter {
    pub fn encrypt(&self, key: &[u8], res: BytesMut) -> Result<BytesMut, Error> {
        let iv: [u8; 12] = random();
        let mut tag = [0; 16];
        let cipher = encrypt_aead(Cipher::aes_128_gcm(), key, Some(&iv), &[], &res, &mut tag)?;

        let mut ret = BytesMut::new();

        ret.reserve(iv.len() + cipher.len() + tag.len());
        ret.put(&iv[..]);
        ret.put(&cipher[..]);
        ret.put(&tag[..]);

        Ok(ret)
    }
}
