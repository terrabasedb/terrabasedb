/*
 * Created on Sun Sep 13 2020
 *
 * This file is a part of Skytable
 * Skytable (formerly known as TerrabaseDB or Skybase) is a free and open-source
 * NoSQL database written by Sayan Nandan ("the Author") with the
 * vision to provide flexibility in data modelling without compromising
 * on performance, queryability or scalability.
 *
 * Copyright (c) 2020, Sayan Nandan <ohsayan@outlook.com>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 *
*/

#![deny(unused_crate_dependencies)]
#![deny(unused_imports)]

//! A library containing a collection of custom derives used by Skytable
//!
//! ## Ghost values
//! We extensively use jargon like 'Ghost values'...but what exactly are they?
//! Ghost values are variables which are provided by the compiler macros, i.e the
//! _proc macros_. These values are just like normal variables except for the fact
//! that they aren't explicitly declared in code, and should be used directly. Make
//! sure that you don't overwrite a macro provided variable!
//!
//! ### Macros and ghost values
//! - `#[dbtest]`:
//!     - `con` - `skytable::AsyncConnection`
//!     - `query` - `skytable::Query`
//!

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{self};

/// This parses a function within a `dbtest` module
///
/// This accepts an `async` function and returns a non-`async` version of it - by
/// making the body of the function use the `tokio` runtime
fn parse_dbtest(mut input: syn::ItemFn) -> Result<TokenStream, syn::Error> {
    let sig = &mut input.sig;
    let fname = sig.ident.to_string();
    let body = &input.block;
    let attrs = &input.attrs;
    let vis = &input.vis;
    let header = quote! {
        #[::core::prelude::v1::test]
    };
    if sig.asyncness.is_none() {
        let msg = "`dbtest` functions need to be async";
        return Err(syn::Error::new_spanned(sig.fn_token, msg));
    }
    sig.asyncness = None;
    let body = quote! {
        let mut con = skytable::AsyncConnection::new("127.0.0.1", 2003).await.unwrap();
        let mut query = skytable::Query::new();
        #body
        {
            let mut __flush__ = skytable::Query::from("flushdb");
            std::assert_eq!(
                con.run_simple_query(&__flush__).await.unwrap(),
                skytable::Response::Item(skytable::Element::RespCode(skytable::RespCode::Okay))
            );
        }
    };
    let result = quote! {
        #header
        #(#attrs)*
        #vis #sig {
            tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .thread_name(#fname)
            .thread_stack_size(3 * 1024 * 1024)
            .enable_all()
            .build()
            .unwrap()
            .block_on(async { #body });
        }
    };
    Ok(result.into())
}

/// This function checks if the current function is eligible to be a test
fn parse_test_sig(input: syn::ItemFn) -> TokenStream {
    for attr in &input.attrs {
        if attr.path.is_ident("test") {
            let msg = "second test attribute is supplied";
            return syn::Error::new_spanned(&attr, msg)
                .to_compile_error()
                .into();
        }
    }

    if !input.sig.inputs.is_empty() {
        let msg = "the test function cannot accept arguments";
        return syn::Error::new_spanned(&input.sig.inputs, msg)
            .to_compile_error()
            .into();
    }
    parse_dbtest(input).unwrap_or_else(|e| e.to_compile_error().into())
}

/// This function accepts an entire module which comprises of `dbtest` functions.
/// It takes each function in turn, and generates `#[test]`able functions for them
fn parse_test_module(args: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemMod);
    let content = match input.content {
        Some((_, c)) => c,
        None => {
            return syn::Error::new_spanned(&input, "Couldn't get the module content")
                .to_compile_error()
                .into()
        }
    };
    let args = syn::parse_macro_input!(args as syn::AttributeArgs);
    let mut skips = Vec::new();
    for arg in args {
        if let syn::NestedMeta::Meta(syn::Meta::NameValue(namevalue)) = arg {
            let ident = namevalue.path.get_ident();
            if ident.is_none() {
                let msg = "Must have specified ident";
                return syn::Error::new_spanned(namevalue, msg)
                    .to_compile_error()
                    .into();
            }
            match ident.unwrap().to_string().to_lowercase().as_str() {
                "skip" => {
                    let skip_lit = namevalue.lit.clone();
                    let span = skip_lit.span();
                    skips = match parse_string(skip_lit, span, "skip") {
                        Ok(s) => s,
                        Err(_) => {
                            return syn::Error::new_spanned(
                                namevalue,
                                "Expected a value for argument `skip`",
                            )
                            .to_compile_error()
                            .into();
                        }
                    }
                    .split_whitespace()
                    .map(|val| val.to_string())
                    .collect();
                }
                x => {
                    let msg = format!("Unknown attribute {} is specified; expected `skip`", x);
                    return syn::Error::new_spanned(namevalue, msg)
                        .to_compile_error()
                        .into();
                }
            }
        }
    }
    let modname = &input.ident;
    if *modname != "__private" {
        return syn::Error::new_spanned(
            modname,
            "By convention, all the modules using the `dbtest` macro have to be called `__private`",
        )
        .to_compile_error()
        .into();
    }
    let mut result = quote! {};
    for item in content {
        match item {
            // We just care about functions, so parse functions and ignore everything
            // else
            syn::Item::Fn(function) => {
                if skips.contains(&function.sig.ident.to_string()) {
                    result = quote! {
                        #result
                        #function
                    };
                    continue;
                }
                let inp = parse_test_sig(function);
                let __tok: syn::ItemFn = syn::parse_macro_input!(inp as syn::ItemFn);
                let tok = quote! {
                    #__tok
                };
                result = quote! {
                    #result
                    #tok
                };
            }
            token => {
                result = quote! {
                    #result
                    #token
                };
            }
        }
    }
    result.into()
}

fn parse_string(int: syn::Lit, span: Span, field: &str) -> Result<String, syn::Error> {
    match int {
        syn::Lit::Str(s) => Ok(s.value()),
        syn::Lit::Verbatim(s) => Ok(s.to_string()),
        _ => Err(syn::Error::new(
            span,
            format!("Failed to parse {} into a string.", field),
        )),
    }
}

#[proc_macro_attribute]
/// The `dbtest` macro starts an async server in the background and is meant for
/// use within the `skyd` or `WORKSPACEROOT/server/` crate. If you use this compiler
/// macro in any other crate, you'll simply get compilation errors
///
/// All tests will clean up all values once a single test is over. **These tests should not
/// be run in multi-threaded environments because they often use the same keys**
/// ## _Ghost_ values
/// This macro gives a `skytable::AsyncConnection` accessible by the `con` variable and a mutable
/// `skytable::Query` accessible by the `query` variable
///
/// ## Requirements
///
/// The `#[dbtest]` macro expects several things. The calling crate:
/// - should have the `tokio` crate as a dependency and should have the
/// `features` set to full
/// - should have the `skytable` crate as a dependency and should have the `features` set to `async` and version
/// upstreamed to `next` on skytable/client-rust
///
/// ## Conventions
/// Since `proc_macro` cannot accept _file-linked_ modules and only accepts inline modules, we have made a workaround, which
/// has led to making this a _convention_.
/// So let's say we have a module `kvengine` in which we have our tests. So, we'll have to wrap around all these test functions
/// in a module `__private` within `kvengine`
///
pub fn dbtest(args: TokenStream, item: TokenStream) -> TokenStream {
    parse_test_module(args, item)
}

#[proc_macro_attribute]
pub fn array(args: TokenStream, item: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(args as syn::AttributeArgs);
    if !args.is_empty() {
        syn::Error::new(proc_macro2::Span::call_site(), "Expected 0 arguments")
            .to_compile_error()
            .into()
    } else {
        // fine, so there's something
        let item = item.to_string();
        if !(item.starts_with("const") || item.starts_with("pub const") || item.starts_with("let"))
        {
            syn::Error::new_spanned(item, "Expected a `const` or `let` declaration")
                .to_compile_error()
                .into()
        } else {
            // fine, so it's [let|pub|pub const] : [ty; len] = [1, 2, 3, 4];
            let item = item.trim();
            let ret: Vec<&str> = item.split('=').collect();
            if ret.len() != 2 {
                syn::Error::new_spanned(item, "Expected a `const` or `let` assignment")
                    .to_compile_error()
                    .into()
            } else {
                // so we have the form we expect
                let (declaration, expression) = (ret[0], ret[1]);
                let expression = expression.trim().replace(" ;", "");
                if !(expression.starts_with('[') && expression.ends_with(']')) {
                    syn::Error::new_spanned(declaration, "Expected an array")
                        .to_compile_error()
                        .into()
                } else {
                    let expression = &expression[1..expression.len() - 1];
                    // so we have the raw numbers, separated by commas
                    let count_provided = expression.split(',').count();
                    let declarations: Vec<&str> = declaration.split(':').collect();
                    if declarations.len() != 2 {
                        syn::Error::new_spanned(declaration, "Expected a type")
                            .to_compile_error()
                            .into()
                    } else {
                        // so we have two parts, let's look at the second part: [ty; len]
                        let starts_ends =
                            declarations[1].starts_with('[') && declarations[1].ends_with(']');
                        let ret: Vec<&str> = declarations[1].split(';').collect();
                        if ret.len() != 2 || starts_ends {
                            syn::Error::new_spanned(declaration, "Expected [T; N]")
                                .to_compile_error()
                                .into()
                        } else {
                            // so we have [T; N], let's make it T; N
                            let len = declarations[1].len();
                            // decl hash T; N
                            let decl = &declarations[1][1..len - 1];
                            let expr: Vec<&str> = decl.split(';').collect();
                            let (_, count) = (expr[0], expr[1].replace(']', ""));
                            let count = count.trim();
                            let count = match count.parse::<usize>() {
                                Ok(cnt) => cnt,
                                Err(_) => {
                                    return syn::Error::new_spanned(
                                        count,
                                        "Expected `[T; N]` where `N` is a positive integer",
                                    )
                                    .to_compile_error()
                                    .into()
                                }
                            };
                            let repeats = count - count_provided;
                            // we have uninit, uninit, uninit, uninit, uninit,
                            let repeat_str = "core::mem::MaybeUninit::uninit(),".repeat(repeats);
                            // expression has 1, 2, 3, 4
                            let expression: String = expression
                                .split(',')
                                .map(|s| {
                                    let mut st = String::new();
                                    st.push_str("MaybeUninit::new(");
                                    st.push_str(&s);
                                    st.push(')');
                                    st.push(',');
                                    st
                                })
                                .collect();
                            // remove the trailing comma
                            let expression = &expression[..expression.len() - 1];
                            // let's join them
                            let ret = "[".to_owned() + expression + "," + &repeat_str + "];";
                            let ret = declaration.to_owned() + "=" + &ret;
                            ret.parse().unwrap()
                        }
                    }
                }
            }
        }
    }
}
