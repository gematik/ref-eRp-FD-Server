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

use std::iter::FromIterator;

use proc_macro::{Group, Punct, Spacing, TokenStream, TokenTree};
use proc_macro2::TokenStream as TokenStream2;

pub enum Attrib {
    Cfg(TokenStream),
    Operation(Operation),
    Interaction(Interaction),
    Resource(Resource),
    Unknown,
}

pub struct Operation {
    pub name: Option<TokenStream2>,
    pub definition: TokenStream2,
}

pub struct Interaction {
    pub name: TokenStream2,
}

pub struct Resource;

impl Attrib {
    pub fn new(group: &Group) -> Result<Self, String> {
        let mut it = group.stream().into_iter();

        let ident = match it.next() {
            Some(TokenTree::Ident(ident)) => ident,
            _ => return Ok(Attrib::Unknown),
        };

        let attrib = match ident.to_string().as_str() {
            "cfg" => {
                let s = TokenStream::from_iter(
                    vec![
                        TokenTree::Punct(Punct::new('#', Spacing::Joint)),
                        TokenTree::Group(group.clone()),
                    ]
                    .into_iter(),
                );

                Attrib::Cfg(s)
            }
            "resource" => Attrib::Resource(Resource::new(it.next())?),
            "operation" => Attrib::Operation(Operation::new(it.next())?),
            "interaction" => Attrib::Interaction(Interaction::new(it.next())?),
            _ => Attrib::Unknown,
        };

        Ok(attrib)
    }
}

impl Operation {
    pub fn new(token: Option<TokenTree>) -> Result<Self, String> {
        let group = match token {
            Some(TokenTree::Group(group)) => group,
            _ => return Err("Expected values for attribute 'interaction'".into()),
        };

        let mut name = None;
        let mut definition = None;

        let it = group.stream().into_iter();

        parse_key_value_list(it, |key, value| {
            match key {
                "name" => name = Some(value),
                "definition" => definition = Some(value),
                s => return Err(format!("Attribute 'operation' has unexpected value: {}", s)),
            }

            Ok(())
        })?;

        Ok(Self {
            name: name.map(Into::into),
            definition: definition
                .ok_or("Attribute 'operation' expect value for 'definition'")?
                .into(),
        })
    }
}

impl Interaction {
    pub fn new(token: Option<TokenTree>) -> Result<Self, String> {
        let group = match token {
            Some(TokenTree::Group(group)) => group,
            _ => return Err("Expected values for attribute 'interaction'".into()),
        };

        Ok(Self {
            name: group.stream().into(),
        })
    }
}

impl Resource {
    pub fn new(_token: Option<TokenTree>) -> Result<Self, String> {
        Ok(Self)
    }
}

pub fn parse_key_value_list<I, F>(mut it: I, mut f: F) -> Result<(), String>
where
    I: Iterator<Item = TokenTree>,
    F: FnMut(&str, TokenStream) -> Result<(), String>,
{
    while let Some(ident) = it.next() {
        let ident = match ident {
            TokenTree::Ident(ident) => ident.to_string(),
            _ => return Err("Attribute expected identifier".into()),
        };

        match it.next() {
            Some(TokenTree::Punct(p)) if p.as_char() == '=' => (),
            _ => return Err("Attribute expected '=' after identifier".into()),
        }

        let mut value = TokenStream::new();
        for token in &mut it {
            match token {
                TokenTree::Punct(p) if p.as_char() == ',' => break,
                token => value.extend(TokenStream::from(token)),
            }
        }

        f(ident.as_str(), value)?;
    }

    Ok(())
}
