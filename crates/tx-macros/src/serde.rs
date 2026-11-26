use crate::parse::{GroupedVariants, ProcessedVariant};
use proc_macro2::{TokenStream, TokenTree};
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

        let (legacy_variant, legacy_arm, legacy_attempt_variant, legacy_attempt_arm) =
            self.generate_legacy_handling();
        let untagged_variants =
            self.generate_untagged_variants(&legacy_variant, false, |ty| quote! { #ty });
        let untagged_conversions = self.generate_untagged_conversions(&legacy_arm);
        let deserialize_impl =
            self.generate_untagged_deserialize(&legacy_attempt_variant, &legacy_attempt_arm);

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
    ///
    /// Returns the `UntaggedTxTypes` variant, its conversion arm, and the variant and arm of the
    /// buffered deserialization attempt.
    fn generate_legacy_handling(&self) -> (TokenStream, TokenStream, TokenStream, TokenStream) {
        if let Some(legacy) = self.variants.legacy_variant() {
            let ty = &legacy.ty;
            let name = &legacy.name;
            let alloy_consensus = self.alloy_consensus;
            let deserialize_with = quote! {
                #alloy_consensus::transaction::untagged_legacy_serde::deserialize
            }
            .to_string();

            let variant = quote! { UntaggedLegacy(#ty) };
            let arm = quote! { UntaggedTxTypes::UntaggedLegacy(tx) => Self::#name(tx), };
            let attempt_variant = quote! {
                #[serde(deserialize_with = #deserialize_with)]
                __UntaggedLegacy(#ty),
            };
            let attempt_arm =
                quote! { __Attempt::__UntaggedLegacy(tx) => Ok(Self::UntaggedLegacy(tx)), };

            (variant, arm, attempt_variant, attempt_arm)
        } else {
            (quote! {}, quote! {}, quote! {}, quote! {})
        }
    }

    /// Generate custom deserialize implementation for untagged types.
    ///
    /// The object is buffered once and the tagged variants, the flattened variants and the
    /// untagged legacy transaction are tried on the buffer, in that order. An object that
    /// starts with a `type` key holding a known tag skips the buffer, see
    /// [`Self::generate_type_first_dispatch`].
    fn generate_untagged_deserialize(
        &self,
        legacy_attempt_variant: &TokenStream,
        legacy_attempt_arm: &TokenStream,
    ) -> TokenStream {
        let generics = self.generics;
        let (_, ty_generics, _) = generics.split_for_impl();
        let unwrapped_generics = &generics.params;
        let serde = &self.serde;
        let serde_str = serde.to_string();
        let serde_json = &self.serde_json;
        let serde_bounds = self.generate_deserialize_bounds();

        // The attempt is declared inside a function, so it needs generics of its own.
        let mut attempt_generics = generics.clone();
        attempt_generics.params.push(syn::parse_quote!(__E));
        let attempt_bounds_str = quote! { __E: #serde::de::Error, #serde_bounds }.to_string();

        let flattened_names = self.variants.flattened.iter().map(|v| &v.name).collect::<Vec<_>>();
        let flattened_tys = self.variants.flattened.iter().map(|v| &v.ty);

        let type_first = self.generate_type_first_dispatch();
        // Only the dispatch needs to know the format and to change the entries.
        let (human_readable_field, human_readable_let, human_readable_init, entries_mut) =
            if type_first.is_some() {
                (
                    quote! { human_readable: bool, },
                    quote! { let human_readable = deserializer.is_human_readable(); },
                    quote! { human_readable, },
                    quote! { mut },
                )
            } else {
                (quote! {}, quote! {}, quote! {}, quote! {})
            };

        // The items declared inside the functions below share a scope with the types of the
        // variants, so their names start with `__`, like the generics next to those of the
        // envelope.
        quote! {
            impl #generics UntaggedTxTypes #ty_generics where #serde_bounds {
                /// Buffers the input and tries the tagged variants, the flattened variants and the
                /// untagged legacy transaction on it, in that order.
                fn deserialize_buffered<'de, D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: #serde::Deserializer<'de>,
                {
                    /// The error of the tagged attempt, which is more useful than the untagged
                    /// error
                    ///  > "data did not match any variant of untagged enum __Attempt"
                    struct __TaggedError<T, E>(E, core::marker::PhantomData<T>);

                    impl<'de, T, E> #serde::Deserialize<'de> for __TaggedError<T, E>
                    where
                        T: #serde::Deserialize<'de>,
                        E: #serde::de::Error,
                    {
                        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                        where
                            D: #serde::Deserializer<'de>,
                        {
                            match T::deserialize(deserializer) {
                                Ok(_) => Err(#serde::de::Error::custom("tagged attempt succeeded on retry")),
                                Err(err) => Ok(Self(E::custom(err), core::marker::PhantomData)),
                            }
                        }
                    }

                    // `untagged` buffers the input once and lets every variant read the buffer
                    // by reference. `__Failed` is last: it is reached only if the tagged attempt
                    // failed, and repeats it to keep its error.
                    #[derive(#serde::Deserialize)]
                    #[serde(untagged, bound = #attempt_bounds_str, crate = #serde_str)]
                    enum __Attempt #attempt_generics {
                        __Tagged(TaggedTxTypes #ty_generics),
                        #(#flattened_names(#flattened_tys),)*
                        #legacy_attempt_variant
                        __Failed(__TaggedError<TaggedTxTypes #ty_generics, __E>),
                    }

                    match #serde::Deserialize::deserialize(deserializer)? {
                        __Attempt::__Tagged(tx) => Ok(Self::Tagged(tx)),
                        #(__Attempt::#flattened_names(tx) => Ok(Self::#flattened_names(tx)),)*
                        #legacy_attempt_arm
                        __Attempt::__Failed(__TaggedError(err, _)) => Err(err),
                    }
                }
            }

            impl<'de, #unwrapped_generics> #serde::Deserialize<'de> for UntaggedTxTypes #generics where #serde_bounds {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: #serde::Deserializer<'de>,
                {
                    /// A key of the object, with the `type` key recognized without allocating.
                    enum __Key<'de> {
                        Type,
                        Borrowed(&'de str),
                        Owned(#serde_json::Value),
                    }

                    impl<'de> #serde::Deserialize<'de> for __Key<'de> {
                        fn deserialize<D: #serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                            struct __KeyVisitor;

                            impl<'de> #serde::de::Visitor<'de> for __KeyVisitor {
                                type Value = __Key<'de>;

                                fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                                    f.write_str("a string key")
                                }

                                fn visit_borrowed_str<E: #serde::de::Error>(self, key: &'de str) -> Result<__Key<'de>, E> {
                                    Ok(if key == "type" { __Key::Type } else { __Key::Borrowed(key) })
                                }

                                fn visit_str<E: #serde::de::Error>(self, key: &str) -> Result<__Key<'de>, E> {
                                    Ok(if key == "type" { __Key::Type } else { __Key::Owned(key.into()) })
                                }
                            }

                            deserializer.deserialize_str(__KeyVisitor)
                        }
                    }

                    /// The value of a `type` key, or the first element of an array.
                    enum __Tag<'de> {
                        Borrowed(&'de str),
                        Other(#serde_json::Value),
                    }

                    impl<'de> #serde::Deserialize<'de> for __Tag<'de> {
                        fn deserialize<D: #serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                            struct __TagVisitor;

                            // A tag is a string that mostly borrows from the input. Anything else
                            // is kept as the value `serde_json` reads it as.
                            impl<'de> #serde::de::Visitor<'de> for __TagVisitor {
                                type Value = __Tag<'de>;

                                fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                                    f.write_str("any valid JSON value")
                                }

                                fn visit_borrowed_str<E: #serde::de::Error>(self, tag: &'de str) -> Result<__Tag<'de>, E> {
                                    Ok(__Tag::Borrowed(tag))
                                }

                                fn visit_str<E: #serde::de::Error>(self, tag: &str) -> Result<__Tag<'de>, E> {
                                    Ok(__Tag::Other(tag.into()))
                                }

                                fn visit_bool<E: #serde::de::Error>(self, tag: bool) -> Result<__Tag<'de>, E> {
                                    Ok(__Tag::Other(tag.into()))
                                }

                                fn visit_i64<E: #serde::de::Error>(self, tag: i64) -> Result<__Tag<'de>, E> {
                                    Ok(__Tag::Other(tag.into()))
                                }

                                fn visit_u64<E: #serde::de::Error>(self, tag: u64) -> Result<__Tag<'de>, E> {
                                    Ok(__Tag::Other(tag.into()))
                                }

                                fn visit_f64<E: #serde::de::Error>(self, tag: f64) -> Result<__Tag<'de>, E> {
                                    Ok(__Tag::Other(tag.into()))
                                }

                                fn visit_unit<E: #serde::de::Error>(self) -> Result<__Tag<'de>, E> {
                                    Ok(__Tag::Other(#serde_json::Value::Null))
                                }

                                fn visit_seq<A: #serde::de::SeqAccess<'de>>(self, seq: A) -> Result<__Tag<'de>, A::Error> {
                                    #serde::Deserialize::deserialize(#serde::de::value::SeqAccessDeserializer::new(seq)).map(__Tag::Other)
                                }

                                fn visit_map<A: #serde::de::MapAccess<'de>>(self, map: A) -> Result<__Tag<'de>, A::Error> {
                                    #serde::Deserialize::deserialize(#serde::de::value::MapAccessDeserializer::new(map)).map(__Tag::Other)
                                }
                            }

                            deserializer.deserialize_any(__TagVisitor)
                        }
                    }

                    impl<'de> __Tag<'de> {
                        /// Hands the tag to a reader of the buffered input.
                        fn replay<S, E>(self, seed: S) -> Result<S::Value, E>
                        where
                            S: #serde::de::DeserializeSeed<'de>,
                            E: #serde::de::Error,
                        {
                            match self {
                                Self::Borrowed(tag) => seed.deserialize(#serde::de::value::BorrowedStrDeserializer::new(tag)),
                                // A buffered unsigned integer selects the variant at that position.
                                // A signed one is rejected like every other tag that is no string.
                                Self::Other(#serde_json::Value::Number(tag)) => match tag.as_i64() {
                                    Some(tag) => seed.deserialize(#serde::de::value::I64Deserializer::new(tag)),
                                    None => seed.deserialize(tag).map_err(E::custom),
                                },
                                Self::Other(tag) => seed.deserialize(tag).map_err(E::custom),
                            }
                        }
                    }

                    /// The entries of the object as its readers get them: the entry that was read
                    /// ahead of `map` first, and `type` values in a form that cannot select a
                    /// variant by position.
                    struct __Entries<'de, M> {
                        /// A key that was read ahead.
                        key: Option<__Key<'de>>,
                        /// A `type` value that was read ahead.
                        tag: Option<__Tag<'de>>,
                        /// Whether the first `type` value already selected the variant, which
                        /// makes another `type` key a duplicate.
                        dispatched: bool,
                        is_type: bool,
                        done: bool,
                        map: M,
                    }

                    impl<'de, M: #serde::de::MapAccess<'de>> #serde::de::MapAccess<'de> for __Entries<'de, M> {
                        type Error = M::Error;

                        fn next_key_seed<K: #serde::de::DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>, M::Error> {
                            use #serde::de::value::BorrowedStrDeserializer;

                            let key = match self.key.take() {
                                Some(key) => key,
                                None if self.done => return Ok(None),
                                None => match self.map.next_key()? {
                                    Some(key) => key,
                                    None => {
                                        self.done = true;
                                        return Ok(None);
                                    }
                                },
                            };
                            self.is_type = matches!(key, __Key::Type);
                            match key {
                                __Key::Type if self.dispatched => Err(#serde::de::Error::duplicate_field("type")),
                                __Key::Type => seed.deserialize(BorrowedStrDeserializer::new("type")),
                                __Key::Borrowed(key) => seed.deserialize(BorrowedStrDeserializer::new(key)),
                                __Key::Owned(key) => seed.deserialize(key).map_err(#serde::de::Error::custom),
                            }
                            .map(Some)
                        }

                        fn next_value_seed<V: #serde::de::DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, M::Error> {
                            let tag = match self.tag.take() {
                                Some(tag) => tag,
                                None if self.is_type => self.map.next_value()?,
                                None => return self.map.next_value_seed(seed),
                            };
                            tag.replay(seed)
                        }
                    }

                    /// The elements of an array as its readers get them. serde reads the first
                    /// one as the tag, so it is handed out like a `type` value.
                    struct __Elements<A> {
                        first: bool,
                        seq: A,
                    }

                    impl<'de, A: #serde::de::SeqAccess<'de>> #serde::de::SeqAccess<'de> for __Elements<A> {
                        type Error = A::Error;

                        fn next_element_seed<T: #serde::de::DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>, A::Error> {
                            if !core::mem::take(&mut self.first) {
                                return self.seq.next_element_seed(seed);
                            }
                            match self.seq.next_element::<__Tag<'de>>()? {
                                Some(tag) => tag.replay(seed).map(Some),
                                None => Ok(None),
                            }
                        }

                        fn size_hint(&self) -> Option<usize> {
                            self.seq.size_hint()
                        }
                    }

                    struct __EnvelopeVisitor<#unwrapped_generics> {
                        #human_readable_field
                        marker: core::marker::PhantomData<fn() -> UntaggedTxTypes #ty_generics>,
                    }

                    impl<'de, #unwrapped_generics> #serde::de::Visitor<'de> for __EnvelopeVisitor #ty_generics where #serde_bounds {
                        type Value = UntaggedTxTypes #ty_generics;

                        fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                            f.write_str("a transaction object")
                        }

                        // Every other value goes to the buffer as well: a flattened variant may
                        // accept it.

                        fn visit_bool<__E: #serde::de::Error>(self, value: bool) -> Result<Self::Value, __E> {
                            UntaggedTxTypes::deserialize_buffered(#serde::de::value::BoolDeserializer::new(value))
                        }

                        fn visit_i64<__E: #serde::de::Error>(self, value: i64) -> Result<Self::Value, __E> {
                            UntaggedTxTypes::deserialize_buffered(#serde::de::value::I64Deserializer::new(value))
                        }

                        fn visit_u64<__E: #serde::de::Error>(self, value: u64) -> Result<Self::Value, __E> {
                            UntaggedTxTypes::deserialize_buffered(#serde::de::value::U64Deserializer::new(value))
                        }

                        fn visit_f64<__E: #serde::de::Error>(self, value: f64) -> Result<Self::Value, __E> {
                            UntaggedTxTypes::deserialize_buffered(#serde::de::value::F64Deserializer::new(value))
                        }

                        fn visit_str<__E: #serde::de::Error>(self, value: &str) -> Result<Self::Value, __E> {
                            UntaggedTxTypes::deserialize_buffered(#serde::de::value::StrDeserializer::new(value))
                        }

                        fn visit_unit<__E: #serde::de::Error>(self) -> Result<Self::Value, __E> {
                            UntaggedTxTypes::deserialize_buffered(#serde::de::value::UnitDeserializer::new())
                        }

                        fn visit_map<__M: #serde::de::MapAccess<'de>>(self, map: __M) -> Result<Self::Value, __M::Error> {
                            let #entries_mut entries = __Entries {
                                key: None,
                                tag: None,
                                dispatched: false,
                                is_type: false,
                                done: false,
                                map,
                            };

                            #type_first

                            UntaggedTxTypes::deserialize_buffered(#serde::de::value::MapAccessDeserializer::new(entries))
                        }

                        fn visit_seq<__A: #serde::de::SeqAccess<'de>>(self, seq: __A) -> Result<Self::Value, __A::Error> {
                            UntaggedTxTypes::deserialize_buffered(#serde::de::value::SeqAccessDeserializer::new(__Elements { first: true, seq }))
                        }
                    }

                    #human_readable_let
                    deserializer.deserialize_any(__EnvelopeVisitor::<#unwrapped_generics> {
                        #human_readable_init
                        marker: core::marker::PhantomData,
                    })
                }
            }
        }
    }

    /// Generate the type-first dispatch used by the untagged deserializer.
    ///
    /// When the object starts with a `type` key holding a known tag, the rest of it is
    /// deserialized straight into that variant, without buffering it. Everything else goes to
    /// the buffer, including the entry that was already read.
    ///
    /// The dispatch is only generated if no other variant can take over the object or the tag:
    /// there are no flattened variants, which could accept an object that the tagged variant
    /// rejects, and no typed variant is `untagged` or has an `alias`. It only covers typed
    /// variants without custom serde attributes.
    fn generate_type_first_dispatch(&self) -> Option<TokenStream> {
        let serde = &self.serde;
        let serde_json = &self.serde_json;
        let alloy_consensus = self.alloy_consensus;

        if !self.variants.flattened.is_empty() {
            return None;
        }

        let changes_matching = |attrs: &TokenStream| {
            attrs.clone().into_iter().any(|token| {
                matches!(token, TokenTree::Ident(ident) if ident == "untagged" || ident == "alias")
            })
        };
        if self.variants.typed.iter().filter_map(|v| v.serde_attrs.as_ref()).any(changes_matching) {
            return None;
        }

        let arms = self.variants.typed.iter().filter(|v| v.serde_attrs.is_none()).map(|v| {
            let ProcessedVariant { name, ty, kind, .. } = v;
            let (rename, aliases) = kind.serde_tag_and_aliases();
            let deserialize = if v.is_legacy() {
                quote! { #alloy_consensus::transaction::signed_legacy_serde::deserialize(rest) }
            } else {
                quote! { <#ty as #serde::Deserialize>::deserialize(rest) }
            };

            quote! {
                Some(#rename #(| #aliases)*) => {
                    entries.dispatched = true;
                    let rest = #alloy_consensus::private::TaggedEntries(entries);
                    return #deserialize.map(|tx| UntaggedTxTypes::Tagged(TaggedTxTypes::#name(tx)));
                }
            }
        });

        Some(quote! {
            // Buffered input is always read as human readable, so other formats take the
            // buffer to keep decoding the same way.
            if self.human_readable {
                match entries.map.next_key()? {
                    Some(__Key::Type) => {
                        let tag: __Tag<'de> = entries.map.next_value()?;
                        let known = match &tag {
                            __Tag::Borrowed(tag) => Some(*tag),
                            __Tag::Other(#serde_json::Value::String(tag)) => Some(tag.as_str()),
                            __Tag::Other(_) => None,
                        };
                        match known {
                            #(#arms)*
                            _ => {}
                        }
                        entries.key = Some(__Key::Type);
                        entries.tag = Some(tag);
                    }
                    Some(key) => entries.key = Some(key),
                    None => entries.done = true,
                }
            }
        })
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
