use crate::{Provider, RootProvider};

use alloy_network::Network;
use std::sync::Arc;

/// A wrapper struct around Arc dyn provider
pub struct WrappedProvider<'a, N>(Arc<dyn Provider<N> + 'a>);

impl<'a, N: Network> WrappedProvider<'a, N> {
    pub fn new<P>(provider: &'a P) -> Self
    where
        P: Provider<N> + 'a,
    {
        Self(Arc::new(provider))
    }
}

impl<N: Network> Provider<N> for WrappedProvider<'_, N> {
    fn root(&self) -> &RootProvider<N> {
        self.0.root()
    }
}

impl<N> std::fmt::Debug for WrappedProvider<'_, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("WrappedProvider")
            .field(&"<dyn Provider>") // Since we can't debug the trait object directly
            .finish()
    }
}
