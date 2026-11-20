use super::forward;
use crate::Provider;
use alloy_network::{Ethereum, Network};
use std::sync::Arc;

/// A wrapper struct around a type erased [`Provider`].
///
/// This type will delegate all functions to the wrapped provider, with the exception of non
/// object-safe functions (e.g. functions requiring `Self: Sized`) which use the default trait
/// implementation.
///
/// This is a convenience type for `Arc<dyn Provider<N> + 'static>`.
#[derive(Clone)]
#[doc(alias = "BoxProvider")]
pub struct DynProvider<N = Ethereum>(Arc<dyn Provider<N> + 'static>);

impl<N: Network> DynProvider<N> {
    /// Creates a new [`DynProvider`] by erasing the type.
    ///
    /// This is the same as [`provider.erased()`](Provider::erased).
    pub fn new<P: Provider<N> + 'static>(provider: P) -> Self {
        Self(Arc::new(provider))
    }
}

impl<N: Network> forward::ForwardingProvider<N> for DynProvider<N> {
    type Inner = dyn Provider<N>;

    #[inline]
    fn inner_provider(&self) -> &Self::Inner {
        &*self.0
    }
}

forward::impl_provider_forwarding! {
    impl[N: Network] Provider<N> for DynProvider<N>;
    // The generic `Self: Sized` methods can't be called on `dyn Provider`, so they keep their
    // defaults.
    skip_generic_methods;
    extra {
        fn erased(self) -> Self
        where
            Self: Sized + 'static,
        {
            self
        }
    }
}

impl<N> std::fmt::Debug for DynProvider<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("DynProvider").field(&"<dyn Provider>").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProviderBuilder;
    fn assert_provider<P: Provider + Sized + Clone + Unpin + 'static>(_: P) {}

    #[test]
    fn test_erased_provider() {
        let provider =
            ProviderBuilder::new().connect_http("http://localhost:8080".parse().unwrap()).erased();
        assert_provider(provider);
    }
}
