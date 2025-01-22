//! # alloy-delegate-macro
//!
//! This crate provides the [`delegator!`] procedural macro, which generators a delegator macro
//! intended to be use by transaction enums.

#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

#[macro_use]
extern crate proc_macro_error2;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;
use crate::delegate::DelegatorInput;

mod delegate;

// TODO: add custom derive macro that auto delegates known alloy traits?

/// Generate a delegator macro delegates function calls to variants
#[proc_macro]
#[proc_macro_error]
pub fn delegator(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as delegate::DelegatorInput);
    expand_delegator_macro(&input).unwrap_or_else(syn::Error::into_compile_error).into()
}

fn expand_delegator_macro(input: &DelegatorInput) -> syn::Result<proc_macro2::TokenStream> {
    let DelegatorInput { variants }  = input;
    Ok(quote! {
            /// Delegates the given function call to the value of each enum
            macro_rules! delegate {
                ($self:expr => $tx:ident.$method:ident($($arg:expr),*)) => {
                    match $self {
                        #(Self::#variants($tx) => $tx.$method($($arg),*),)*
                    }
                };
            }
        })
}
