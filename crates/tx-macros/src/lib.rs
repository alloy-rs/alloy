#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(clippy::option_if_let_else)]

mod expand;
mod parse;
mod serde;

use expand::Expander;
use parse::{EnvelopeArgs, GroupedVariants};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, DeriveInput, Error, Ident};

/// Derives an EIP-2718 transaction envelope and its transaction-type enum.
///
/// # Input requirements
///
/// The target must be an enum, and every variant must contain exactly one unnamed field. Annotate
/// each variant with either `#[envelope(ty = N)]` or `#[envelope(flatten)]`.
///
/// For `ty = N`, the inner type must implement the transaction, `Typed2718`, `Encodable2718`, and
/// `Decodable2718` contracts used by the generated implementations. `N` controls the generated
/// type enum and decode dispatch; it does **not** override the inner value's reported type or
/// encoded prefix. The inner implementation must therefore already report, encode, and decode the
/// same ID. Untagged fallback decoding tries each variant's inner `fallback_decode` in declaration
/// order, so non-legacy variants must reject legacy input and fallback acceptance must not overlap.
/// `ty = 0` receives the macro's legacy JSON handling and should be used only for the legacy
/// transaction representation.
///
/// EIP-2718 type bytes are in `0x00..=0x7f`. The attribute parser currently accepts any `u8`, but
/// values above `0x7f` are not valid EIP-2718 envelope types and are not interoperable with
/// standard raw EIP-2718 decoding, even though generated RLP methods may round-trip them.
///
/// A `flatten` variant delegates to an inner `TransactionEnvelope`. Its type IDs must not overlap
/// any other direct or flattened variant, because overlapping IDs make dispatch order-dependent.
///
/// The derive itself generates `PartialEq`, `Eq`, and `Hash` for the envelope, so do not also list
/// those traits in `#[derive(...)]`. Derive `Clone` and `Debug` as needed by the generated trait
/// bounds.
///
/// # Container attributes
///
/// - `#[envelope(tx_type_name = MyTxType)]` names the generated public type enum. It defaults to
///   `{EnvelopeName}Type`.
/// - `#[envelope(alloy_consensus = path::to::consensus)]` changes the path used by generated code.
///   The default is `::alloy_consensus`. When using only the `alloy` meta crate, set this to
///   `alloy::consensus`.
/// - `#[envelope(typed = MyTypedTransaction)]` also generates a corresponding typed-transaction
///   enum. Syntactic `Signed<T>` and `Sealed<T>` fields are unwrapped; other field types remain as
///   declared, so the result is not necessarily unsigned.
/// - `#[envelope(serde_cfg(feature = "serde"))]` gates generated Serde implementations with the
///   supplied `cfg` predicate.
/// - `#[envelope(arbitrary_cfg(feature = "arbitrary"))]` similarly gates generated `Arbitrary`
///   implementations.
///
/// Serde or `Arbitrary` code is emitted only when the corresponding `alloy-tx-macros` feature is
/// enabled; `alloy-consensus` forwards its features. Without a `*_cfg` attribute, emitted
/// implementations are unconditional in the consuming crate.
///
/// # Variant attributes
///
/// - `#[envelope(ty = N)]` declares a directly supported type ID.
/// - `#[envelope(flatten)]` delegates type identification and codec behavior to an inner envelope.
/// - `#[envelope(ty = N, typed = CustomType)]` selects the field type used for that variant in a
///   generated typed-transaction enum. By default, the macro syntactically unwraps `Signed<T>` and
///   `Sealed<T>` to `T`; other field types are reused unchanged.
/// - A variant's `#[serde(...)]` options are forwarded to the macro's internal representation when
///   Serde generation is enabled.
///
/// # Generated API
///
/// In addition to the named transaction-type enum, the macro implements `Transaction`,
/// `TransactionEnvelope`, `Typed2718`, `IsTyped2718`, `Encodable2718`, `Decodable2718`, and RLP
/// encoding and decoding for the envelope. Supplying `typed = ...` creates the corresponding enum
/// and its transaction/type implementations; see the crate-level example.
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
