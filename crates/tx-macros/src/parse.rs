use darling::{FromDeriveInput, FromMeta, FromVariant};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Path, Type};

/// Container-level arguments for the TransactionEnvelope derive macro.
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(envelope))]
pub(crate) struct EnvelopeArgs {
    /// The identifier of the input enum.
    pub ident: Ident,

    /// The generics of the input enum.
    pub generics: syn::Generics,

    /// Custom name for the generated transaction type enum.
    /// Defaults to `{EnumName}Type`.
    #[darling(default)]
    pub tx_type_name: Option<Ident>,

    /// Custom path to the alloy_consensus crate.
    /// Defaults to `::alloy_consensus`.
    #[darling(default)]
    pub alloy_consensus: Option<Path>,

    /// Custom `cfg_attr` value for serde implementations.
    #[darling(default)]
    pub serde_cfg: Option<syn::Meta>,

    /// Custom `cfg_attr` value for arbitrary implementations.
    #[darling(default)]
    pub arbitrary_cfg: Option<syn::Meta>,

    /// Optional typed transaction enum name to generate.
    /// When specified, generates a corresponding TypedTransaction enum.
    #[darling(default)]
    pub typed: Option<Ident>,

    /// The enum data (variants).
    pub data: darling::ast::Data<EnvelopeVariant, ()>,
}

/// Variant of transaction envelope enum.
#[derive(Debug, FromVariant)]
#[darling(attributes(envelope), forward_attrs(serde, doc))]
pub(crate) struct EnvelopeVariant {
    /// The identifier of the variant.
    pub ident: Ident,

    /// The fields of the variant.
    pub fields: darling::ast::Fields<syn::Type>,

    /// Kind of the variant.
    #[darling(flatten)]
    pub kind: VariantKind,

    /// Optional custom typed transaction type for this variant.
    #[darling(default)]
    pub typed: Option<Ident>,

    /// Forwarded attributes.
    pub attrs: Vec<syn::Attribute>,
}

/// Kind of the envelope variant.
#[derive(Debug, Clone, FromMeta)]
pub(crate) enum VariantKind {
    /// A standalone transaction with a type tag.
    #[darling(rename = "ty")]
    Typed(u8),
    /// Flattened envelope.
    #[darling(word, rename = "flatten")]
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
#[derive(Debug, Clone)]
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
    pub(crate) fn from_args(args: EnvelopeArgs) -> darling::Result<Self> {
        // Validate it's an enum
        let variants = match args.data {
            darling::ast::Data::Enum(variants) => variants,
            _ => {
                return Err(darling::Error::custom(
                    "`TransactionEnvelope` can only be derived for enums",
                )
                .with_span(&args.ident));
            }
        };

        let mut processed = Vec::new();
        for variant in variants {
            let EnvelopeVariant { ident, fields, kind, attrs, typed } = variant;

            let mut serde_attrs = None;
            let mut doc_attrs = Vec::new();

            for attr in attrs {
                if attr.path().is_ident("serde") {
                    if let syn::Meta::List(list) = attr.meta {
                        serde_attrs = Some(list.tokens);
                    }
                } else if attr.path().is_ident("doc") {
                    doc_attrs.push(attr);
                }
            }

            // Check that variant has exactly one unnamed field
            let ty = match &fields.style {
                darling::ast::Style::Tuple if fields.len() == 1 => fields
                    .fields
                    .into_iter()
                    .next()
                    .expect("len checked"),
                darling::ast::Style::Tuple => {
                    return Err(darling::Error::custom(format!(
                        "expected exactly one field, found {}",
                        fields.len()
                    ))
                    .with_span(&ident))
                }
                _ => {
                    return Err(darling::Error::custom(
                        "TransactionEnvelope variants must have a single unnamed field",
                    )
                    .with_span(&ident))
                }
            };

            processed.push(ProcessedVariant {
                name: ident,
                ty,
                kind,
                serde_attrs,
                doc_attrs,
                typed,
            });
        }

        let typed =
            processed.iter().filter(|v| matches!(v.kind, VariantKind::Typed(_))).cloned().collect();

        let flattened = processed
            .iter()
            .filter(|v| matches!(v.kind, VariantKind::Flattened))
            .cloned()
            .collect();

        Ok(Self { all: processed, typed, flattened })
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
