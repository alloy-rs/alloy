use alloc::sync::Arc;
use core::hash::{Hash, Hasher};

// Re-export for convenience
pub use c_kzg::KzgSettings;

/// Precompute value that optimizes computing cell kzg proofs.
///
/// Set to 0 as we do not use `compute_cells_and_kzg_proofs` or `recover_cells_and_kzg_proofs`.
///
/// Learn more: <https://github.com/ethereum/c-kzg-4844/blob/dffa18ee350aeef38f749ffad24a27c1645fb4f8/README.md?plain=1#L112>
const PRECOMPUTE: u64 = 0;

/// KZG settings.
#[derive(Clone, Debug, Default, Eq)]
pub enum EnvKzgSettings {
    /// Default mainnet trusted setup.
    #[default]
    Default,
    /// Custom trusted setup.
    Custom(Arc<KzgSettings>),
}

// Implement PartialEq and Hash manually because `c_kzg::KzgSettings` does not implement them.
impl PartialEq for EnvKzgSettings {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Default, Self::Default) => true,
            (Self::Custom(a), Self::Custom(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl Hash for EnvKzgSettings {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Self::Default => {}
            Self::Custom(settings) => Arc::as_ptr(settings).hash(state),
        }
    }
}

impl EnvKzgSettings {
    /// Returns the KZG settings.
    ///
    /// This will initialize the default settings if it is not already loaded.
    #[inline]
    pub fn get(&self) -> &KzgSettings {
        match self {
            Self::Default => c_kzg::ethereum_kzg_settings(PRECOMPUTE),
            Self::Custom(settings) => settings,
        }
    }

    /// Load custom KZG settings from a trusted setup file.
    #[cfg(feature = "std")]
    pub fn load_from_trusted_setup_file(
        trusted_setup_file: &std::path::Path,
    ) -> Result<Self, c_kzg::Error> {
        let settings = KzgSettings::load_trusted_setup_file(trusted_setup_file, PRECOMPUTE)?;
        Ok(Self::Custom(Arc::new(settings)))
    }
}
