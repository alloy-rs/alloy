use alloc::string::String;

/// Error variants when converting from [crate::Transaction] to [alloy_consensus::Signed]
/// transaction.
#[derive(Debug, derive_more::Display)]
pub enum ConversionError {
    /// A custom Conversion Error that doesn't fit other categories.
    #[display("conversion error: {_0}")]
    Custom(String),
}

#[cfg(feature = "std")]
impl std::error::Error for ConversionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}
