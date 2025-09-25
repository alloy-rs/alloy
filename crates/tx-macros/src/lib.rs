#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![allow(clippy::option_if_let_else)]

mod expand;
mod parse;
mod serde;

use expand::Expander;
use parse::{EnvelopeArgs, GroupedVariants};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, DeriveInput, Error, Ident};

/// Derive macro for creating transaction envelope types.
///
/// This macro generates a transaction envelope implementation that supports
/// multiple transaction types following the EIP-2718 standard.
///
/// # Container Attributes
///
/// - `#[envelope(tx_type_name = MyTxType)]` - Custom name for the generated transaction type enum
/// - `#[envelope(alloy_consensus = path::to::alloy)]` - Custom path to alloy_consensus crate
/// - `#[envelope(typed = MyTypedTransaction)]` - Generate a corresponding TypedTransaction enum
///   (optional)
///
/// # Variant Attributes
/// - Each variant must be annotated with `envelope` attribute with one of the following options:
///   - `#[envelope(ty = N)]` - Specify the transaction type ID (0-255)
///   - `#[envelope(ty = N, typed = CustomType)]` - Use a custom transaction type for this variant
///     in the generated TypedTransaction (optional)
///   - `#[envelope(flatten)]` - Flatten this variant to delegate to inner envelope type
///
/// # Generated Code
///
/// The macro generates:
/// - A `MyTxType` enum with transaction type variants
/// - Implementations of `Transaction`, `Typed2718`, `Encodable2718`, `Decodable2718`
/// - Serde serialization/deserialization support (if `serde` feature is enabled)
/// - Arbitrary implementations (if `arbitrary` feature is enabled)
/// - Optionally, a TypedTransaction enum (if `typed` attribute is specified)
#[proc_macro_derive(TransactionEnvelope, attributes(envelope, serde))]
pub fn derive_transaction_envelope(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match expand_transaction_envelope(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Expand the transaction envelope derive macro.
fn expand_transaction_envelope(input: syn::DeriveInput) -> Result<proc_macro2::TokenStream, Error> {
    use darling::FromDeriveInput;

    // Parse the input with darling
    let args = EnvelopeArgs::from_derive_input(&input)
        .map_err(|e| Error::new_spanned(&input.ident, e.to_string()))?;

    // Extract config values before consuming args
    let input_type_name = args.ident.clone();
    let tx_type_enum_name = args
        .tx_type_name
        .clone()
        .unwrap_or_else(|| Ident::new(&format!("{input_type_name}Type"), input_type_name.span()));
    let alloy_consensus =
        args.alloy_consensus.clone().unwrap_or_else(|| parse_quote!(::alloy_consensus));
    let generics = args.generics.clone();
    let typed = args.typed.clone();
    let serde_cfg = match args.serde_cfg.as_ref() {
        Some(syn::Meta::List(list)) => list.tokens.clone(),
        Some(_) => {
            return Err(Error::new_spanned(
                &input.ident,
                "serde_cfg must be a list like `serde_cfg(feature = \"serde\")`",
            ))
        }
        // this is always true
        None => quote! { all() },
    };

    let arbitrary_cfg = match args.arbitrary_cfg.as_ref() {
        Some(syn::Meta::List(list)) => list.tokens.clone(),
        Some(_) => {
            return Err(Error::new_spanned(
                &input.ident,
                "arbitrary_cfg must be a list like `arbitrary_cfg(feature = \"arbitrary\")`",
            ))
        }
        None => quote! { all() },
    };

    let variants = GroupedVariants::from_args(args)?;

    let alloy_primitives = quote! { #alloy_consensus::private::alloy_primitives };
    let alloy_eips = quote! { #alloy_consensus::private::alloy_eips };
    let alloy_rlp = quote! { #alloy_consensus::private::alloy_rlp };

    // Expand the macro
    let expander = Expander {
        input_type_name,
        tx_type_enum_name,
        alloy_consensus,
        generics,
        serde_enabled: cfg!(feature = "serde"),
        serde_cfg,
        arbitrary_cfg,
        arbitrary_enabled: cfg!(feature = "arbitrary"),
        alloy_primitives,
        alloy_eips,
        alloy_rlp,
        variants,
        typed,
    };
    Ok(expander.expand())
}
