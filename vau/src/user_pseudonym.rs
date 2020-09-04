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

use std::sync::Arc;
use std::time::{Duration, Instant};

use openssl::{
    error::ErrorStack,
    hash::MessageDigest,
    memcmp::eq,
    pkey::{PKey, Private},
    sign::Signer,
    symm::Cipher,
};
use rand::random;
use tokio::sync::Mutex;

use crate::Error;

use super::misc::{hex_decode, hex_encode};

#[derive(Clone, Default)]
pub struct UserPseudonymGenerator(Arc<Mutex<Inner>>);

struct Inner {
    pkey: Option<PKey<Private>>,
    pkey_timeout: Instant,
}

impl UserPseudonymGenerator {
    pub async fn generate(&self) -> Result<String, Error> {
        let key = self.0.lock().await.key()?;
        let pnp: [u8; 16] = random();

        let mut signer = Signer::new(MessageDigest::sha256(), &key)?;
        signer.update(&pnp)?;

        let cmac = signer.sign_to_vec()?;
        let cmac = hex_encode(&cmac);
        let pnp = hex_encode(&pnp);
        let pn = format!("{}-{}", pnp, cmac);

        Ok(pn)
    }

    pub async fn verify(&self, pn: &str) -> bool {
        macro_rules! some {
            ($e:expr) => {
                match $e {
                    Some(x) => x,
                    None => return false,
                }
            };
        }

        let mut it = pn.split('-');

        let pnp = some!(it.next());
        let pnp = some!(hex_decode(pnp).ok());

        let actual_cmac = some!(it.next());
        let actual_cmac = some!(hex_decode(actual_cmac).ok());

        let key = some!(self.0.lock().await.key().ok());
        let mut signer = some!(Signer::new(MessageDigest::sha256(), &key).ok());

        if signer.update(&pnp).is_err() {
            return false;
        }

        let expected_cmac = some!(signer.sign_to_vec().ok());

        eq(&actual_cmac, &expected_cmac)
    }
}

impl Inner {
    fn key(&mut self) -> Result<PKey<Private>, ErrorStack> {
        if self.pkey_timeout < Instant::now() || self.pkey.is_none() {
            let (key, timeout) = generate_key()?;

            self.pkey_timeout = timeout;
            self.pkey = Some(key);
        }

        Ok(self.pkey.as_ref().unwrap().clone())
    }
}

impl Default for Inner {
    fn default() -> Self {
        Self {
            pkey: None,
            pkey_timeout: Instant::now(),
        }
    }
}

fn generate_key() -> Result<(PKey<Private>, Instant), ErrorStack> {
    let key: [u8; 16] = random();

    let key = PKey::cmac(&KEY_CIPHER, &key)?;
    let timeout = Instant::now() + Duration::from_secs(10 * 24 * 60 * 60);

    Ok((key, timeout))
}

lazy_static! {
    static ref KEY_CIPHER: Cipher = Cipher::aes_128_cbc();
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn generate_and_verify() {
        let gen = UserPseudonymGenerator::default();
        let np = gen.generate().await.unwrap();
        let ret = gen.verify(&np).await;

        assert_eq!(true, ret);
    }
}
