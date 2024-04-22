use alloy_primitives::B256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// A consensus hashable item, with its memoized hash.
///
/// We do not implement
pub struct Sealed<T> {
    /// The inner item
    inner: T,
    /// Its hash.
    seal: B256,
}

impl<T> core::ops::Deref for Sealed<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}

impl<T> Sealed<T> {
    /// Instantiate without performing the hash. This should be used carefully.
    pub const fn new_unchecked(inner: T, seal: B256) -> Self {
        Self { inner, seal }
    }

    /// Decompose into parts.
    #[allow(clippy::missing_const_for_fn)] // false positive
    pub fn into_parts(self) -> (T, B256) {
        (self.inner, self.seal)
    }

    /// Get the inner item.
    #[inline(always)]
    pub const fn inner(&self) -> &T {
        &self.inner
    }

    /// Get the hash.
    #[inline(always)]
    pub const fn seal(&self) -> B256 {
        self.seal
    }

    /// Geth the hash (alias for [`Self::seal`]).
    #[inline(always)]
    pub const fn hash(&self) -> B256 {
        self.seal()
    }
}

/// Sealeable objects.
pub trait Sealable: Sized {
    /// Calculate the seal hash, this may be slow.
    fn hash(&self) -> B256;

    /// Seal the object by calculating the hash. This may be slow.
    fn seal_slow(self) -> Sealed<Self> {
        let seal = self.hash();
        Sealed::new_unchecked(self, seal)
    }

    /// Instantiate an unchecked seal. This should be used with caution.
    fn seal_unchecked(self, seal: B256) -> Sealed<Self> {
        Sealed::new_unchecked(self, seal)
    }
}
