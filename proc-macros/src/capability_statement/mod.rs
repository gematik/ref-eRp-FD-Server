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

mod misc;
mod resource;

use proc_macro::{token_stream::IntoIter, Delimiter, Group, TokenStream, TokenTree};
use proc_macro2::TokenStream as TokenStream2;

pub use resource::resource;

use misc::{parse_key_value_list, Attrib};

pub fn capability_statement(attribs: TokenStream, tokens: TokenStream) -> TokenStream {
    let res = CapabilityStatementMacro::new(attribs, tokens).and_then(|x| x.execute());

    match res {
        Ok(stream) => stream,
        Err(s) => panic!(
            "Error in 'capability_statement' macro: {}! {}",
            s, EXAMPLE_MSG
        ),
    }
}

struct CapabilityStatementMacro {
    result: TokenStream,
    tokens: IntoIter,
    init: TokenStream2,
    handler: TokenStream2,
    resources: Vec<Resource>,
}

struct Resource {
    field: TokenStream2,
    cfg: Option<TokenStream2>,
}

impl CapabilityStatementMacro {
    fn new(attribs: TokenStream, tokens: TokenStream) -> Result<Self, String> {
        let mut init = None;
        let mut handler = None;

        parse_key_value_list(attribs.into_iter(), |key, value| {
            match key {
                "init" => init = Some(value),
                "handler" => handler = Some(value),
                s => return Err(format!("Unexpected value: {}", s)),
            }

            Ok(())
        })?;

        Ok(Self {
            tokens: tokens.into_iter(),
            result: TokenStream::new(),
            init: init.ok_or("Missing the 'init' attribute")?.into(),
            handler: handler.ok_or("Missing the 'handler' attribute")?.into(),
            resources: Vec::new(),
        })
    }

    fn execute(mut self) -> Result<TokenStream, String> {
        while let Some(token) = self.next_token() {
            match token {
                TokenTree::Ident(ident) if ident.to_string() == "struct" => break,
                _ => (),
            }
        }

        let name = match self.next_token() {
            Some(TokenTree::Ident(ident)) => TokenStream::from(TokenTree::Ident(ident)).into(),
            _ => return Err("Expected identifier for struct!".into()),
        };

        let group = match self.tokens.next() {
            Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Brace => group,
            _ => return Err("Expected implementation group".into()),
        };

        if let Some(t) = self.tokens.next() {
            return Err(format!("Unexpected token: {}", t));
        }

        let mut code = self.handle_inner(group)?;
        code.extend(TokenStream::from(quote! {
            __capability_statement: resources::CapabilityStatement,
        }));

        self.result
            .extend(TokenStream::from(TokenTree::Group(Group::new(
                Delimiter::Brace,
                code,
            ))));

        self.generate(name);

        Ok(self.result)
    }

    fn handle_inner(&mut self, group: Group) -> Result<TokenStream, String> {
        let mut ret = TokenStream::new();
        let mut it = group.stream().into_iter();

        let mut cfg = None;
        let mut resource = None;

        while let Some(token) = it.next() {
            match token {
                TokenTree::Punct(punct) if punct.as_char() == '#' => {
                    let group = match it.next() {
                        Some(TokenTree::Group(group)) => group,
                        _ => return Err("Expected group inside attribute".into()),
                    };

                    match Attrib::new(&group)? {
                        Attrib::Cfg(s) => {
                            if cfg.is_some() {
                                return Err("Attribute 'cfg' is already set".into());
                            }

                            ret.extend(s.clone());

                            cfg = Some(s);
                        }
                        Attrib::Resource(r) => {
                            if resource.is_some() {
                                return Err("Attribute 'resource' is already set".into());
                            }

                            resource = Some(r);
                        }
                        Attrib::Unknown => {
                            ret.extend(TokenStream::from(TokenTree::Punct(punct.clone())));
                            ret.extend(TokenStream::from(TokenTree::Group(group)));
                        }
                        _ => return Err(format!("Unexpected attribute: {}", group)),
                    }
                }
                TokenTree::Ident(ident) => {
                    let token = TokenTree::Ident(ident);
                    let field = TokenStream::from(token);
                    let cfg = cfg.take().map(Into::into);

                    if resource.take().is_some() {
                        self.resources.push(Resource {
                            cfg,
                            field: field.clone().into(),
                        })
                    }

                    ret.extend(field);
                }
                token => ret.extend(TokenStream::from(token)),
            };
        }

        Ok(ret)
    }

    fn generate(&mut self, name: TokenStream2) {
        let configure_routes = self.resources.iter().map(|r| {
            let cfg = &r.cfg;
            let field = &r.field;

            quote! {
                #cfg
                self.#field.configure_routes(cfg);
            }
        });

        let update_capability_statement = self.resources.iter().map(|r| {
            let cfg = &r.cfg;
            let field = &r.field;

            quote! {
                #cfg
                self.#field.update_capability_statement(&mut self.__capability_statement);
            }
        });

        let default = self.resources.iter().map(|r| {
            let field = &r.field;

            quote! {
                #field: Default::default(),
            }
        });

        let init = &self.init;
        let handler = &self.handler;
        let code = quote! {
            impl #name {
                pub fn capability_statement(&self) -> &resources::CapabilityStatement {
                    &self.__capability_statement
                }

                pub fn configure_routes(&self, cfg: &mut actix_web::web::ServiceConfig) {
                    let handler = actix_web::web::get().to(#handler);
                    let resource = actix_web::web::resource("/metadata").route(handler);

                    cfg.service(resource);

                    #(#configure_routes)*
                }

                fn update_capability_statement(&mut self)
                {
                    #(#update_capability_statement)*
                }
            }

            impl Default for #name {
                fn default() -> Self {
                    let mut ret = Self {
                        __capability_statement: #init(),
                        #(#default)*
                    };

                    ret.update_capability_statement();

                    ret
                }
            }
        };

        let code = TokenStream::from(code);
        self.result.extend(code);
    }

    fn next_token(&mut self) -> Option<TokenTree> {
        let ret = self.tokens.next();

        if let Some(token) = &ret {
            self.result.extend(TokenStream::from(token.clone()));
        }

        ret
    }
}

const EXAMPLE_MSG: &str = r##"
    #[capability_statement(init = capability_statement_create, handler = capability_statement_get)]
    struct Routes {
        #[resource]
        task: TaskRoutes,
    }"##;
