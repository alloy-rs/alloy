use proc_macro2::{Group, TokenStream};
use quote::quote;
use syn::{
    meta::ParseNestedMeta, parse::Parse, Attribute, Data, DeriveInput, Error, Fields, Generics,
    Ident, LitInt, LitStr, Meta, Path, Result, Type,
};

/// Parsed input of the TransactionEnvelope derive macro.
pub(crate) struct EnvelopeArgs {
    /// The identifier of the input enum.
    pub ident: Ident,

    /// The generics of the input enum.
    pub generics: Generics,

    /// Custom name for the generated transaction type enum.
    /// Defaults to `{EnumName}Type`.
    pub tx_type_name: Option<Ident>,

    /// Custom path to the alloy_consensus crate.
    /// Defaults to `::alloy_consensus`.
    pub alloy_consensus: Option<Path>,

    /// Custom `cfg_attr` value for serde implementations.
    pub serde_cfg: Option<TokenStream>,

    /// Custom `cfg_attr` value for arbitrary implementations.
    pub arbitrary_cfg: Option<TokenStream>,

    /// Optional typed transaction enum name to generate.
    /// When specified, generates a corresponding TypedTransaction enum.
    pub typed: Option<Ident>,

    /// The enum variants.
    pub variants: Vec<ProcessedVariant>,
}

impl EnvelopeArgs {
    /// Parses the derive input and its `#[envelope(...)]` attributes.
    pub(crate) fn from_derive_input(input: DeriveInput) -> Result<Self> {
        let Data::Enum(data) = input.data else {
            return Err(Error::new_spanned(
                &input.ident,
                "`TransactionEnvelope` can only be derived for enums",
            ));
        };

        let mut args = Self {
            ident: input.ident,
            generics: input.generics,
            tx_type_name: None,
            alloy_consensus: None,
            serde_cfg: None,
            arbitrary_cfg: None,
            typed: None,
            variants: Vec::new(),
        };

        for attr in envelope_attrs(&input.attrs) {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("tx_type_name") {
                    set(&mut args.tx_type_name, &meta, parse_value(&meta)?)
                } else if meta.path.is_ident("alloy_consensus") {
                    set(&mut args.alloy_consensus, &meta, parse_value(&meta)?)
                } else if meta.path.is_ident("typed") {
                    set(&mut args.typed, &meta, parse_value(&meta)?)
                } else if meta.path.is_ident("serde_cfg") {
                    set(&mut args.serde_cfg, &meta, parse_list(&meta)?)
                } else if meta.path.is_ident("arbitrary_cfg") {
                    set(&mut args.arbitrary_cfg, &meta, parse_list(&meta)?)
                } else {
                    Err(meta.error("unknown `envelope` attribute"))
                }
            })?;
        }

        for variant in data.variants {
            let mut kind = None;
            let mut typed = None;
            for attr in envelope_attrs(&variant.attrs) {
                attr.parse_nested_meta(|meta| {
                    let new_kind = if meta.path.is_ident("ty") {
                        VariantKind::Typed(parse_ty(&meta)?)
                    } else if meta.path.is_ident("flatten") {
                        VariantKind::Flattened
                    } else if meta.path.is_ident("typed") {
                        return set(&mut typed, &meta, parse_value(&meta)?);
                    } else {
                        return Err(meta.error("unknown `envelope` attribute"));
                    };
                    if kind.replace(new_kind).is_some() {
                        return Err(meta.error("expected only one of `ty` or `flatten`"));
                    }
                    Ok(())
                })?;
            }
            let Some(kind) = kind else {
                return Err(Error::new_spanned(
                    &variant.ident,
                    "expected `#[envelope(ty = N)]` or `#[envelope(flatten)]`",
                ));
            };

            let mut serde_attrs = None;
            let mut doc_attrs = Vec::new();
            for attr in variant.attrs {
                if attr.path().is_ident("serde") {
                    if let Meta::List(list) = attr.meta {
                        serde_attrs = Some(list.tokens);
                    }
                } else if attr.path().is_ident("doc") {
                    doc_attrs.push(attr);
                }
            }

            // Check that variant has exactly one unnamed field
            let ty = match variant.fields {
                Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                    fields.unnamed.into_iter().next().unwrap().ty
                }
                Fields::Unnamed(fields) => {
                    return Err(Error::new_spanned(
                        &variant.ident,
                        format!("expected exactly one field, found {}", fields.unnamed.len()),
                    ))
                }
                _ => {
                    return Err(Error::new_spanned(
                        &variant.ident,
                        "TransactionEnvelope variants must have a single unnamed field",
                    ))
                }
            };

            args.variants.push(ProcessedVariant {
                name: variant.ident,
                ty,
                kind,
                serde_attrs,
                doc_attrs,
                typed,
            });
        }

        Ok(args)
    }
}

/// Returns the `#[envelope(...)]` attributes, skipping empty `#[envelope]` ones.
fn envelope_attrs(attrs: &[Attribute]) -> impl Iterator<Item = &Attribute> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("envelope") && !matches!(attr.meta, Meta::Path(_)))
}

/// Stores an attribute value, rejecting a key that was already set.
fn set<T>(slot: &mut Option<T>, meta: &ParseNestedMeta<'_>, value: T) -> Result<()> {
    if slot.replace(value).is_some() {
        return Err(meta.error("duplicate `envelope` attribute"));
    }
    Ok(())
}

/// Parses `key = value`, also accepting the value as a string literal (`key = "value"`).
fn parse_value<T: Parse>(meta: &ParseNestedMeta<'_>) -> Result<T> {
    let input = meta.value()?;
    if input.peek(LitStr) {
        input.parse::<LitStr>()?.parse()
    } else {
        input.parse()
    }
}

/// Parses `ty = N`, also accepting `ty = "N"`, which is parsed with `str::parse` like darling did.
fn parse_ty(meta: &ParseNestedMeta<'_>) -> Result<u8> {
    let input = meta.value()?;
    if input.peek(LitStr) {
        let lit = input.parse::<LitStr>()?;
        lit.value().parse().map_err(|e| Error::new(lit.span(), e))
    } else {
        input.parse::<LitInt>()?.base10_parse()
    }
}

/// Parses the contents of `key(...)`, `key[...]` or `key{...}`.
fn parse_list(meta: &ParseNestedMeta<'_>) -> Result<TokenStream> {
    Ok(meta.input.parse::<Group>()?.stream())
}

/// Kind of the envelope variant.
#[derive(Clone, Debug)]
pub(crate) enum VariantKind {
    /// A standalone transaction with a type tag.
    Typed(u8),
    /// Flattened envelope.
    Flattened,
}

impl VariantKind {
    /// Returns serde transaction enum tag and aliases.
    pub(crate) fn serde_tag_and_aliases(&self) -> (String, Vec<String>) {
        let Self::Typed(ty) = self else { return Default::default() };

        let tx_type_hex = format!("{ty:x}");

        let mut aliases = vec![];
        // Add alias for single digit hex values (e.g., "0x0" for "0x00")
        if tx_type_hex.len() == 1 {
            aliases.push(format!("0x0{}", tx_type_hex));
        }

        // Add alias for uppercase values (e.g., "0x7E" for "0x7e")
        if tx_type_hex != tx_type_hex.to_uppercase() {
            aliases.push(format!("0x{}", tx_type_hex.to_uppercase()));
        }

        (format!("0x{tx_type_hex}"), aliases)
    }
}

/// Processed variant information.
#[derive(Clone)]
pub(crate) struct ProcessedVariant {
    /// The variant name.
    pub name: Ident,
    /// The inner type of the variant.
    pub ty: Type,
    /// The kind of variant.
    pub kind: VariantKind,
    /// The serde attributes for the variant.
    pub serde_attrs: Option<TokenStream>,
    /// The doc attributes for the variant.
    pub doc_attrs: Vec<syn::Attribute>,
    /// Optional custom typed transaction type for this variant.
    pub typed: Option<Ident>,
}

impl ProcessedVariant {
    /// Returns true if this is a legacy transaction variant (type 0).
    pub(crate) const fn is_legacy(&self) -> bool {
        matches!(self.kind, VariantKind::Typed(0))
    }

    /// Returns the inner type to use as unsigned type for the typed transaction enum.
    pub(crate) fn inner_type(&self) -> TokenStream {
        // If a custom type is provided, use it
        if let Some(custom) = &self.typed {
            return quote! { #custom };
        }

        let ty = &self.ty;

        // For most cases, we need to extract T from Signed<T>
        if let syn::Type::Path(type_path) = ty {
            if let Some(segment) = type_path.path.segments.last() {
                if segment.ident == "Signed" || segment.ident == "Sealed" {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                            return quote! { #inner_ty };
                        }
                    }
                }
            }
        }
        // Fallback to original type
        quote! { #ty }
    }
}

/// Groups variants by their kind for easier code generation.
pub(crate) struct GroupedVariants {
    /// All variants.
    pub all: Vec<ProcessedVariant>,
    /// Only typed variants (with transaction type IDs).
    pub typed: Vec<ProcessedVariant>,
    /// Only flattened variants.
    pub flattened: Vec<ProcessedVariant>,
}

impl GroupedVariants {
    /// Create grouped variants from a list of processed variants.
    pub(crate) fn new(processed: Vec<ProcessedVariant>) -> Self {
        let typed =
            processed.iter().filter(|v| matches!(v.kind, VariantKind::Typed(_))).cloned().collect();

        let flattened = processed
            .iter()
            .filter(|v| matches!(v.kind, VariantKind::Flattened))
            .cloned()
            .collect();

        Self { all: processed, typed, flattened }
    }

    /// Find the legacy variant if it exists.
    pub(crate) fn legacy_variant(&self) -> Option<&ProcessedVariant> {
        self.all.iter().find(|v| v.is_legacy())
    }

    /// Get all variant names.
    pub(crate) fn variant_names(&self) -> Vec<&syn::Ident> {
        self.all.iter().map(|v| &v.name).collect()
    }

    /// Get all variant types.
    pub(crate) fn variant_types(&self) -> Vec<&syn::Type> {
        self.all.iter().map(|v| &v.ty).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_tag() {
        assert_eq!(
            VariantKind::Typed(126).serde_tag_and_aliases(),
            ("0x7e".to_string(), vec!["0x7E".to_string()])
        );
        assert_eq!(
            VariantKind::Typed(1).serde_tag_and_aliases(),
            ("0x1".to_string(), vec!["0x01".to_string()])
        );
        assert_eq!(
            VariantKind::Typed(10).serde_tag_and_aliases(),
            ("0xa".to_string(), vec!["0x0a".to_string(), "0xA".to_string()])
        );
    }
}
