use crate::parse::{GroupedVariants, VariantKind};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Ident, Path};

/// Expander for the TransactionEnvelope derive macro.
pub(crate) struct Expander {
    /// The name of the input enum.
    pub(crate) input_type_name: Ident,
    /// The name of the generated transaction type enum.
    pub(crate) tx_type_enum_name: Ident,
    /// The path to alloy_consensus.
    pub(crate) alloy_consensus: Path,
    /// The generics of the input enum.
    pub(crate) generics: syn::Generics,
    /// Whether serde feature is enabled.
    pub(crate) serde_enabled: bool,
    /// Whether arbitrary feature is enabled.
    pub(crate) arbitrary_enabled: bool,
    /// Cached path for alloy_primitives.
    pub(crate) alloy_primitives: TokenStream,
    /// Cached path for alloy_eips.
    pub(crate) alloy_eips: TokenStream,
    /// Cached path for alloy_rlp.
    pub(crate) alloy_rlp: TokenStream,
    /// Grouped variants for code generation.
    pub(crate) variants: GroupedVariants,
}

impl Expander {
    /// Expand the derive macro into implementations.
    pub(crate) fn expand(&self) -> TokenStream {
        let imports = self.generate_imports();
        let tx_type_enum = self.generate_tx_type_enum();
        let trait_impls = self.generate_trait_impls();
        let serde_impls = self.generate_serde_impls();
        let arbitrary_impls = self.generate_arbitrary_impls();

        quote! {
            #imports
            #tx_type_enum
            #trait_impls
            #serde_impls
            #arbitrary_impls
        }
    }

    /// Generate necessary imports.
    fn generate_imports(&self) -> TokenStream {
        let alloy_eips = &self.alloy_eips;
        quote! {
            use #alloy_eips::Encodable2718 as _;
            use #alloy_eips::Decodable2718 as _;
        }
    }

    /// Generate the transaction type enum.
    fn generate_tx_type_enum(&self) -> TokenStream {
        let tx_type_enum_name = &self.tx_type_enum_name;
        let alloy_eips = &self.alloy_eips;
        let alloy_consensus = &self.alloy_consensus;

        let variants = self.generate_tx_type_variants();
        let conversions = self.generate_tx_type_conversions();
        let typed_impls = self.generate_tx_type_typed_impls();
        let serde_derive = self.generate_tx_type_serde_derive();

        let doc_comment = format!("Transaction types supported by [`{}`].", self.input_type_name);

        quote! {
            #[doc = #doc_comment]
            #[repr(u8)]
            #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
            #serde_derive
            pub enum #tx_type_enum_name {
                #variants
            }

            #conversions
            #typed_impls

            impl #alloy_eips::eip2718::IsTyped2718 for #tx_type_enum_name {
                fn is_type(type_id: u8) -> bool {
                    Self::try_from(type_id).is_ok()
                }
            }

            impl PartialEq<u8> for #tx_type_enum_name {
                fn eq(&self, other: &u8) -> bool {
                    u8::from(*self) == *other
                }
            }

            impl PartialEq<#tx_type_enum_name> for u8 {
                fn eq(&self, other: &#tx_type_enum_name) -> bool {
                    *self == u8::from(*other)
                }
            }

            impl #alloy_consensus::private::alloy_rlp::Encodable for #tx_type_enum_name {
                fn encode(&self, out: &mut dyn #alloy_consensus::private::alloy_rlp::BufMut) {
                    u8::from(*self).encode(out);
                }

                fn length(&self) -> usize {
                    u8::from(*self).length()
                }
            }

            impl #alloy_consensus::private::alloy_rlp::Decodable for #tx_type_enum_name {
                fn decode(buf: &mut &[u8]) -> #alloy_consensus::private::alloy_rlp::Result<Self> {
                    let ty = u8::decode(buf)?;
                    Self::try_from(ty).map_err(|_| #alloy_consensus::private::alloy_rlp::Error::Custom("invalid transaction type"))
                }
            }
        }
    }

    /// Generate variants for the transaction type enum.
    fn generate_tx_type_variants(&self) -> TokenStream {
        let alloy_consensus = &self.alloy_consensus;
        let variants = self.variants.all.iter().map(|v| {
            let name = &v.name;
            let ty = &v.ty;

            match &v.kind {
                VariantKind::Flattened => {
                    let doc_comment =
                        format!("Transaction type of an inner `{}`.", ty.to_token_stream());
                    quote! {
                        #[doc = #doc_comment]
                        #name(<#ty as #alloy_consensus::TransactionEnvelope>::TxType)
                    }
                }
                VariantKind::Typed(ty_id) => {
                    let doc_comment = format!("Transaction type of `{}`.", ty.to_token_stream());
                    quote! {
                        #[doc = #doc_comment]
                        #name = #ty_id
                    }
                }
            }
        });

        quote! { #(#variants),* }
    }

    /// Generate conversion implementations for the transaction type enum.
    fn generate_tx_type_conversions(&self) -> TokenStream {
        let tx_type_enum_name = &self.tx_type_enum_name;
        let alloy_primitives = &self.alloy_primitives;
        let alloy_eips = &self.alloy_eips;

        let from_arms = self.variants.all.iter().map(|v| {
            let name = &v.name;
            match &v.kind {
                VariantKind::Typed(ty_id) => quote! { #tx_type_enum_name::#name => #ty_id },
                VariantKind::Flattened => {
                    quote! { #tx_type_enum_name::#name(inner) => inner.into() }
                }
            }
        });

        let try_from_arms = self.variants.all.iter().map(|v| {
            let name = &v.name;
            match &v.kind {
                VariantKind::Flattened => quote! {
                    if let Ok(inner) = TryFrom::try_from(value) {
                        return Ok(Self::#name(inner))
                    }
                },
                VariantKind::Typed(ty_id) => quote! {
                    if value == #ty_id {
                        return Ok(Self::#name)
                    }
                },
            }
        });

        quote! {
            impl From<#tx_type_enum_name> for u8 {
                fn from(value: #tx_type_enum_name) -> Self {
                    match value {
                        #(#from_arms),*
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
                    #(#try_from_arms);*
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
        }
    }

    /// Generate typed implementation for transaction type enum.
    fn generate_tx_type_typed_impls(&self) -> TokenStream {
        let tx_type_enum_name = &self.tx_type_enum_name;
        let alloy_consensus = &self.alloy_consensus;

        let arms = self.variants.all.iter().map(|v| {
            let name = &v.name;
            match &v.kind {
                VariantKind::Flattened => quote! {
                    Self::#name(inner) => #alloy_consensus::Typed2718::ty(inner)
                },
                VariantKind::Typed(ty_id) => quote! {
                    Self::#name => #ty_id
                },
            }
        });

        quote! {
            impl #alloy_consensus::Typed2718 for #tx_type_enum_name {
                fn ty(&self) -> u8 {
                    match self {
                        #(#arms),*
                    }
                }
            }
        }
    }

    /// Generate serde derive for transaction type enum if enabled.
    fn generate_tx_type_serde_derive(&self) -> TokenStream {
        if self.serde_enabled {
            let alloy_primitives = &self.alloy_primitives;
            let alloy_consensus = &self.alloy_consensus;
            let u8_path = quote! { #alloy_primitives::U8 }.to_string();
            let u64_path = quote! { #alloy_primitives::U64 }.to_string();
            let serde_str = quote! { #alloy_consensus::private::serde }.to_string();

            quote! {
                #[derive(#alloy_consensus::private::serde::Serialize, #alloy_consensus::private::serde::Deserialize)]
                #[serde(into = #u8_path, try_from = #u64_path, crate = #serde_str)]
            }
        } else {
            quote! {}
        }
    }

    /// Generate trait implementations for the main enum.
    fn generate_trait_impls(&self) -> TokenStream {
        let eq_impl = self.generate_eq_impl();
        let hash_impl = self.generate_hash_impl();
        let transaction_impl = self.generate_transaction_impl();
        let typed_impl = self.generate_typed_impl();
        let encodable_impl = self.generate_encodable_impl();
        let decodable_impl = self.generate_decodable_impl();
        let envelope_impl = self.generate_envelope_impl();

        quote! {
            #eq_impl
            #hash_impl
            #transaction_impl
            #typed_impl
            #encodable_impl
            #decodable_impl
            #envelope_impl
        }
    }

    /// Generate PartialEq and Eq implementations.
    fn generate_eq_impl(&self) -> TokenStream {
        let input_type_name = &self.input_type_name;
        let (impl_generics, ty_generics, _) = self.generics.split_for_impl();

        let variant_names = self.variants.variant_names();
        let variant_types = self.variants.variant_types();

        quote! {
            impl #impl_generics PartialEq for #input_type_name #ty_generics
            where
                #(#variant_types: PartialEq),*
            {
                fn eq(&self, other: &Self) -> bool {
                    match (self, other) {
                        #((Self::#variant_names(tx), Self::#variant_names(other)) => tx.eq(other),)*
                        _ => false,
                    }
                }
            }

            impl #impl_generics Eq for #input_type_name #ty_generics where #(#variant_types: PartialEq),* {}
        }
    }

    /// Generate Hash implementation.
    fn generate_hash_impl(&self) -> TokenStream {
        let input_type_name = &self.input_type_name;
        let (impl_generics, ty_generics, _) = self.generics.split_for_impl();
        let alloy_eips = &self.alloy_eips;

        quote! {
            impl #impl_generics core::hash::Hash for #input_type_name #ty_generics
            where
                Self: #alloy_eips::Encodable2718,
            {
                fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
                    self.trie_hash().hash(state);
                }
            }
        }
    }

    /// Generate Transaction trait implementation.
    fn generate_transaction_impl(&self) -> TokenStream {
        let input_type_name = &self.input_type_name;
        let (impl_generics, ty_generics, _) = self.generics.split_for_impl();
        let alloy_consensus = &self.alloy_consensus;
        let alloy_primitives = &self.alloy_primitives;
        let alloy_eips = &self.alloy_eips;

        let variant_names = self.variants.variant_names();
        let variant_types = self.variants.variant_types();

        quote! {
            impl #impl_generics #alloy_consensus::Transaction for #input_type_name #ty_generics
            where
                Self: core::fmt::Debug,
                #(#variant_types: #alloy_consensus::Transaction),*
            {
                #[inline]
                fn chain_id(&self) -> Option<u64> {
                    match self { #(Self::#variant_names(tx) => tx.chain_id(),)* }
                }

                #[inline]
                fn nonce(&self) -> u64 {
                    match self { #(Self::#variant_names(tx) => tx.nonce(),)* }
                }

                #[inline]
                fn gas_limit(&self) -> u64 {
                    match self { #(Self::#variant_names(tx) => tx.gas_limit(),)* }
                }

                #[inline]
                fn gas_price(&self) -> Option<u128> {
                    match self { #(Self::#variant_names(tx) => tx.gas_price(),)* }
                }

                #[inline]
                fn max_fee_per_gas(&self) -> u128 {
                    match self { #(Self::#variant_names(tx) => tx.max_fee_per_gas(),)* }
                }

                #[inline]
                fn max_priority_fee_per_gas(&self) -> Option<u128> {
                    match self { #(Self::#variant_names(tx) => tx.max_priority_fee_per_gas(),)* }
                }

                #[inline]
                fn max_fee_per_blob_gas(&self) -> Option<u128> {
                    match self { #(Self::#variant_names(tx) => tx.max_fee_per_blob_gas(),)* }
                }

                #[inline]
                fn priority_fee_or_price(&self) -> u128 {
                    match self { #(Self::#variant_names(tx) => tx.priority_fee_or_price(),)* }
                }

                #[inline]
                fn effective_gas_price(&self, base_fee: Option<u64>) -> u128 {
                    match self { #(Self::#variant_names(tx) => tx.effective_gas_price(base_fee),)* }
                }

                #[inline]
                fn is_dynamic_fee(&self) -> bool {
                    match self { #(Self::#variant_names(tx) => tx.is_dynamic_fee(),)* }
                }

                #[inline]
                fn kind(&self) -> #alloy_primitives::TxKind {
                    match self { #(Self::#variant_names(tx) => tx.kind(),)* }
                }

                #[inline]
                fn is_create(&self) -> bool {
                    match self { #(Self::#variant_names(tx) => tx.is_create(),)* }
                }

                #[inline]
                fn value(&self) -> #alloy_primitives::U256 {
                    match self { #(Self::#variant_names(tx) => tx.value(),)* }
                }

                #[inline]
                fn input(&self) -> &#alloy_primitives::Bytes {
                    match self { #(Self::#variant_names(tx) => tx.input(),)* }
                }

                #[inline]
                fn access_list(&self) -> Option<&#alloy_eips::eip2930::AccessList> {
                    match self { #(Self::#variant_names(tx) => tx.access_list(),)* }
                }

                #[inline]
                fn blob_versioned_hashes(&self) -> Option<&[#alloy_primitives::B256]> {
                    match self { #(Self::#variant_names(tx) => tx.blob_versioned_hashes(),)* }
                }

                #[inline]
                fn authorization_list(&self) -> Option<&[#alloy_eips::eip7702::SignedAuthorization]> {
                    match self { #(Self::#variant_names(tx) => tx.authorization_list(),)* }
                }
            }
        }
    }

    /// Generate Typed2718 implementations.
    fn generate_typed_impl(&self) -> TokenStream {
        let input_type_name = &self.input_type_name;
        let tx_type_enum_name = &self.tx_type_enum_name;
        let (impl_generics, ty_generics, _) = self.generics.split_for_impl();
        let alloy_consensus = &self.alloy_consensus;
        let alloy_eips = &self.alloy_eips;

        let variant_names = self.variants.variant_names();
        let variant_types = self.variants.variant_types();

        quote! {
            impl #impl_generics #alloy_eips::eip2718::IsTyped2718 for #input_type_name #ty_generics {
                fn is_type(type_id: u8) -> bool {
                    <#tx_type_enum_name as #alloy_eips::eip2718::IsTyped2718>::is_type(type_id)
                }
            }

            impl #impl_generics #alloy_consensus::Typed2718 for #input_type_name #ty_generics
            where
                #(#variant_types: #alloy_eips::eip2718::Typed2718),*
            {
                fn ty(&self) -> u8 {
                    match self {
                        #(Self::#variant_names(tx) => tx.ty(),)*
                    }
                }
            }
        }
    }

    /// Generate Encodable2718 implementation.
    fn generate_encodable_impl(&self) -> TokenStream {
        let input_type_name = &self.input_type_name;
        let (impl_generics, ty_generics, _) = self.generics.split_for_impl();
        let alloy_primitives = &self.alloy_primitives;
        let alloy_eips = &self.alloy_eips;
        let alloy_rlp = &self.alloy_rlp;

        let variant_names = self.variants.variant_names();
        let variant_types = self.variants.variant_types();

        quote! {
            impl #impl_generics #alloy_eips::Encodable2718 for #input_type_name #ty_generics
            where
                #(#variant_types: #alloy_eips::Encodable2718),*
            {
                fn encode_2718_len(&self) -> usize {
                    match self {
                        #(Self::#variant_names(tx) => tx.encode_2718_len(),)*
                    }
                }

                fn encode_2718(&self, out: &mut dyn #alloy_rlp::BufMut) {
                    match self {
                        #(Self::#variant_names(tx) => tx.encode_2718(out),)*
                    }
                }

                fn trie_hash(&self) -> #alloy_primitives::B256 {
                    match self {
                        #(Self::#variant_names(tx) => tx.trie_hash(),)*
                    }
                }
            }

            impl #impl_generics #alloy_rlp::Encodable for #input_type_name #ty_generics
            where
                #(#variant_types: #alloy_eips::Encodable2718),*
            {
                fn encode(&self, out: &mut dyn #alloy_rlp::BufMut) {
                    <Self as #alloy_eips::Encodable2718>::network_encode(self, out)
                }

                fn length(&self) -> usize {
                    <Self as #alloy_eips::Encodable2718>::network_len(self)
                }
            }

        }
    }

    /// Generate Decodable2718 implementation.
    fn generate_decodable_impl(&self) -> TokenStream {
        let input_type_name = &self.input_type_name;
        let tx_type_enum_name = &self.tx_type_enum_name;
        let (impl_generics, ty_generics, _) = self.generics.split_for_impl();
        let alloy_eips = &self.alloy_eips;
        let alloy_rlp = &self.alloy_rlp;

        let typed_decode_arms = self.variants.all.iter().map(|v| {
            let name = &v.name;
            match &v.kind {
                VariantKind::Flattened => quote! {
                    #tx_type_enum_name::#name(_) => Ok(Self::#name(#alloy_eips::Decodable2718::typed_decode(ty, buf)?))
                },
                VariantKind::Typed(_) => quote! {
                    #tx_type_enum_name::#name => Ok(Self::#name(#alloy_eips::Decodable2718::typed_decode(ty, buf)?))
                },
            }
        });

        let fallback_decode_arms = self.variants.all.iter().map(|v| {
            let name = &v.name;
            quote! {
                if let Ok(tx) = #alloy_eips::Decodable2718::fallback_decode(buf) {
                    return Ok(Self::#name(tx))
                }
            }
        });

        let variant_types = self.variants.variant_types();

        quote! {
            impl #impl_generics #alloy_eips::Decodable2718 for #input_type_name #ty_generics
            where
                #(#variant_types: #alloy_eips::Decodable2718),*
            {
                fn typed_decode(ty: u8, buf: &mut &[u8]) -> #alloy_eips::eip2718::Eip2718Result<Self> {
                    match ty.try_into().map_err(|_| alloy_rlp::Error::Custom("unexpected tx type"))? {
                        #(#typed_decode_arms,)*
                    }
                }

                fn fallback_decode(buf: &mut &[u8]) -> #alloy_eips::eip2718::Eip2718Result<Self> {
                    #(#fallback_decode_arms)*

                    return Err(#alloy_eips::eip2718::Eip2718Error::UnexpectedType(0))
                }
            }

            impl #impl_generics #alloy_rlp::Decodable for #input_type_name #ty_generics
            where
                #(#variant_types: #alloy_eips::Decodable2718),*
            {
                fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
                    Ok(<Self as #alloy_eips::Decodable2718>::network_decode(buf)?)
                }
            }

        }
    }

    /// Generate TransactionEnvelope trait implementation.
    fn generate_envelope_impl(&self) -> TokenStream {
        let input_type_name = &self.input_type_name;
        let tx_type_enum_name = &self.tx_type_enum_name;
        let (impl_generics, ty_generics, _) = self.generics.split_for_impl();
        let alloy_consensus = &self.alloy_consensus;

        quote! {
            impl #impl_generics #alloy_consensus::TransactionEnvelope for #input_type_name #ty_generics
            where
                Self: #alloy_consensus::Transaction
            {
                type TxType = #tx_type_enum_name;
            }
        }
    }

    /// Generate serde implementations if enabled.
    fn generate_serde_impls(&self) -> TokenStream {
        if !self.serde_enabled {
            return quote! {};
        }

        crate::serde::SerdeGenerator::new(
            &self.input_type_name,
            &self.generics,
            &self.variants,
            &self.alloy_consensus,
        )
        .generate()
    }

    /// Generate arbitrary implementations if enabled.
    fn generate_arbitrary_impls(&self) -> TokenStream {
        if !self.arbitrary_enabled {
            return quote! {};
        }

        let input_type_name = &self.input_type_name;
        let tx_type_enum_name = &self.tx_type_enum_name;
        let (impl_generics, ty_generics, _) = self.generics.split_for_impl();
        let alloy_consensus = &self.alloy_consensus;
        let arbitrary = quote! { #alloy_consensus::private::arbitrary };

        let num_variants = self.variants.all.len();

        let tx_type_arms = self.variants.all.iter().enumerate().map(|(i, v)| {
            let name = &v.name;
            match &v.kind {
                VariantKind::Typed(_) => quote! { #i => Ok(Self::#name) },
                VariantKind::Flattened => quote! { #i => Ok(Self::#name(u.arbitrary()?)) },
            }
        });

        let enum_variant_arms = self.variants.all.iter().enumerate().map(|(i, v)| {
            let name = &v.name;
            quote! { #i => Ok(Self::#name(u.arbitrary()?)) }
        });

        let variant_types = self.variants.variant_types();

        quote! {
            impl #arbitrary::Arbitrary<'_> for #tx_type_enum_name {
                fn arbitrary(u: &mut #arbitrary::Unstructured<'_>) -> #arbitrary::Result<Self> {
                    match u.int_in_range(0..=#num_variants-1)? {
                        #(#tx_type_arms,)*
                        _ => unreachable!(),
                    }
                }
            }

            impl #impl_generics #arbitrary::Arbitrary<'_> for #input_type_name #ty_generics
            where
                #(#variant_types: for<'a> #arbitrary::Arbitrary<'a>),*
            {
                fn arbitrary(u: &mut #arbitrary::Unstructured<'_>) -> #arbitrary::Result<Self> {
                    match u.int_in_range(0..=#num_variants-1)? {
                        #(#enum_variant_arms,)*
                        _ => unreachable!(),
                    }
                }
            }
        }
    }
}
