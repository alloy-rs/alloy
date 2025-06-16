use darling::{FromDeriveInput, FromMeta, FromVariant};
use syn::{Ident, Path, Type};

/// Container-level arguments for the TransactionEnvelope derive macro.
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(envelope), forward_attrs(allow, doc, cfg))]
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

    /// The enum data (variants).
    pub data: darling::ast::Data<EnvelopeVariant, ()>,
}

/// Variant of transaction envelope enum.
#[derive(Debug, FromVariant)]
#[darling(attributes(envelope))]
pub(crate) struct EnvelopeVariant {
    /// The identifier of the variant.
    pub ident: Ident,

    /// The fields of the variant.
    pub fields: darling::ast::Fields<syn::Type>,

    /// Kind of the variant.
    #[darling(flatten)]
    pub kind: VariantKind,
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

/// Processed variant information.
#[derive(Debug, Clone)]
pub(crate) struct ProcessedVariant {
    /// The variant name.
    pub name: Ident,
    /// The inner type of the variant.
    pub ty: Type,
    /// The kind of variant.
    pub kind: VariantKind,
}

impl ProcessedVariant {
    /// Returns true if this is a legacy transaction variant (type 0).
    pub(crate) fn is_legacy(&self) -> bool {
        matches!(self.kind, VariantKind::Typed(0))
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
            let EnvelopeVariant { ident, fields, kind } = variant;

            // Check that variant has exactly one unnamed field
            let ty = match &fields.style {
                darling::ast::Style::Tuple if fields.len() == 1 => fields.fields[0].clone(),
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

            processed.push(ProcessedVariant { name: ident, ty, kind });
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
