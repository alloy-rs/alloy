//! # alloy-delegate-macro
//!
//! This crate provides the [`delegator!`] procedural macro, which generators a delegator macro intended to be use by transaction enums.

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

mod delegate;

#[proc_macro]
#[proc_macro_error]
pub fn sol(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as alloy_sol_macro_input::SolInput);

    SolMacroExpander.expand(&input).unwrap_or_else(syn::Error::into_compile_error).into()
}

struct SolMacroExpander;

impl SolInputExpander for SolMacroExpander {
    fn expand(&mut self, input: &SolInput) -> syn::Result<proc_macro2::TokenStream> {
        let input = input.clone();

        #[cfg(feature = "json")]
        let is_json = matches!(input.kind, SolInputKind::Json { .. });
        #[cfg(not(feature = "json"))]
        let is_json = false;

        // Convert JSON input to Solidity input
        #[cfg(feature = "json")]
        let input = input.normalize_json()?;

        let SolInput { attrs, path, kind } = input;
        let include = path.map(|p| {
            let p = p.to_str().unwrap();
            quote! { const _: &'static [u8] = ::core::include_bytes!(#p); }
        });

        let tokens = match kind {
            SolInputKind::Sol(mut file) => {
                // Attributes have already been added to the inner contract generated in
                // `normalize_json`.
                if !is_json {
                    file.attrs.extend(attrs);
                }

                crate::expand::expand(file)
            }
            SolInputKind::Type(ty) => {
                let (sol_attrs, rest) = SolAttrs::parse(&attrs)?;
                if !rest.is_empty() {
                    return Err(syn::Error::new_spanned(
                        rest.first().unwrap(),
                        "only `#[sol]` attributes are allowed here",
                    ));
                }

                let mut crates = crate::expand::ExternCrates::default();
                crates.fill(&sol_attrs);
                Ok(crate::expand::expand_type(&ty, &crates))
            }
            #[cfg(feature = "json")]
            SolInputKind::Json(_, _) => unreachable!("input already normalized"),
        }?;

        Ok(quote! {
            #include
            #tokens
        })
    }
}
