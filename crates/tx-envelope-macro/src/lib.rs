#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use std::fmt::Debug;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, spanned::Spanned, Data, DeriveInput,
    Expr, Fields, Ident, MetaNameValue, Path, Token, Type,
};

#[derive(Debug)]
enum VariantKind {
    Flattened,
    Typed(u8),
}

#[derive(Debug)]
struct Variant {
    name: Ident,
    ty: Type,
    kind: VariantKind,
}

/// Implements the `TransactionEnvelope` trait and defines TxType enum.
#[proc_macro_derive(TxEnvelope, attributes(envelope))]
pub fn delegate(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let input_type_name = input.ident;
    let mut tx_type_enum_name =
        Ident::new(&format!("{}Type", input_type_name), input_type_name.span());
    let mut alloy_consensus: Path = parse_quote!(::alloy_consensus);
    let generics = input.generics.clone();
    let mut bounds = input.generics.into_token_stream();

    let attrs = input.attrs.iter().filter_map(|attr| {
        if let syn::Meta::List(list) = &attr.meta {
            list.path.is_ident("envelope").then_some(list)
        } else {
            None
        }
    });

    for list in attrs {
        let values =
            match list.parse_args_with(Punctuated::<MetaNameValue, Token![,]>::parse_terminated) {
                Ok(values) => values,
                Err(err) => {
                    return err.to_compile_error().into();
                }
            };
        for value in values {
            if value.path.is_ident("tx_type_name") {
                tx_type_enum_name =
                    Ident::new(&value.value.to_token_stream().to_string(), value.value.span());
            }

            if value.path.is_ident("alloy_consensus") {
                if let Ok(path) = syn::parse(value.value.to_token_stream().into()) {
                    alloy_consensus = path;
                }
            }

            if value.path.is_ident("bound") {
                if let Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(lit_str), .. }) = value.value {
                    match lit_str.value().parse::<proc_macro2::TokenStream>() {
                        Ok(tokens) => {
                            bounds = quote!(<#tokens>);
                        },
                        Err(err) => {
                            return syn::Error::new(lit_str.span(), err).to_compile_error().into()
                        }
                    }
                }
            }
        }
    }

    let Data::Enum(data) = input.data else {
        return syn::Error::new(input_type_name.span(), "TxEnvelope can only be derived for enums")
            .into_compile_error()
            .into();
    };

    let mut variants = Vec::new();
    for mut variant in data.variants {
        let Fields::Unnamed(value) = &mut variant.fields else {
            return syn::Error::new(variant.span(), "expected unit variant")
                .into_compile_error()
                .into();
        };

        let ty = match value.unnamed.len() {
            0 => {
                return syn::Error::new(variant.span(), "expected single field variant")
                    .into_compile_error()
                    .into()
            }
            2.. => {
                return syn::Error::new(value.unnamed[1].span(), "expected single field variant")
                    .into_compile_error()
                    .into()
            }
            1 => value.unnamed.pop().unwrap().into_value().ty,
        };

        let attrs = variant
            .attrs
            .iter()
            .filter_map(|attr| {
                if let syn::Meta::List(list) = &attr.meta {
                    list.path.is_ident("envelope").then_some(list)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let kind = if attrs.iter().any(|list| list.tokens.to_string() == "flatten") {
            VariantKind::Flattened
        } else if let Some(ty) = attrs.iter().find_map(|attr| {
            if let Ok(meta) = syn::parse::<MetaNameValue>(attr.tokens.clone().into()) {
                if meta.path.is_ident("ty") {
                    if let Ok(ty) = meta.value.into_token_stream().to_string().parse::<u8>() {
                        Some(ty)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }) {
            VariantKind::Typed(ty)
        } else {
            return syn::Error::new(variant.span(), "expected `flaten` or `ty` attribute")
                .into_compile_error()
                .into();
        };

        variants.push(Variant { name: variant.ident, ty, kind });
    }

    let variant_names = variants.iter().map(|v| &v.name).collect::<Vec<_>>();

    let tx_type_variants = variants.iter().map(|v| {
        let Variant { name, ty, kind } = v;
        match kind {
            VariantKind::Flattened => quote! {
                /// Transaction type of an inner [`#ty`].
                #name(<#ty as TransactionEnvelope>::TxType)
            },
            VariantKind::Typed(ty) => quote! {
                /// Transaction type of [`#ty`].
                #name = #ty
            },
        }
    });

    let typed_2718_impls = variants.iter().map(|v| {
        let Variant { name, kind, .. } = v;
        match kind {
            VariantKind::Flattened => quote! {
                Self::#name(inner) => #alloy_consensus::Typed2718::ty(inner)
            },
            VariantKind::Typed(ty) => quote! {
                Self::#name => #ty
            },
        }
    });

    let try_from_impls = variants.iter().map(|v| {
        let Variant { name, kind, ty } = v;
        match kind {
            VariantKind::Flattened => quote! {
                if let Ok(inner) = #ty::try_from(value) {
                    return Ok(Self::#name(inner))
                }
            },
            VariantKind::Typed(ty) => quote! {
                if value == #ty {
                    return Ok(Self::#name)
                }
            },
        }
    });

    let maybe_imports = if cfg!(feature = "serde") {
        quote! {
            use #alloy_consensus::private::alloy_primitives::U8 as PrimitivesU8;
            use #alloy_consensus::private::alloy_primitives::U64 as PrimitivesU64;
        }
    } else {
        quote! {}
    };

    let maybe_serde_derive = if cfg!(feature = "serde") {
        quote! {
            #[derive(#alloy_consensus::private::serde::Serialize, #alloy_consensus::private::serde::Deserialize)]
            #[serde(
                into = "PrimitivesU8", 
                try_from = "PrimitivesU64",
            )]
        }
    } else {
        quote! {}
    };

    quote! {
        #maybe_imports

        /// Transaction types supported by [`#inputt_type_name`].
        #[repr(u8)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #maybe_serde_derive
        pub enum #tx_type_enum_name {
            #(#tx_type_variants),*
        }

        impl From<#tx_type_enum_name> for u8 {
            fn from(value: #tx_type_enum_name) -> Self {
                value as Self
            }
        }

        impl From<#tx_type_enum_name> for #alloy_consensus::private::alloy_primitives::U8 {
            fn from(value: #tx_type_enum_name) -> Self {
                Self::from(u8::from(value))
            }
        }

        impl TryFrom<u8> for #tx_type_enum_name {
            type Error = #alloy_consensus::private::alloy_eips::eip2718::Eip2718Error;
        
            fn try_from(value: u8) -> Result<Self, Self::Error> {
                #(#try_from_impls);*
                return Err(#alloy_consensus::private::alloy_eips::eip2718::Eip2718Error::UnexpectedType(value))
            }
        }

        impl TryFrom<u64> for #tx_type_enum_name {
            type Error = &'static str;
        
            fn try_from(value: u64) -> Result<Self, Self::Error> {
                let err = || "invalid tx type";
                let value: u8 = value.try_into().map_err(|_| err())?;
                Self::try_from(value).map_err(|_| err())
            }
        }
        
        impl TryFrom<#alloy_consensus::private::alloy_primitives::U8> for #tx_type_enum_name {
            type Error = #alloy_consensus::private::alloy_eips::eip2718::Eip2718Error;
        
            fn try_from(value: #alloy_consensus::private::alloy_primitives::U8) -> Result<Self, Self::Error> {
                value.to::<u8>().try_into()
            }
        }
        
        impl TryFrom<#alloy_consensus::private::alloy_primitives::U64> for #tx_type_enum_name {
            type Error = &'static str;
        
            fn try_from(value: #alloy_consensus::private::alloy_primitives::U64) -> Result<Self, Self::Error> {
                value.to::<u64>().try_into()
            }
        }

        impl #alloy_consensus::private::alloy_eips::eip2718::IsTyped2718 for #tx_type_enum_name {
            fn is_type(type_id: u8) -> bool {
                Self::try_from(type_id).is_ok()
            }
        }

        impl #alloy_consensus::Typed2718 for #tx_type_enum_name {
            fn ty(&self) -> u8 {
                match self {
                    #(#typed_2718_impls),*
                }
            }
        }

        impl #bounds #alloy_consensus::Transaction for #input_type_name #generics {
            #[inline]
            fn chain_id(&self) -> Option<u64> {
                match self {
                    #(
                        Self::#variant_names(tx) => tx.chain_id(),
                    )*
                }

            }

            #[inline]
            fn nonce(&self) -> u64 {
                match self {
                    #(
                        Self::#variant_names(tx) => tx.nonce(),
                    )*
                }
            }

            #[inline]
            fn gas_limit(&self) -> u64 {
                match self {
                    #(
                        Self::#variant_names(tx) => tx.gas_limit(),
                    )*
                }
            }

            #[inline]
            fn gas_price(&self) -> Option<u128> {
                match self {
                    #(
                        Self::#variant_names(tx) => tx.gas_price(),
                    )*
                }
            }

            #[inline]
            fn max_fee_per_gas(&self) -> u128 {
                match self {
                    #(
                        Self::#variant_names(tx) => tx.max_fee_per_gas(),
                    )*
                }
            }

            #[inline]
            fn max_priority_fee_per_gas(&self) -> Option<u128> {
                match self {
                    #(
                        Self::#variant_names(tx) => tx.max_priority_fee_per_gas(),
                    )*
                }
            }

            #[inline]
            fn max_fee_per_blob_gas(&self) -> Option<u128> {
                match self {
                    #(
                        Self::#variant_names(tx) => tx.max_fee_per_blob_gas(),
                    )*
                }
            }

            #[inline]
            fn priority_fee_or_price(&self) -> u128 {
                match self {
                    #(
                        Self::#variant_names(tx) => tx.priority_fee_or_price(),
                    )*
                }
            }

            #[inline]
            fn effective_gas_price(&self, base_fee: Option<u64>) -> u128 {
                match self {
                    #(
                        Self::#variant_names(tx) => tx.effective_gas_price(base_fee),
                    )*
                }
            }

            #[inline]
            fn is_dynamic_fee(&self) -> bool {
                match self {
                    #(
                        Self::#variant_names(tx) => tx.is_dynamic_fee(),
                    )*
                }
            }

            #[inline]
            fn kind(&self) -> TxKind {
                match self {
                    #(
                        Self::#variant_names(tx) => tx.kind(),
                    )*
                }
            }

            #[inline]
            fn is_create(&self) -> bool {
                match self {
                    #(
                        Self::#variant_names(tx) => tx.is_create(),
                    )*
                }
            }

            #[inline]
            fn value(&self) -> U256 {
                match self {
                    #(
                        Self::#variant_names(tx) => tx.value(),
                    )*
                }
            }

            #[inline]
            fn input(&self) -> &Bytes {
                match self {
                    #(
                        Self::#variant_names(tx) => tx.input(),
                    )*
                }
            }

            #[inline]
            fn access_list(&self) -> Option<&AccessList> {
                match self {
                    #(
                        Self::#variant_names(tx) => tx.access_list(),
                    )*
                }
            }

            #[inline]
            fn blob_versioned_hashes(&self) -> Option<&[B256]> {
                match self {
                    #(
                        Self::#variant_names(tx) => tx.blob_versioned_hashes(),
                    )*
                }
            }

            #[inline]
            fn authorization_list(&self) -> Option<&[#alloy_consensus::transaction::SignedAuthorization]> {
                match self {
                    #(
                        Self::#variant_names(tx) => tx.authorization_list(),
                    )*
                }
            }
        }

        impl #bounds #alloy_consensus::private::alloy_eips::eip2718::IsTyped2718 for #input_type_name #generics {
            fn is_type(type_id: u8) -> bool {
                <#tx_type_enum_name as IsTyped2718>::is_type(type_id)
            }
        }

        impl #bounds #alloy_consensus::Typed2718 for #input_type_name #generics {
            fn ty(&self) -> u8 {
                match self {
                    #(
                        Self::#variant_names(tx) => tx.ty(),
                    )*
                }
            }
        }

        impl #bounds #alloy_consensus::TransactionEnvelope for #input_type_name #generics {
            type TxType = #tx_type_enum_name;
        }
    }
    .into()
}
