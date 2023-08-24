use std::marker::PhantomData;

use alloy_networks::Network;
use alloy_transports::Transport;

use crate::Provider;

/// A layering abstraction in the vein of [`tower::Layer`]
///
/// [`tower::Layer`]: https://docs.rs/tower/latest/tower/trait.Layer.html
pub trait ProviderLayer<P: Provider<N, T>, N: Network, T: Transport> {
    type Provider: Provider<N, T>;

    fn layer(&self, inner: P) -> Self::Provider;
}

pub struct Stack<T, Inner, Outer> {
    inner: Inner,
    outer: Outer,
    _pd: std::marker::PhantomData<fn() -> T>,
}

impl<T, Inner, Outer> Stack<T, Inner, Outer> {
    /// Create a new `Stack`.
    pub fn new(inner: Inner, outer: Outer) -> Self {
        Stack {
            inner,
            outer,
            _pd: std::marker::PhantomData,
        }
    }
}

impl<P, N, T, Inner, Outer> ProviderLayer<P, N, T> for Stack<T, Inner, Outer>
where
    T: Transport,
    N: Network,
    P: Provider<N, T>,
    Inner: ProviderLayer<P, N, T>,
    Outer: ProviderLayer<Inner::Provider, N, T>,
{
    type Provider = Outer::Provider;

    fn layer(&self, provider: P) -> Self::Provider {
        let inner = self.inner.layer(provider);

        self.outer.layer(inner)
    }
}

/// A builder for constructing a [`Provider`] from various layers.
pub struct ProviderBuilder<L, N = (), T = ()> {
    layer: L,

    transport: PhantomData<T>,
    network: PhantomData<N>,
}

impl<L, N, T> ProviderBuilder<L, N, T> {
    pub fn layer<Inner>(self, layer: Inner) -> ProviderBuilder<Stack<T, Inner, L>> {
        ProviderBuilder {
            layer: Stack::new(layer, self.layer),
            transport: PhantomData,
            network: PhantomData,
        }
    }

    /// Change the network.
    pub fn network<Net: Network>(self) -> ProviderBuilder<L, Net, T> {
        ProviderBuilder {
            layer: self.layer,
            transport: self.transport,
            network: PhantomData,
        }
    }

    pub fn provider<P>(self, provider: P) -> L::Provider
    where
        L: ProviderLayer<P, N, T>,
        P: Provider<N, T>,
        T: Transport,
        N: Network,
    {
        self.layer.layer(provider)
    }
}

// Copyright (c) 2019 Tower Contributors

// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:

// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
