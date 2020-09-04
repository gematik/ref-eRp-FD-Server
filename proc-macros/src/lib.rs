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

#[macro_use]
extern crate quote;

mod capability_statement;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn capability_statement(attribs: TokenStream, tokens: TokenStream) -> TokenStream {
    capability_statement::capability_statement(attribs, tokens)
}

#[proc_macro_attribute]
pub fn capability_statement_resource(attribs: TokenStream, tokens: TokenStream) -> TokenStream {
    capability_statement::resource(attribs, tokens)
}
