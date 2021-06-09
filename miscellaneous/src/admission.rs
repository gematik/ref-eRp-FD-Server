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

use std::str::FromStr;

use asn1::{
    parse, write, Explicit, ObjectIdentifier, ParseError, PrintableString, Sequence, SequenceWriter,
};

#[derive(Debug)]
pub struct Admission {
    pub professions: Vec<Profession>,
}

#[derive(Debug)]
pub enum Profession {
    Arzt,
    Zahnarzt,
    Any(ObjectIdentifier<'static>),
}

impl Admission {
    pub fn from_der(der: &[u8]) -> Result<Self, ParseError> {
        let professions = parse(der, |d| {
            d.read_element::<Sequence>()?.parse(|d| {
                let _names = d.read_element::<Explicit<Sequence, 4>>()?;
                let seq = d.read_element::<Sequence>()?;
                let seq = read_sequence(seq)?;
                let seq = read_sequence(seq)?;

                read_professions(seq)
            })
        })?;

        Ok(Self { professions })
    }

    pub fn to_der(&self) -> Vec<u8> {
        write(|w| {
            w.write_element(&SequenceWriter::new(&|w| {
                w.write_element(&Explicit::<_, 4>::new(SequenceWriter::new(&|w| {
                    w.write_element(&PrintableString::new("TODO: This sequence should contain the common names of the organization (countryName, organizationName, ...)").unwrap());
                })));
                w.write_element(&SequenceWriter::new(&|w| {
                    w.write_element(&SequenceWriter::new(&|w| {
                        w.write_element(&SequenceWriter::new(&|w| {
                            for profession in &self.professions {
                                w.write_element(&SequenceWriter::new(&|w| {
                                    w.write_element(&SequenceWriter::new(&|w| {
                                        w.write_element(&PrintableString::new("TODO: This sequence should contain the profession as UTF8 string").unwrap());
                                    }));
                                    w.write_element(&SequenceWriter::new(&|w| match &profession {
                                        Profession::Arzt => w.write_element(&*OID_ARZT),
                                        Profession::Zahnarzt => w.write_element(&*OID_ZAHNARZT),
                                        Profession::Any(oid) => w.write_element(oid),
                                    }));
                                    w.write_element(&PrintableString::new("TODO: This sequence should contain the identifier of the organization").unwrap());
                                }));
                            }
                        }));
                    }));
                }));
            }));
        })
    }
}

impl FromStr for Profession {
    type Err = ();

    fn from_str(oid: &str) -> Result<Self, Self::Err> {
        Ok(Self::Any(ObjectIdentifier::from_string(oid).ok_or(())?))
    }
}

fn read_professions(seq: Sequence) -> Result<Vec<Profession>, ParseError> {
    seq.parse(|d| {
        let mut professions: Vec<Profession> = Vec::new();

        while !d.is_empty() {
            let profession = d.read_element::<Sequence>()?;
            let profession = read_profession(profession)?;

            if let Some(profession) = profession {
                professions.push(profession);
            }
        }

        Ok(professions)
    })
}

fn read_profession(seq: Sequence) -> Result<Option<Profession>, ParseError> {
    seq.parse(|d| {
        let _profession = d.read_element::<Sequence>()?;
        let oid = d.read_element::<Sequence>()?;
        let oid = read_oid(oid)?;
        let _ident = d.read_element::<PrintableString>()?;

        if oid == *OID_ARZT {
            Ok(Some(Profession::Arzt))
        } else if oid == *OID_ZAHNARZT {
            Ok(Some(Profession::Zahnarzt))
        } else {
            Ok(None)
        }
    })
}

fn read_sequence(seq: Sequence) -> Result<Sequence, ParseError> {
    seq.parse(|d| d.read_element::<Sequence>())
}

fn read_oid(seq: Sequence) -> Result<ObjectIdentifier, ParseError> {
    seq.parse(|d| d.read_element::<ObjectIdentifier>())
}

lazy_static! {
    static ref OID_ARZT: ObjectIdentifier<'static> =
        ObjectIdentifier::from_string("1.2.276.0.76.4.30").unwrap();
    static ref OID_ZAHNARZT: ObjectIdentifier<'static> =
        ObjectIdentifier::from_string("1.2.276.0.76.4.31").unwrap();
}
