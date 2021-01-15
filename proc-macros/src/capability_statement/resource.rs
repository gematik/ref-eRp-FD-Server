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

use std::mem::take;

use proc_macro::{token_stream::IntoIter, Delimiter, Group, TokenStream, TokenTree};
use proc_macro2::TokenStream as TokenStream2;

use super::misc::{parse_key_value_list, Attrib, Interaction, Operation};

pub fn resource(attribs: TokenStream, tokens: TokenStream) -> TokenStream {
    let res = ResourceMacro::new(attribs, tokens).and_then(|x| x.execute());

    match res {
        Ok(stream) => stream,
        Err(s) => panic!(
            "Error in 'capability_statement_resource' macro: {}! {}",
            s, EXAMPLE_MSG
        ),
    }
}

struct ResourceMacro {
    result: TokenStream,
    tokens: IntoIter,
    name: Option<String>,
    routes: Vec<Route>,
    type_: TokenStream2,
    profile: TokenStream2,
    supported_profiles: Option<TokenStream2>,
}

struct Route {
    method: TokenStream2,
    cfg: Option<TokenStream2>,
    operations: Vec<Operation>,
    interactions: Vec<Interaction>,
}

impl ResourceMacro {
    fn new(attribs: TokenStream, tokens: TokenStream) -> Result<Self, String> {
        let mut type_ = None;
        let mut profile = None;
        let mut supported_profiles = None;

        parse_key_value_list(attribs.into_iter(), |key, value| {
            match key {
                "type" => type_ = Some(value),
                "profile" => profile = Some(value),
                "supported_profiles" => supported_profiles = Some(value),
                s => return Err(format!("Unexpected value: {}", s)),
            }

            Ok(())
        })?;

        Ok(Self {
            tokens: tokens.into_iter(),
            result: TokenStream::new(),
            name: None,
            routes: Vec::new(),
            type_: type_.ok_or("Missing the 'type' attribute")?.into(),
            profile: profile.ok_or("Missing the 'profile' attribute")?.into(),
            supported_profiles: supported_profiles.map(Into::into),
        })
    }

    fn execute(mut self) -> Result<TokenStream, String> {
        match self.next_token() {
            Some(TokenTree::Ident(ident)) if ident.to_string() == "impl" => (),
            _ => return Err("Expected identifier 'impl'".into()),
        }

        match self.next_token() {
            Some(TokenTree::Ident(ident)) => self.name = Some(ident.to_string()),
            _ => return Err("Expected identifier for struct".into()),
        }

        let group = match self.tokens.next() {
            Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Brace => group,
            _ => return Err("Expected implementation group".into()),
        };

        if let Some(t) = self.tokens.next() {
            return Err(format!("Unexpected token: {}", t));
        }

        let mut code = self.handle_inner(group)?;
        code.extend(self.generate_configure_routes());
        code.extend(self.generate_update_capability_statement());

        self.result
            .extend(TokenStream::from(TokenTree::Group(Group::new(
                Delimiter::Brace,
                code,
            ))));
        self.result.extend(generate_helper());

        Ok(self.result)
    }

    fn handle_inner(&mut self, group: Group) -> Result<TokenStream, String> {
        let mut cfg = None;
        let mut operations = Vec::new();
        let mut interactions = Vec::new();

        let mut ret = TokenStream::new();
        let mut it = group.stream().into_iter();
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
                        Attrib::Operation(o) => operations.push(o),
                        Attrib::Interaction(i) => interactions.push(i),
                        Attrib::Unknown => {
                            ret.extend(TokenStream::from(TokenTree::Punct(punct.clone())));
                            ret.extend(TokenStream::from(TokenTree::Group(group)));
                        }
                        _ => return Err(format!("Unexpected attribute: {}", group)),
                    }
                }
                TokenTree::Ident(ident) if ident.to_string() == "fn" => {
                    let token = it.next().ok_or("Expected method name after 'fn' keyword")?;

                    ret.extend(TokenStream::from(TokenTree::Ident(ident)));
                    ret.extend(TokenStream::from(token.clone()));

                    match token {
                        TokenTree::Ident(ident) => {
                            let token = TokenTree::Ident(ident);
                            let method = TokenStream::from(token.clone()).into();
                            let cfg = cfg.take().map(Into::into);

                            self.routes.push(Route {
                                cfg,
                                method,
                                operations: take(&mut operations),
                                interactions: take(&mut interactions),
                            });
                        }
                        _ => return Err("Expected method name after 'fn' keyword".into()),
                    }
                }
                token => ret.extend(TokenStream::from(token)),
            };
        }

        Ok(ret)
    }

    fn generate_configure_routes(&self) -> TokenStream {
        let routes = self.routes.iter().map(|route| {
            let cfg = &route.cfg;
            let method = &route.method;

            quote! {
                #cfg
                self.#method(cfg);
            }
        });

        let configure_routes = quote! {
            pub fn configure_routes(&self, cfg: &mut actix_web::web::ServiceConfig) {
                #(#routes)*
            }
        };

        configure_routes.into()
    }

    fn generate_update_capability_statement(&self) -> TokenStream {
        let type_ = &self.type_;
        let profile = &self.profile;
        let supported_profiles = match &self.supported_profiles {
            Some(supported_profiles) => {
                quote! { #supported_profiles.iter().map(|s| (*s).into()).collect() }
            }
            None => quote! { Vec::new() },
        };

        let update_resource = self.routes.iter().map(|route| {
            let cfg = &route.cfg;
            let method = &route.method;
            let method = quote! {
                stringify!(#method)
            };

            let operations = route.operations.iter().map(|operation| {
                let name = operation.name.as_ref().unwrap_or(&method);
                let definition = &operation.definition;

                quote! {
                    #cfg
                    update_resource_operation(res, #name, #definition);
                }
            });
            let interactions = route.interactions.iter().map(|interaction| {
                let name = &interaction.name;

                quote! {
                    #cfg
                    update_resource_interaction(res, #name);
                }
            });

            quote! {
                #(#operations)*
                #(#interactions)*
            }
        });

        let update_capability_statement = quote! {
            pub fn update_capability_statement(
                &self,
                capability_statement: &mut resources::CapabilityStatement)
            {
                if capability_statement.rest.is_empty() {
                    let rest = resources::capability_statement::Rest {
                        mode: resources::capability_statement::Mode::Server,
                        resource: Vec::new(),
                    };

                    capability_statement.rest.push(rest);
                }

                let rest = &mut capability_statement.rest[0];

                for res in &mut rest.resource {
                    if res.type_ == #type_ && res.profile == #profile {
                        self.update_resource(res);
                        return;
                    }
                }

                let mut res = resources::capability_statement::Resource {
                    type_: #type_.into(),
                    profile: #profile.into(),
                    supported_profiles: #supported_profiles,
                    operation: Vec::new(),
                    interaction: Vec::new(),
                };

                self.update_resource(&mut res);

                rest.resource.push(res);
            }

            fn update_resource(&self, res: &mut resources::capability_statement::Resource) {
                #(#update_resource)*
            }
        };

        update_capability_statement.into()
    }

    fn next_token(&mut self) -> Option<TokenTree> {
        let ret = self.tokens.next();

        if let Some(token) = &ret {
            self.result.extend(TokenStream::from(token.clone()));
        }

        ret
    }
}

fn generate_helper() -> TokenStream {
    let helper = quote! {
        fn update_resource_operation(
            res: &mut resources::capability_statement::Resource,
            name: &str,
            definition: &str,
        ) {
            for op in &mut res.operation {
                if op.name == name {
                    op.definition = definition.into();
                    return;
                }
            }
            let op = resources::capability_statement::Operation {
                name: name.into(),
                definition: definition.into(),
            };
            res.operation.push(op);
        }

        fn update_resource_interaction(
            res: &mut resources::capability_statement::Resource,
            interaction: resources::capability_statement::Interaction,
        ) {
            for int in &res.interaction {
                if *int == interaction {
                    return;
                }
            }
            res.interaction.push(interaction);
        }
    };

    helper.into()
}

const EXAMPLE_MSG: &str = r##"
    #[capability_statement_resource(type = Type::Task, profile = RESOURCE_PROFILE_TASK)]
    impl TaskRoutes {
        #[operation(definition = OPERATION_TASK_CREATE)]
        fn create(&self, cfg: &mut ServiceConfig) {
            ...
        }

        #[operation(definition = OPERATION_TASK_ACTIVATE)]
        fn activate(&self, cfg: &mut ServiceConfig) {
            ...
        }

        #[interaction(Interaction::Read)]
        fn get(&self, cfg: &mut ServiceConfig) {
            ...
        }
    }"##;
