use crate::parse::{GroupedVariants, ProcessedVariant, VariantKind};
use alloy_primitives::U8;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Path};

/// Generate serde implementations for the transaction envelope.
pub(crate) struct SerdeGenerator<'a> {
    input_type_name: &'a Ident,
    generics: &'a syn::Generics,
    variants: &'a GroupedVariants,
    alloy_consensus: &'a Path,
    serde: TokenStream,
    serde_cfg: &'a TokenStream,
}

impl<'a> SerdeGenerator<'a> {
    pub(crate) fn new(
        input_type_name: &'a Ident,
        generics: &'a syn::Generics,
        variants: &'a GroupedVariants,
        alloy_consensus: &'a Path,
        serde_cfg: &'a TokenStream,
    ) -> Self {
        let serde = quote! { #alloy_consensus::private::serde };
        Self { input_type_name, generics, variants, alloy_consensus, serde, serde_cfg }
    }

    /// Generate all serde-related code.
    pub(crate) fn generate(&self) -> TokenStream {
        let serde_bounds = self.generate_serde_bounds();
        let serde_bounds_str = serde_bounds.to_string();

        let tagged_enum = self.generate_tagged_enum(&serde_bounds_str);
        let untagged_enum = self.generate_untagged_enum(&serde_bounds_str);
        let impls = self.generate_serde_impls(&serde_bounds);

        let serde_cfg = self.serde_cfg;

        quote! {
            #[cfg(#serde_cfg)]
            const _: () = {
                #tagged_enum
                #untagged_enum
                #impls
            };
        }
    }

    /// Generate serde bounds.
    fn generate_serde_bounds(&self) -> TokenStream {
        let input_type_name = self.input_type_name;
        let (_, ty_generics, _) = self.generics.split_for_impl();
        let variant_types = self.variants.all.iter().map(|v| &v.ty);
        let serde = &self.serde;

        quote! {
            #input_type_name #ty_generics: Clone,
            #(#variant_types: #serde::Serialize + #serde::de::DeserializeOwned),*
        }
    }

    /// Generate the tagged transaction types enum.
    fn generate_tagged_enum(&self, serde_bounds_str: &str) -> TokenStream {
        let generics = self.generics;
        let serde = &self.serde;
        let serde_str = serde.to_string();

        let tagged_variants = self.generate_tagged_variants();
        let from_tagged_impl = self.generate_from_tagged_impl();

        quote! {
            #[derive(Debug, #serde::Serialize, #serde::Deserialize)]
            #[serde(tag = "type", bound = #serde_bounds_str, crate = #serde_str)]
            enum TaggedTxTypes #generics {
                #(#tagged_variants),*
            }

            #from_tagged_impl
        }
    }

    /// Generate tagged variants for serde.
    fn generate_tagged_variants(&self) -> Vec<TokenStream> {
        self.variants
            .typed
            .iter()
            .filter_map(|v| {
                let ProcessedVariant { name, ty, kind } = v;

                if let VariantKind::Typed(tx_type) = kind {
                    let tx_type = U8::from(*tx_type);
                    let rename = format!("0x{tx_type:x}");

                    // Add alias for single digit hex values (e.g., "0x0" for "0x00")
                    let maybe_alias = if rename.len() == 3 {
                        let alias = format!("0x0{}", rename.chars().last().unwrap());
                        quote! { , alias = #alias }
                    } else {
                        quote! {}
                    };

                    // Special handling for legacy transactions
                    let maybe_with = if v.is_legacy() {
                        let alloy_consensus = &self.alloy_consensus;
                        let path = quote! {
                            #alloy_consensus::transaction::signed_legacy_serde
                        }
                        .to_string();
                        quote! { , with = #path }
                    } else {
                        quote! {}
                    };

                    Some(quote! {
                        #[serde(rename = #rename #maybe_alias #maybe_with)]
                        #name(#ty)
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Generate From implementation for tagged types.
    fn generate_from_tagged_impl(&self) -> TokenStream {
        let input_type_name = self.input_type_name;
        let (impl_generics, ty_generics, _) = self.generics.split_for_impl();
        let unwrapped_generics = &self.generics.params;

        let typed_names = self.variants.typed.iter().map(|v| &v.name).collect::<Vec<_>>();

        quote! {
            impl #impl_generics From<TaggedTxTypes #ty_generics> for #input_type_name #ty_generics {
                fn from(value: TaggedTxTypes #ty_generics) -> Self {
                    match value {
                        #(
                            TaggedTxTypes::<#unwrapped_generics>::#typed_names(value) => Self::#typed_names(value),
                        )*
                    }
                }
            }
        }
    }

    /// Generate the untagged transaction types enum.
    fn generate_untagged_enum(&self, serde_bounds_str: &str) -> TokenStream {
        let generics = self.generics;
        let serde = &self.serde;
        let serde_str = serde.to_string();

        let (legacy_variant, legacy_arm, legacy_deserialize) = self.generate_legacy_handling();
        let untagged_variants = self.generate_untagged_variants(&legacy_variant);
        let untagged_conversions = self.generate_untagged_conversions(&legacy_arm);
        let deserialize_impl = self.generate_untagged_deserialize(&legacy_deserialize);

        quote! {
            #[derive(#serde::Serialize)]
            #[serde(untagged, bound = #serde_bounds_str, crate = #serde_str)]
            pub(crate) enum UntaggedTxTypes #generics {
                Tagged(TaggedTxTypes #generics),
                #untagged_variants
            }

            #deserialize_impl
            #untagged_conversions
        }
    }

    /// Generate untagged variants. This includes flattened envelopes and legacy transactions.
    fn generate_untagged_variants(&self, legacy_variant: &TokenStream) -> TokenStream {
        let flattened_variants = self.variants.flattened.iter().map(|v| {
            let name = &v.name;
            let ty = &v.ty;
            quote! { #name(#ty) }
        });

        quote! {
            #(#flattened_variants,)*
            #legacy_variant
        }
    }

    /// Generate legacy transaction handling for serde.
    fn generate_legacy_handling(&self) -> (TokenStream, TokenStream, TokenStream) {
        if let Some(legacy) = self.variants.legacy_variant() {
            let ty = &legacy.ty;
            let name = &legacy.name;
            let alloy_consensus = self.alloy_consensus;

            let variant = quote! { UntaggedLegacy(#ty) };
            let arm = quote! { UntaggedTxTypes::UntaggedLegacy(tx) => Self::#name(tx), };
            let deserialize = quote! {
                if let Ok(val) = #alloy_consensus::transaction::untagged_legacy_serde::deserialize(deserializer).map(Self::UntaggedLegacy) {
                    return Ok(val);
                }
            };

            (variant, arm, deserialize)
        } else {
            (quote! {}, quote! {}, quote! {})
        }
    }

    /// Generate custom deserialize implementation for untagged types.
    fn generate_untagged_deserialize(&self, legacy_deserialize: &TokenStream) -> TokenStream {
        let generics = self.generics;
        let unwrapped_generics = &generics.params;
        let serde = &self.serde;
        let serde_bounds = self.generate_serde_bounds();

        let flattened_names = self.variants.flattened.iter().map(|v| &v.name);

        quote! {
            // Manually modified derived serde(untagged) to preserve the error of the TaggedTxEnvelope
            // attempt. Note: This uses private serde API
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

                    // proceed with flattened variants
                    #(
                        if let Ok(val) = #serde::Deserialize::deserialize(deserializer).map(Self::#flattened_names) {
                            return Ok(val);
                        }
                    )*

                    #legacy_deserialize

                    // return the original error, which is more useful than the untagged error
                    //  > "data did not match any variant of untagged enum MaybeTaggedTxEnvelope"
                    tagged_res
                }
            }
        }
    }

    /// Generate conversion implementations for untagged types.
    fn generate_untagged_conversions(&self, legacy_arm: &TokenStream) -> TokenStream {
        let input_type_name = self.input_type_name;
        let (impl_generics, ty_generics, _) = self.generics.split_for_impl();
        let unwrapped_generics = &self.generics.params;
        let flattened_names = self.variants.flattened.iter().map(|v| &v.name).collect::<Vec<_>>();
        let typed_names = self.variants.typed.iter().map(|v| &v.name).collect::<Vec<_>>();

        quote! {
            impl #impl_generics From<UntaggedTxTypes #ty_generics> for #input_type_name #ty_generics {
                fn from(value: UntaggedTxTypes #ty_generics) -> Self {
                    match value {
                        UntaggedTxTypes::Tagged(value) => value.into(),
                        #(
                            UntaggedTxTypes::#flattened_names(value) => Self::#flattened_names(value),
                        )*
                        #legacy_arm
                    }
                }
            }

            impl #impl_generics From<#input_type_name #ty_generics> for UntaggedTxTypes #ty_generics {
                fn from(value: #input_type_name #ty_generics) -> Self {
                    match value {
                        #(
                            #input_type_name::<#unwrapped_generics>::#flattened_names(value) => Self::#flattened_names(value),
                        )*
                        #(
                            #input_type_name::<#unwrapped_generics>::#typed_names(value) => Self::Tagged(TaggedTxTypes::#typed_names(value)),
                        )*
                    }
                }
            }
        }
    }

    /// Generate Deserialize implementation.
    fn generate_serde_impls(&self, serde_bounds: &TokenStream) -> TokenStream {
        let input_type_name = self.input_type_name;
        let serde = &self.serde;
        let (impl_generics, ty_generics, _) = self.generics.split_for_impl();
        let unwrapped_generics = &self.generics.params;

        quote! {
            impl #impl_generics #serde::Serialize for #input_type_name #ty_generics where #serde_bounds {
                fn serialize<S: #serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    UntaggedTxTypes::<#unwrapped_generics>::from(self.clone()).serialize(serializer)
                }
            }

            impl <'de, #unwrapped_generics> #serde::Deserialize<'de> for #input_type_name #ty_generics where #serde_bounds {
                fn deserialize<D: #serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    UntaggedTxTypes::<#unwrapped_generics>::deserialize(deserializer).map(Into::into)
                }
            }
        }
    }
}
