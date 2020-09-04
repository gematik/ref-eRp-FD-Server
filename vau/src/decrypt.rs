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

use bytes::BytesMut;
use openssl::{
    bn::BigNumContext,
    derive::Deriver,
    ec::{EcGroup, EcKey, EcPoint},
    hash::MessageDigest,
    hkdf::{Hkdf, Mode},
    nid::Nid,
    pkey::{PKey, Private},
    symm::{decrypt_aead, Cipher},
};

use crate::Error;

pub struct Decrypter {
    pkey: PKey<Private>,
    context: BigNumContext,
}

impl Decrypter {
    pub fn new(pkey: EcKey<Private>) -> Result<Self, Error> {
        let context = BigNumContext::new()?;
        let pkey = PKey::from_ec_key(pkey)?;

        Ok(Self { pkey, context })
    }

    pub fn decrypt(&mut self, mut payload: BytesMut) -> Result<BytesMut, Error> {
        const LEN_VERSION: usize = 1;
        const LEN_COORDINATE: usize = 32;
        const LEN_IV: usize = 12;
        const LEN_TAG: usize = 16;
        const LEN_TOTAL: usize = LEN_VERSION + 2 * LEN_COORDINATE + LEN_IV + LEN_TAG;

        if payload.len() < LEN_TOTAL {
            return Err(Error::DecodeError);
        }

        let payload = payload.as_mut();
        if payload[0] != 0x01 {
            return Err(Error::DecodeError);
        }

        payload[0] = 0x04;

        let (client_public_key, payload) = payload.split_at(LEN_VERSION + 2 * LEN_COORDINATE);
        let (iv, payload) = payload.split_at(LEN_IV);
        let (cipher, tag) = payload.split_at(payload.len() - LEN_TAG);

        let group = &*BRAINPOOL_P256_R1;

        let client_public_key = EcPoint::from_bytes(group, &client_public_key, &mut self.context)?;
        let client_public_key = EcKey::from_public_key(group, &client_public_key)?;
        let client_public_key = PKey::from_ec_key(client_public_key)?;

        let mut deriver = Deriver::new(&self.pkey)?;
        deriver.set_peer(&client_public_key)?;

        let shared_secret = deriver.derive_to_vec()?;
        let aes_key = Hkdf::new(MessageDigest::sha256())?
            .set_mode(Mode::ExtractAndExpand)?
            .set_info(Some(b"ecies-vau-transport"))?
            .set_secret(&shared_secret)?
            .derive(LEN_TAG)?;

        let data = decrypt_aead(Cipher::aes_128_gcm(), &aes_key, Some(&iv), &[], cipher, tag)
            .map_err(|_| Error::DecodeError)?;
        let data = BytesMut::from(&data[..]);

        Ok(data)
    }
}

lazy_static! {
    pub static ref BRAINPOOL_P256_R1: EcGroup =
        EcGroup::from_curve_name(Nid::from_raw(927)).unwrap();
}

#[cfg(test)]
mod test {
    use super::*;

    use std::str::from_utf8;

    use super::super::misc::hex_decode;

    #[test]
    fn decrypt() {
        let payload = "01 754e548941e5cd073fed6d734578a484be9f0bbfa1b6fa3168ed7ffb22878f0f 9aef9bbd932a020d8828367bd080a3e72b36c41ee40c87253f9b1b0beb8371bf 257db4604af8ae0dfced37ce 86c2b491c7a8309e750b4e6e307219863938c204dfe85502ee0a";
        let payload = payload.replace(" ", "");
        let payload = hex_decode(&payload).unwrap();
        let payload = BytesMut::from(&payload[..]);

        let vau_key = r#"
-----BEGIN EC PARAMETERS-----
BgkrJAMDAggBAQc=
-----END EC PARAMETERS-----
-----BEGIN EC PRIVATE KEY-----
MHgCAQEEIKVKVoW4D3H9Xr7pFlmvqYyEfFyGTiM1hEFGZ1r8WV48oAsGCSskAwMC
CAEBB6FEA0IABIY0ISgw2tRXygUwXmaHE0FmucIaZf/r9VX05137BIiIZuS2hDYk
y9pDyX6omWi8Qf1TV2+CwD76fWAbn6ysKyk=
-----END EC PRIVATE KEY-----
"#;
        let vau_key = EcKey::private_key_from_pem(vau_key.as_bytes()).unwrap();

        let mut decrypter = Decrypter::new(vau_key).unwrap();
        let data = decrypter.decrypt(payload).unwrap();
        let data = from_utf8(&data).unwrap();

        assert_eq!("Hallo Test", data);
    }
}
