use crate::parse::{GroupedVariants, ProcessedVariant};
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
    serde_json: TokenStream,
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
        let serde_json = quote! { #alloy_consensus::private::serde_json };
        Self { input_type_name, generics, variants, alloy_consensus, serde, serde_json, serde_cfg }
    }

    /// Generate all serde-related code.
    pub(crate) fn generate(&self) -> TokenStream {
        let serialize_bounds = self.generate_serialize_bounds();
        let deserialize_bounds = self.generate_deserialize_bounds();
        let deserialize_bounds_str = deserialize_bounds.to_string();
        let serialize_bounds_str = serialize_bounds.to_string();

        let tagged_enum = self.generate_tagged_enum(&deserialize_bounds_str);
        let tagged_ref_enum = self.generate_tagged_ref_enum(&serialize_bounds_str);
        let untagged_enum = self.generate_untagged_enum();
        let untagged_ref_enum = self.generate_untagged_ref_enum(&serialize_bounds_str);
        let impls = self.generate_serde_impls(&serialize_bounds, &deserialize_bounds);

        let serde_cfg = self.serde_cfg;

        quote! {
            #[cfg(#serde_cfg)]
            const _: () = {
                #tagged_enum
                #tagged_ref_enum
                #untagged_enum
                #untagged_ref_enum
                #impls
            };
        }
    }

    /// Generate serialize bounds.
    fn generate_serialize_bounds(&self) -> TokenStream {
        let variant_types = self.variants.all.iter().map(|v| &v.ty);
        let serde = &self.serde;

        quote! { #(#variant_types: #serde::Serialize),* }
    }

    /// Generate deserialize bounds.
    fn generate_deserialize_bounds(&self) -> TokenStream {
        let variant_types = self.variants.all.iter().map(|v| &v.ty);
        let serde = &self.serde;

        quote! { #(#variant_types: #serde::de::DeserializeOwned),* }
    }

    /// Returns the generics with a leading `'__alloy_ref` lifetime for the borrowed serialization
    /// enums.
    fn borrowed_generics(&self) -> syn::Generics {
        let mut generics = self.generics.clone();
        generics.params.insert(0, syn::parse_quote!('__alloy_ref));
        generics
    }

    /// Generate the tagged transaction types enum.
    fn generate_tagged_enum(&self, serde_bounds_str: &str) -> TokenStream {
        let generics = self.generics;
        let serde = &self.serde;
        let serde_str = serde.to_string();

        let tagged_variants = self.generate_tagged_variants();
        let from_tagged_impl = self.generate_from_tagged_impl();

        quote! {
            #[derive(Debug, #serde::Deserialize)]
            #[serde(tag = "type", bound = #serde_bounds_str, crate = #serde_str)]
            enum TaggedTxTypes #generics {
                #(#tagged_variants),*
            }

            #from_tagged_impl
        }
    }

    /// Generate the borrowed tagged transaction types enum, used to serialize without cloning.
    fn generate_tagged_ref_enum(&self, serialize_bounds_str: &str) -> TokenStream {
        let generics = self.borrowed_generics();
        let serde = &self.serde;
        let serde_str = serde.to_string();

        let tagged_variants = self.generate_tagged_variants_with(|ty| quote! { &'__alloy_ref #ty });

        // Without typed variants nothing else uses the lifetime.
        let maybe_marker = if tagged_variants.is_empty() {
            quote! {
                #[serde(skip)]
                Marker {
                    lifetime: core::marker::PhantomData<&'__alloy_ref ()>,
                    never: core::convert::Infallible,
                }
            }
        } else {
            quote! {}
        };

        quote! {
            #[derive(#serde::Serialize)]
            #[serde(tag = "type", bound = #serialize_bounds_str, crate = #serde_str)]
            enum TaggedTxTypesRef #generics {
                #(#tagged_variants,)*
                #maybe_marker
            }
        }
    }

    /// Generate tagged variants for serde.
    fn generate_tagged_variants(&self) -> Vec<TokenStream> {
        self.generate_tagged_variants_with(|ty| quote! { #ty })
    }

    /// Generate tagged variants for serde, wrapping each variant's type with `wrap`.
    fn generate_tagged_variants_with(
        &self,
        wrap: impl Fn(&syn::Type) -> TokenStream,
    ) -> Vec<TokenStream> {
        self.variants
            .typed
            .iter()
            .map(|v| {
                let ProcessedVariant { name, ty, kind, serde_attrs, typed: _, doc_attrs: _ } = v;

                let (rename, aliases) = kind.serde_tag_and_aliases();

                // Special handling for legacy transactions
                let maybe_with = if v.is_legacy() {
                    let alloy_consensus = &self.alloy_consensus;
                    let path = quote! {
                        #alloy_consensus::transaction::signed_legacy_serde
                    }
                    .to_string();
                    quote! { with = #path, }
                } else {
                    quote! {}
                };

                let maybe_other = serde_attrs.clone().unwrap_or_default();
                let ty = wrap(ty);

                quote! {
                    #[serde(rename = #rename, #(alias = #aliases,)* #maybe_with #maybe_other)]
                    #name(#ty)
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
    fn generate_untagged_enum(&self) -> TokenStream {
        let generics = self.generics;

        let (legacy_variant, legacy_arm, legacy_deserialize) = self.generate_legacy_handling();
        let untagged_variants =
            self.generate_untagged_variants(&legacy_variant, false, |ty| quote! { #ty });
        let untagged_conversions = self.generate_untagged_conversions(&legacy_arm);
        let deserialize_impl = self.generate_untagged_deserialize(&legacy_deserialize);

        quote! {
            pub(crate) enum UntaggedTxTypes #generics {
                Tagged(TaggedTxTypes #generics),
                #untagged_variants
            }

            #deserialize_impl
            #untagged_conversions
        }
    }

    /// Generate the borrowed untagged transaction types enum, used to serialize without cloning.
    fn generate_untagged_ref_enum(&self, serialize_bounds_str: &str) -> TokenStream {
        let generics = self.borrowed_generics();
        let (_, ty_generics, _) = generics.split_for_impl();
        let serde = &self.serde;
        let serde_str = serde.to_string();
        let input_type_name = self.input_type_name;
        let (_, env_ty_generics, _) = self.generics.split_for_impl();
        let unwrapped_generics = &self.generics.params;

        let untagged_variants =
            self.generate_untagged_variants(&quote! {}, true, |ty| quote! { &'__alloy_ref #ty });
        let flattened_names = self.variants.flattened.iter().map(|v| &v.name).collect::<Vec<_>>();
        let typed_names = self.variants.typed.iter().map(|v| &v.name).collect::<Vec<_>>();

        quote! {
            #[derive(#serde::Serialize)]
            #[serde(untagged, bound = #serialize_bounds_str, crate = #serde_str)]
            enum UntaggedTxTypesRef #generics {
                Tagged(TaggedTxTypesRef #ty_generics),
                #untagged_variants
            }

            impl<'__alloy_ref, #unwrapped_generics> From<&'__alloy_ref #input_type_name #env_ty_generics> for UntaggedTxTypesRef #ty_generics {
                fn from(value: &'__alloy_ref #input_type_name #env_ty_generics) -> Self {
                    match value {
                        #(
                            #input_type_name::<#unwrapped_generics>::#flattened_names(value) => Self::#flattened_names(value),
                        )*
                        #(
                            #input_type_name::<#unwrapped_generics>::#typed_names(value) => Self::Tagged(TaggedTxTypesRef::#typed_names(value)),
                        )*
                    }
                }
            }
        }
    }

    /// Generate untagged variants. This includes flattened envelopes and legacy transactions.
    ///
    /// The variants' `#[serde(...)]` attributes are only forwarded to enums that derive serde.
    fn generate_untagged_variants(
        &self,
        legacy_variant: &TokenStream,
        forward_serde_attrs: bool,
        wrap: impl Fn(&syn::Type) -> TokenStream,
    ) -> TokenStream {
        let flattened_variants = self.variants.flattened.iter().map(|v| {
            let name = &v.name;
            let ty = wrap(&v.ty);

            let maybe_attributes = match &v.serde_attrs {
                Some(attrs) if forward_serde_attrs => quote! { #[serde(#attrs)] },
                _ => quote! {},
            };

            quote! { #maybe_attributes #name(#ty) }
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
        let (_, ty_generics, _) = generics.split_for_impl();
        let unwrapped_generics = &generics.params;
        let serde = &self.serde;
        let serde_json = &self.serde_json;
        let serde_bounds = self.generate_deserialize_bounds();

        let flattened_names = self.variants.flattened.iter().map(|v| &v.name);

        let type_first = self.generate_type_first_dispatch();

        quote! {
            impl #generics UntaggedTxTypes #ty_generics where #serde_bounds {
                // Manually modified derived serde(untagged) to preserve the error of the
                // TaggedTxTypes attempt.
                fn deserialize_value<E: #serde::de::Error>(value: #serde_json::Value) -> Result<Self, E> {
                    use #serde::Deserialize;

                    let deserializer = &value;

                    let tagged_res =
                        TaggedTxTypes::<#unwrapped_generics>::deserialize(deserializer).map(Self::Tagged).map_err(E::custom);

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
                    //  > "data did not match any variant of untagged enum UntaggedTxTypes"
                    tagged_res
                }
            }

            impl<'de, #unwrapped_generics> #serde::Deserialize<'de> for UntaggedTxTypes #generics where #serde_bounds {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: #serde::Deserializer<'de>,
                {
                    #type_first

                    Self::deserialize_value(#serde_json::Value::deserialize(deserializer)?)
                }
            }
        }
    }

    /// Generate the type-first dispatch used by the untagged deserializer.
    ///
    /// When the input object starts with a `type` key holding a known tag, the object is
    /// deserialized straight into that variant, without buffering it. Everything else is
    /// buffered into a `serde_json::Value` and goes through the generic path.
    ///
    /// The dispatch is only generated if there are no flattened variants, which could accept
    /// an object that the tagged variant rejects, and only covers typed variants without
    /// custom serde attributes.
    fn generate_type_first_dispatch(&self) -> TokenStream {
        let serde = &self.serde;
        let serde_json = &self.serde_json;
        let alloy_consensus = self.alloy_consensus;
        let generics = self.generics;
        let (_, ty_generics, _) = generics.split_for_impl();
        let unwrapped_generics = &generics.params;
        let serde_bounds = self.generate_deserialize_bounds();

        if !self.variants.flattened.is_empty() {
            return quote! {};
        }

        let arms = self.variants.typed.iter().filter(|v| v.serde_attrs.is_none()).map(|v| {
            let ProcessedVariant { name, ty, kind, .. } = v;
            let (rename, aliases) = kind.serde_tag_and_aliases();
            let deserialize = if v.is_legacy() {
                quote! { #alloy_consensus::transaction::signed_legacy_serde::deserialize(map_deserializer) }
            } else {
                quote! { <#ty as #serde::Deserialize>::deserialize(map_deserializer) }
            };

            quote! {
                #rename #(| #aliases)* => {
                    let map_deserializer = #serde::de::value::MapAccessDeserializer::new(map);
                    return #deserialize.map(|tx| UntaggedTxTypes::Tagged(TaggedTxTypes::#name(tx)));
                }
            }
        });

        quote! {
            /// A map key, with the `type` key recognized without allocating.
            enum FirstKey {
                Type,
                Other(#serde_json::Value),
            }

            impl<'de> #serde::Deserialize<'de> for FirstKey {
                fn deserialize<D: #serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    struct KeyVisitor;

                    impl<'de> #serde::de::Visitor<'de> for KeyVisitor {
                        type Value = FirstKey;

                        fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                            f.write_str("a string key")
                        }

                        fn visit_str<E: #serde::de::Error>(self, key: &str) -> Result<FirstKey, E> {
                            Ok(if key == "type" { FirstKey::Type } else { FirstKey::Other(key.into()) })
                        }
                    }

                    deserializer.deserialize_str(KeyVisitor)
                }
            }

            struct TypeFirstVisitor<#unwrapped_generics>(core::marker::PhantomData<fn() -> UntaggedTxTypes #ty_generics>);

            impl<'de, #unwrapped_generics> #serde::de::Visitor<'de> for TypeFirstVisitor #ty_generics where #serde_bounds {
                type Value = UntaggedTxTypes #ty_generics;

                fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.write_str("a transaction object")
                }

                fn visit_map<M: #serde::de::MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
                    let mut object = #serde_json::Map::new();

                    match map.next_key::<FirstKey>()? {
                        None => {}
                        Some(FirstKey::Type) => {
                            let tag = map.next_value::<#serde_json::Value>()?;
                            if let #serde_json::Value::String(tag) = &tag {
                                match tag.as_str() {
                                    #(#arms)*
                                    _ => {}
                                }
                            }
                            object.insert("type".into(), tag);
                        }
                        Some(FirstKey::Other(#serde_json::Value::String(key))) => {
                            object.insert(key, map.next_value()?);
                        }
                        Some(FirstKey::Other(_)) => unreachable!(),
                    }

                    while let Some((key, value)) = map.next_entry()? {
                        object.insert(key, value);
                    }

                    UntaggedTxTypes::deserialize_value(#serde_json::Value::Object(object))
                }

                fn visit_seq<A: #serde::de::SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
                    let value = <#serde_json::Value as #serde::Deserialize>::deserialize(#serde::de::value::SeqAccessDeserializer::new(seq))?;

                    UntaggedTxTypes::deserialize_value(value)
                }
            }

            // Non human readable formats deserialize from a `serde_json::Value` buffer, which is
            // always human readable, so they take the generic path.
            if deserializer.is_human_readable() {
                return deserializer.deserialize_any(TypeFirstVisitor::<#unwrapped_generics>(core::marker::PhantomData));
            }
        }
    }

    /// Generate conversion implementations for untagged types.
    fn generate_untagged_conversions(&self, legacy_arm: &TokenStream) -> TokenStream {
        let input_type_name = self.input_type_name;
        let (impl_generics, ty_generics, _) = self.generics.split_for_impl();
        let flattened_names = self.variants.flattened.iter().map(|v| &v.name).collect::<Vec<_>>();

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
        }
    }

    /// Generate Serialize and Deserialize implementations.
    fn generate_serde_impls(
        &self,
        serialize_bounds: &TokenStream,
        deserialize_bounds: &TokenStream,
    ) -> TokenStream {
        let input_type_name = self.input_type_name;
        let serde = &self.serde;
        let (impl_generics, ty_generics, _) = self.generics.split_for_impl();
        let unwrapped_generics = &self.generics.params;

        quote! {
            impl #impl_generics #serde::Serialize for #input_type_name #ty_generics where #serialize_bounds {
                fn serialize<S: #serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    UntaggedTxTypesRef::<'_, #unwrapped_generics>::from(self).serialize(serializer)
                }
            }

            impl <'de, #unwrapped_generics> #serde::Deserialize<'de> for #input_type_name #ty_generics where #deserialize_bounds {
                fn deserialize<D: #serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    UntaggedTxTypes::<#unwrapped_generics>::deserialize(deserializer).map(Into::into)
                }
            }
        }
    }
}
