#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use std::fmt::Debug;

use alloy_primitives::U8;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, spanned::Spanned, Data, DeriveInput,
    Fields, Ident, MetaNameValue, Path, Token, Type,
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

impl Variant {
    fn is_legacy(&self) -> bool {
        matches!(self.kind, VariantKind::Typed(0))
    }
}

/// Implements the `TransactionEnvelope` trait and defines TxType enum.
#[proc_macro_derive(TransactionEnvelope, attributes(envelope))]
pub fn delegate(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let input_type_name = input.ident;
    let mut tx_type_enum_name =
        Ident::new(&format!("{}Type", input_type_name), input_type_name.span());
    let mut alloy_consensus: Path = parse_quote!(::alloy_consensus);
    let generics = input.generics.clone();
    let unwrapped_generics = generics.params.clone();

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
        }
    }

    let alloy_primitives = quote! { #alloy_consensus::private::alloy_primitives };
    let alloy_eips = quote! { #alloy_consensus::private::alloy_eips };

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
    let variant_types = variants.iter().map(|v| &v.ty).collect::<Vec<_>>();

    let typed_variant_names = variants
        .iter()
        .filter_map(|v| matches!(v.kind, VariantKind::Typed(_)).then_some(&v.name))
        .collect::<Vec<_>>();

    let typed_variant_tx_types = variants
        .iter()
        .filter_map(|v| if let VariantKind::Typed(ty) = v.kind { Some(ty) } else { None })
        .collect::<Vec<_>>();

    let flattened_names = variants
        .iter()
        .filter_map(|v| matches!(v.kind, VariantKind::Flattened).then_some(&v.name))
        .collect::<Vec<_>>();

    let flattened_types = variants
        .iter()
        .filter_map(|v| matches!(v.kind, VariantKind::Flattened).then_some(&v.ty))
        .collect::<Vec<_>>();

    let transaction_bounds = quote! {
        Self: core::fmt::Debug, #(#variant_types: #alloy_consensus::Transaction),*
    };

    let typed_bounds = quote! {
        #(#variant_types: #alloy_eips::eip2718::Typed2718),*
    };

    let encodable_bounds = quote! {
        #(#variant_types: #alloy_eips::Encodable2718),*
    };

    let decodable_bounds = quote! {
        #(#variant_types: #alloy_eips::Decodable2718),*
    };

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
        let Variant { name, kind, .. } = v;
        match kind {
            VariantKind::Flattened => quote! {
                if let Ok(inner) = TryFrom::try_from(value) {
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

    let maybe_txtype_serde = if cfg!(feature = "serde") {
        let u8_path = quote! { #alloy_primitives::U8 }.to_string();
        let u64_path = quote! { #alloy_primitives::U64 }.to_string();
        quote! {
            #[derive(#alloy_consensus::private::serde::Serialize, #alloy_consensus::private::serde::Deserialize)]
            #[serde(into = #u8_path, try_from = #u64_path)]
        }
    } else {
        quote! {}
    };

    let maybe_tx_arbitrary = if cfg!(feature = "arbitrary") {
        let arbitrary_bounds = quote! {
            #(#variant_types: for<'a> #alloy_consensus::private::arbitrary::Arbitrary<'a>),*
        };

        let num_variants = variants.len();
        let range = 0..num_variants;

        quote! {
            impl #generics #alloy_consensus::private::arbitrary::Arbitrary<'_> for #input_type_name #generics where #arbitrary_bounds {
                fn arbitrary(u: &mut #alloy_consensus::private::arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
                    match u.int_in_range(0..=#num_variants-1)? {
                        #(
                            #range => Ok(Self::#variant_names(u.arbitrary()?)),
                        )*
                        _ => unreachable!(),
                    }
                }
            }
        }
    } else {
        quote! {}
    };

    let maybe_tx_serde = if cfg!(feature = "serde") {
        let serde_bounds = quote! {  #input_type_name #generics: Clone, #(#variant_types: serde::Serialize + serde::de::DeserializeOwned),* };

        let serde_bounds_str = serde_bounds.to_string();

        let tagged_variants = variants.iter().filter_map(|v| {
            let Variant { name, ty, kind } = v;
            match kind {
                VariantKind::Flattened => None,
                VariantKind::Typed(tx_type) => {
                    let tx_type = U8::from(*tx_type);
                    let rename = format!("0x{:x}", tx_type);
                    let maybe_alias = if rename.len() == 3 {
                        let alias = format!("0x0{}", rename.chars().last().unwrap());
                        quote! {
                            , alias = #alias
                        }
                    } else {
                        quote! {}
                    };
                    let maybe_with = if v.is_legacy() {
                        let path = quote! {
                            #alloy_consensus::transaction::legacy::signed_legacy_serde
                        }
                        .to_string();
                        quote! {
                            , with = #path
                        }
                    } else {
                        quote! {}
                    };
                    Some(quote! {
                        #[serde(rename = #rename #maybe_alias #maybe_with)]
                        #name(#ty)
                    })
                }
            }
        });

        let (
            maybe_untagged_legacy_variant,
            maybe_untagged_legacy_arm,
            maybe_untagged_legacy_deserialize,
        ) = if let Some(v) = variants.iter().find(|v| v.is_legacy()) {
            let Variant { ty, name, .. } = v;

            let variant = quote! {
                UntaggedLegacy(#ty)
            };

            let arm = quote! {
                UntaggedTxTypes::UntaggedLegacy(tx) => Self::#name(tx),
            };

            let deserialize = quote! {
                if let Ok(val) = #alloy_consensus::transaction::legacy::untagged_legacy_serde::deserialize(deserializer).map(Self::UntaggedLegacy) {
                    return Ok(val);
                }
            };

            (variant, arm, deserialize)
        } else {
            (quote! {}, quote! {}, quote! {})
        };

        let serde = quote! {
            #alloy_consensus::private::serde
        };

        // NB: Why do we need this?
        //
        // Because the tag may be missing, we need an abstraction over tagged (with
        // type) and untagged (always legacy). This is [`MaybeTaggedTxEnvelope`].
        //
        // The tagged variant is [`TaggedTxEnvelope`], which always has a type tag.
        //
        // We serialize via [`TaggedTxEnvelope`] and deserialize via
        // [`MaybeTaggedTxEnvelope`].
        quote! {
            const _: () = {
                #[derive(Debug, serde::Serialize, serde::Deserialize)]
                #[serde(tag = "type", bound = #serde_bounds_str)]
                enum TaggedTxTypes #generics {
                    #(
                        #tagged_variants
                    ),*
                }

                impl #generics From<TaggedTxTypes #generics> for #input_type_name #generics {
                    fn from(value: TaggedTxTypes #generics) -> Self {
                        match value {
                            #(
                                TaggedTxTypes::<#unwrapped_generics>::#typed_variant_names(value) => Self::#typed_variant_names(value),
                            )*
                        }
                    }
                }

                #[derive(#serde::Serialize)]
                #[serde(untagged, bound = #serde_bounds_str)]
                pub(crate) enum UntaggedTxTypes #generics {
                    Tagged(TaggedTxTypes #generics),
                    #(
                        #flattened_names(#flattened_types),
                    )*
                    #maybe_untagged_legacy_variant
                }

                // Manually modified derived serde(untagged) to preserve the error of the [`TaggedTxEnvelope`]
                // attempt. Note: This use private serde API
                impl<'de, #unwrapped_generics> #serde::Deserialize<'de> for UntaggedTxTypes #generics where #serde_bounds {
                    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                    where
                        D: #serde::Deserializer<'de>,
                    {
                        let content = #serde::__private::de::Content::deserialize(deserializer)?;
                        let deserializer =
                            #serde::__private::de::ContentRefDeserializer::<D::Error>::new(&content);

                        let tagged_res =
                            TaggedTxTypes::<#unwrapped_generics>::deserialize(deserializer).map(Self::Tagged);

                        if tagged_res.is_ok() {
                            // return tagged if successful
                            return tagged_res;
                        }

                        // proceed with untagged variants
                        #(
                            if let Ok(val) = #serde::Deserialize::deserialize(deserializer).map(Self::#flattened_names) {
                                return Ok(val);
                            }
                        )*

                        #maybe_untagged_legacy_deserialize

                        // return the original error, which is more useful than the untagged error
                        //  > "data did not match any variant of untagged enum MaybeTaggedTxEnvelope"
                        tagged_res
                    }
                }

                impl #generics From<UntaggedTxTypes #generics> for #input_type_name #generics {
                    fn from(value: UntaggedTxTypes #generics) -> Self {
                        match value {
                            UntaggedTxTypes::Tagged(value) => value.into(),
                            #(
                                UntaggedTxTypes::#flattened_names(value) => Self::#flattened_names(value),
                            )*
                            #maybe_untagged_legacy_arm
                        }
                    }
                }

                impl #generics From<#input_type_name #generics> for UntaggedTxTypes #generics {
                    fn from(value: #input_type_name #generics) -> Self {
                        match value {
                            #(
                                #input_type_name::<#unwrapped_generics>::#flattened_names(value) => Self::#flattened_names(value),
                            )*
                            #(
                                #input_type_name::<#unwrapped_generics>::#typed_variant_names(value) => Self::Tagged(TaggedTxTypes::#typed_variant_names(value))
                            ),*
                        }
                    }
                }

                impl #generics #serde::Serialize for #input_type_name #generics where #serde_bounds {
                    fn serialize<S: #serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                        UntaggedTxTypes::<#unwrapped_generics>::from(self.clone()).serialize(serializer)
                    }
                }

                impl <'de, #unwrapped_generics> #serde::Deserialize<'de> for #input_type_name #generics where #serde_bounds {
                    fn deserialize<D: #serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                        UntaggedTxTypes::<#unwrapped_generics>::deserialize(deserializer).map(Into::into)
                    }
                }
            };
        }
    } else {
        quote! {}
    };

    quote! {
        use #alloy_eips::Encodable2718 as _;
        use #alloy_eips::Decodable2718 as _;

        /// Transaction types supported by [`#inputt_type_name`].
        #[repr(u8)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #maybe_txtype_serde
        pub enum #tx_type_enum_name {
            #(#tx_type_variants),*
        }

        impl From<#tx_type_enum_name> for u8 {
            fn from(value: #tx_type_enum_name) -> Self {
                match value {
                    #(
                        #tx_type_enum_name::#typed_variant_names => #typed_variant_tx_types,
                    )*
                    #(
                        #tx_type_enum_name::#flattened_names(inner) => inner.into()
                    )*
                }
            }
        }

        impl From<#tx_type_enum_name> for #alloy_primitives::U8 {
            fn from(value: #tx_type_enum_name) -> Self {
                Self::from(u8::from(value))
            }
        }

        impl TryFrom<u8> for #tx_type_enum_name {
            type Error = #alloy_eips::eip2718::Eip2718Error;

            fn try_from(value: u8) -> Result<Self, Self::Error> {
                #(#try_from_impls);*
                return Err(#alloy_eips::eip2718::Eip2718Error::UnexpectedType(value))
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

        impl TryFrom<#alloy_primitives::U8> for #tx_type_enum_name {
            type Error = #alloy_eips::eip2718::Eip2718Error;

            fn try_from(value: #alloy_primitives::U8) -> Result<Self, Self::Error> {
                value.to::<u8>().try_into()
            }
        }

        impl TryFrom<#alloy_primitives::U64> for #tx_type_enum_name {
            type Error = &'static str;

            fn try_from(value: #alloy_primitives::U64) -> Result<Self, Self::Error> {
                value.to::<u64>().try_into()
            }
        }

        impl #alloy_eips::eip2718::IsTyped2718 for #tx_type_enum_name {
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

        impl #generics core::hash::Hash for #input_type_name #generics
        where
            Self: #alloy_eips::Encodable2718,
        {
            fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
                self.trie_hash().hash(state);
            }
        }

        impl #generics #alloy_consensus::Transaction for #input_type_name #generics where #transaction_bounds {
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
            fn kind(&self) -> #alloy_primitives::TxKind {
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
            fn value(&self) -> #alloy_primitives::U256 {
                match self {
                    #(
                        Self::#variant_names(tx) => tx.value(),
                    )*
                }
            }

            #[inline]
            fn input(&self) -> &#alloy_primitives::Bytes {
                match self {
                    #(
                        Self::#variant_names(tx) => tx.input(),
                    )*
                }
            }

            #[inline]
            fn access_list(&self) -> Option<&#alloy_eips::eip2930::AccessList> {
                match self {
                    #(
                        Self::#variant_names(tx) => tx.access_list(),
                    )*
                }
            }

            #[inline]
            fn blob_versioned_hashes(&self) -> Option<&[#alloy_primitives::B256]> {
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

        impl #generics #alloy_eips::eip2718::IsTyped2718 for #input_type_name #generics {
            fn is_type(type_id: u8) -> bool {
                <#tx_type_enum_name as #alloy_eips::eip2718::IsTyped2718>::is_type(type_id)
            }
        }

        impl #generics #alloy_consensus::Typed2718 for #input_type_name #generics where #typed_bounds {
            fn ty(&self) -> u8 {
                match self {
                    #(
                        Self::#variant_names(tx) => tx.ty(),
                    )*
                }
            }
        }

        impl #generics #alloy_eips::Encodable2718 for #input_type_name #generics where #encodable_bounds {
            fn encode_2718_len(&self) -> usize {
                match self {
                    #(
                        Self::#variant_names(tx) => tx.encode_2718_len(),
                    )*
                }
            }

            fn encode_2718(&self, out: &mut dyn #alloy_consensus::private::alloy_rlp::BufMut) {
                match self {
                    #(
                        Self::#variant_names(tx) => tx.encode_2718(out),
                    )*
                }
            }

            fn trie_hash(&self) -> #alloy_primitives::B256 {
                match self {
                    #(
                        Self::#variant_names(tx) => tx.trie_hash(),
                    )*
                }
            }
        }

        impl #generics #alloy_consensus::private::alloy_rlp::Decodable for #input_type_name #generics where #decodable_bounds {
            fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
                Ok(Self::network_decode(buf)?)
            }
        }

        impl #generics #alloy_eips::Decodable2718 for #input_type_name #generics where #decodable_bounds {
            fn typed_decode(ty: u8, buf: &mut &[u8]) -> #alloy_eips::eip2718::Eip2718Result<Self> {
                match ty.try_into().map_err(|_| alloy_rlp::Error::Custom("unexpected tx type"))? {
                    #(
                        #tx_type_enum_name::#flattened_names(_) => Ok(Self::#flattened_names(#alloy_eips::Decodable2718::typed_decode(ty, buf)?)),
                    )*
                    #(
                        #tx_type_enum_name::#typed_variant_names => Ok(Self::#typed_variant_names(#alloy_eips::Decodable2718::typed_decode(ty, buf)?)),
                    )*
                }
            }

            fn fallback_decode(buf: &mut &[u8]) -> #alloy_eips::eip2718::Eip2718Result<Self> {
                #(
                    if let Ok(tx) = #alloy_eips::Decodable2718::fallback_decode(buf) {
                        return Ok(Self::#flattened_names(tx))
                    }
                )*
                #(
                    if let Ok(tx) = #alloy_eips::Decodable2718::fallback_decode(buf) {
                        return Ok(Self::#typed_variant_names(tx))
                    }
                )*

                return Err(#alloy_eips::eip2718::Eip2718Error::UnexpectedType(0))
            }
        }

        impl #generics #alloy_consensus::private::alloy_rlp::Encodable for #input_type_name #generics where #encodable_bounds {
            fn encode(&self, out: &mut dyn #alloy_consensus::private::alloy_rlp::BufMut) {
                self.network_encode(out)
            }

            fn length(&self) -> usize {
                self.network_len()
            }
        }

        impl #generics #alloy_consensus::TransactionEnvelope for #input_type_name #generics where #transaction_bounds {
            type TxType = #tx_type_enum_name;
        }

        #maybe_tx_serde
        #maybe_tx_arbitrary
    }
    .into()
}
