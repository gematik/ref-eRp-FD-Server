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

use std::iter::once;

use async_trait::async_trait;
use resources::{Signature, SignatureFormat, SignatureType};

use crate::fhir::{
    decode::{decode_any, DataStream, Decode, DecodeError, DecodeStream, Fields},
    encode::{encode_any, DataStorage, Encode, EncodeError, EncodeStream},
};

use super::primitives::{
    decode_coding, decode_reference, encode_coding, encode_reference, CodeEx, CodingEx,
};

/* Decode */

#[async_trait(?Send)]
impl Decode for Signature {
    async fn decode<S>(stream: &mut DecodeStream<S>) -> Result<Self, DecodeError<S::Error>>
    where
        S: DataStream,
    {
        let mut fields = Fields::new(&["type", "when", "who", "targetFormat", "sigFormat", "data"]);

        stream.root("Signature").await?;

        let type_ = stream.decode(&mut fields, decode_coding).await?;
        let when = stream.decode(&mut fields, decode_any).await?;
        let who = stream.decode(&mut fields, decode_reference).await?;
        let target_format = stream
            .decode_opt::<Option<String>, _>(&mut fields, decode_any)
            .await?;
        let sig_format = stream
            .decode_opt::<Option<String>, _>(&mut fields, decode_any)
            .await?;
        let data = stream.decode(&mut fields, decode_any).await?;

        stream.end().await?;

        let format = match (target_format.as_deref(), sig_format.as_deref()) {
            (Some("application/fhir+xml"), Some("application/pkcs7-mime")) => {
                Some(SignatureFormat::Xml)
            }
            (Some("application/fhir+json"), Some("application/jose")) => {
                Some(SignatureFormat::Json)
            }
            (None, None) => None,
            (t, s) => {
                return Err(DecodeError::Custom {
                    message: format!(
                        "Signature format does not match (target = {:?}, signature = {:?})",
                        t, s
                    ),
                    path: stream.path().into(),
                })
            }
        };

        Ok(Signature {
            type_,
            when,
            who,
            format,
            data,
        })
    }
}

/* Encode */

impl Encode for &Signature {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        let (target_format, sig_format) = match self.format.as_ref() {
            Some(SignatureFormat::Xml) => {
                (Some("application/fhir+xml"), Some("application/pkcs7-mime"))
            }
            Some(SignatureFormat::Json) => {
                (Some("application/fhir+json"), Some("application/jose"))
            }
            Some(SignatureFormat::Unknown) => (None, None),
            None => (None, None),
        };

        stream
            .root("Signature")?
            .encode_vec("type", once(&self.type_), encode_coding)?
            .encode("when", &self.when, encode_any)?
            .encode("who", &self.who, encode_reference)?
            .encode_opt("targetFormat", target_format, encode_any)?
            .encode_opt("sigFormat", sig_format, encode_any)?
            .encode("data", &self.data, encode_any)?
            .end()?;

        Ok(())
    }
}

/* Misc */

impl CodingEx for SignatureType {
    type Code = Self;

    fn from_parts(code: Self::Code) -> Self {
        code
    }

    fn code(&self) -> &Self::Code {
        &self
    }

    fn display(&self) -> Option<&'static str> {
        match &self {
            Self::AuthorsSignature => Some("Author's Signature"),
            Self::CoauthorsSignature => Some("Coauthor's Signature"),
            Self::CoParticipantsSignature => Some("Co-participant's Signature"),
            Self::TranscriptionistRecorderSignature => Some("Transcriptionist/Recorder Signature"),
            Self::VerificationSignature => Some("Verification Signature"),
            Self::ValidationSignature => Some("Validation Signature"),
            Self::ConsentSignature => Some("Consent Signature"),
            Self::SignatureWitnessSignature => Some("Signature Witness Signature"),
            Self::EventWitnessSignature => Some("Event Witness Signature"),
            Self::IdentityWitnessSignature => Some("Identity Witness Signature"),
            Self::ConsentWitnessSignature => Some("Consent Witness Signature"),
            Self::InterpreterSignature => Some("Interpreter Signature"),
            Self::ReviewSignature => Some("Review Signature"),
            Self::SourceSignature => Some("Source Signature"),
            Self::AddendumSignature => Some("Addendum Signature"),
            Self::ModificationSignature => Some("Modification Signature"),
            Self::AdministrativeSignature => Some("Administrative (Error/Edit) Signature"),
            Self::TimestampSignature => Some("Timestamp Signature"),
        }
    }

    fn system() -> Option<&'static str> {
        Some("urn:iso-astm:E1762-95:2013")
    }
}

impl CodeEx for SignatureType {
    fn from_parts(value: String) -> Result<Self, String> {
        match value.as_str() {
            "1.2.840.10065.1.12.1.1" => Ok(SignatureType::AuthorsSignature),
            "1.2.840.10065.1.12.1.2" => Ok(SignatureType::CoauthorsSignature),
            "1.2.840.10065.1.12.1.3" => Ok(SignatureType::CoParticipantsSignature),
            "1.2.840.10065.1.12.1.4" => Ok(SignatureType::TranscriptionistRecorderSignature),
            "1.2.840.10065.1.12.1.5" => Ok(SignatureType::VerificationSignature),
            "1.2.840.10065.1.12.1.6" => Ok(SignatureType::ValidationSignature),
            "1.2.840.10065.1.12.1.7" => Ok(SignatureType::ConsentSignature),
            "1.2.840.10065.1.12.1.8" => Ok(SignatureType::SignatureWitnessSignature),
            "1.2.840.10065.1.12.1.9" => Ok(SignatureType::EventWitnessSignature),
            "1.2.840.10065.1.12.1.10" => Ok(SignatureType::IdentityWitnessSignature),
            "1.2.840.10065.1.12.1.11" => Ok(SignatureType::ConsentWitnessSignature),
            "1.2.840.10065.1.12.1.12" => Ok(SignatureType::InterpreterSignature),
            "1.2.840.10065.1.12.1.13" => Ok(SignatureType::ReviewSignature),
            "1.2.840.10065.1.12.1.14" => Ok(SignatureType::SourceSignature),
            "1.2.840.10065.1.12.1.15" => Ok(SignatureType::AddendumSignature),
            "1.2.840.10065.1.12.1.16" => Ok(SignatureType::ModificationSignature),
            "1.2.840.10065.1.12.1.17" => Ok(SignatureType::AdministrativeSignature),
            "1.2.840.10065.1.12.1.18" => Ok(SignatureType::TimestampSignature),
            _ => Err(value),
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::AuthorsSignature => "1.2.840.10065.1.12.1.1",
            Self::CoauthorsSignature => "1.2.840.10065.1.12.1.2",
            Self::CoParticipantsSignature => "1.2.840.10065.1.12.1.3",
            Self::TranscriptionistRecorderSignature => "1.2.840.10065.1.12.1.4",
            Self::VerificationSignature => "1.2.840.10065.1.12.1.5",
            Self::ValidationSignature => "1.2.840.10065.1.12.1.6",
            Self::ConsentSignature => "1.2.840.10065.1.12.1.7",
            Self::SignatureWitnessSignature => "1.2.840.10065.1.12.1.8",
            Self::EventWitnessSignature => "1.2.840.10065.1.12.1.9",
            Self::IdentityWitnessSignature => "1.2.840.10065.1.12.1.10",
            Self::ConsentWitnessSignature => "1.2.840.10065.1.12.1.11",
            Self::InterpreterSignature => "1.2.840.10065.1.12.1.12",
            Self::ReviewSignature => "1.2.840.10065.1.12.1.13",
            Self::SourceSignature => "1.2.840.10065.1.12.1.14",
            Self::AddendumSignature => "1.2.840.10065.1.12.1.15",
            Self::ModificationSignature => "1.2.840.10065.1.12.1.16",
            Self::AdministrativeSignature => "1.2.840.10065.1.12.1.17",
            Self::TimestampSignature => "1.2.840.10065.1.12.1.18",
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::convert::TryInto;
    use std::fs::read_to_string;
    use std::str::from_utf8;

    use crate::fhir::{
        decode::{tests::load_stream, JsonDecode, XmlDecode},
        encode::{JsonEncode, XmlEncode},
    };

    use super::super::super::tests::{trim_json_str, trim_xml_str};

    #[tokio::test]
    async fn test_decode_json() {
        let mut stream = load_stream("./examples/signature.json");

        let actual = stream.json::<Signature>().await.unwrap();
        let expected = test_signature();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_decode_xml() {
        let mut stream = load_stream("./examples/signature.xml");

        let actual = stream.xml::<Signature>().await.unwrap();
        let expected = test_signature();

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_encode_json() {
        let value = test_signature();

        let actual = (&value).json().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/signature.json").unwrap();

        assert_eq!(trim_json_str(&actual), trim_json_str(&expected));
    }

    #[tokio::test]
    async fn test_encode_xml() {
        let value = test_signature();

        let actual = (&value).xml().unwrap();
        let actual = from_utf8(&actual).unwrap();
        let expected = read_to_string("./examples/signature.xml").unwrap();

        assert_eq!(trim_xml_str(&actual), trim_xml_str(&expected));
    }

    pub fn test_signature() -> Signature {
        Signature {
            type_: SignatureType::AuthorsSignature,
            when: "2020-03-20T07:31:34.328+00:00".try_into().unwrap(),
            who: "https://prescriptionserver.telematik/Device/ErxService".into(),
            data: "MIII FQYJ KoZI hvcN AQcC oIII BjCC CAIC AQEx DzAN Bglg hkgB ZQME AgEF ADAL"
                .into(),
            format: Some(SignatureFormat::Json),
        }
    }
}
