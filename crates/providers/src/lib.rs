#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![warn(
    // TODO:
    // missing_copy_implementations,
    // missing_debug_implementations,
    // missing_docs,
    unreachable_pub,
    // clippy::missing_const_for_fn,
    rustdoc::all
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use alloy_networks::{Network, Transaction};
use alloy_primitives::Address;
use alloy_rpc_client::RpcClient;
use alloy_transport::{BoxTransport, Transport, TransportResult};
use std::{borrow::Cow, marker::PhantomData};

mod builder;
pub use builder::{ProviderBuilder, ProviderLayer, Stack};

pub mod provider;
pub mod utils;

/// A network-wrapped RPC client.
///
/// This type allows you to specify (at the type-level) that the RPC client is
/// for a specific network. This helps avoid accidentally using the wrong
/// connection to access a network.
#[derive(Debug)]
pub struct NetworkRpcClient<N: Network, T: Transport = BoxTransport> {
    pub network: PhantomData<fn() -> N>,
    pub client: RpcClient<T>,
}

impl<N, T> std::ops::Deref for NetworkRpcClient<N, T>
where
    N: Network,
    T: Transport,
{
    type Target = RpcClient<T>;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl<N, T> From<RpcClient<T>> for NetworkRpcClient<N, T>
where
    N: Network,
    T: Transport,
{
    fn from(client: RpcClient<T>) -> Self {
        Self { network: PhantomData, client }
    }
}

impl<N, T> From<NetworkRpcClient<N, T>> for RpcClient<T>
where
    N: Network,
    T: Transport,
{
    fn from(client: NetworkRpcClient<N, T>) -> Self {
        client.client
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
/// Provider is parameterized with a network and a transport. The default
/// transport is type-erased, but you can do `Provider<N, Http>`.
pub trait Provider<N: Network, T: Transport = BoxTransport>: Send + Sync {
    fn raw_client(&self) -> &RpcClient<T> {
        &self.client().client
    }

    /// Return a reference to the inner RpcClient.
    fn client(&self) -> &NetworkRpcClient<N, T>;

    /// Return a reference to the inner Provider.
    ///
    /// Providers are object safe now :)
    fn inner(&self) -> &dyn Provider<N, T>;

    async fn estimate_gas(
        &self,
        tx: &N::TransactionRequest,
    ) -> TransportResult<alloy_primitives::U256> {
        self.inner().estimate_gas(tx).await
    }

    /// Get the transaction count for an address. Used for finding the
    /// appropriate nonce.
    ///
    /// TODO: block number/hash/tag
    async fn get_transaction_count(
        &self,
        address: Address,
    ) -> TransportResult<alloy_primitives::U256> {
        self.inner().get_transaction_count(address).await
    }

    /// Send a transaction to the network.
    ///
    /// The transaction type is defined by the network.
    async fn send_transaction(&self, tx: &N::TransactionRequest) -> TransportResult<N::Receipt> {
        self.inner().send_transaction(tx).await
    }

    async fn populate_gas(&self, tx: &mut N::TransactionRequest) -> TransportResult<()> {
        let gas = self.estimate_gas(&*tx).await;

        gas.map(|gas| tx.set_gas(gas))
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<N: Network, T: Transport + Clone> Provider<N, T> for NetworkRpcClient<N, T> {
    fn client(&self) -> &NetworkRpcClient<N, T> {
        self
    }

    fn inner(&self) -> &dyn Provider<N, T> {
        panic!("called inner on <RpcClient as Provider>")
    }

    async fn estimate_gas(
        &self,
        tx: &<N as Network>::TransactionRequest,
    ) -> TransportResult<alloy_primitives::U256> {
        self.prepare("eth_estimateGas", Cow::Borrowed(tx)).await
    }

    async fn get_transaction_count(
        &self,
        address: Address,
    ) -> TransportResult<alloy_primitives::U256> {
        self.prepare(
            "eth_getTransactionCount",
            Cow::<(Address, String)>::Owned((address, "latest".to_string())),
        )
        .await
    }

    async fn send_transaction(&self, tx: &N::TransactionRequest) -> TransportResult<N::Receipt> {
        self.prepare("eth_sendTransaction", Cow::Borrowed(tx)).await
    }
}

#[cfg(test)]
mod test {
    use crate::Provider;
    use alloy_networks::Network;

    // checks that `Provider<N>` is object-safe
    fn __compile_check<N: Network>() -> Box<dyn Provider<N>> {
        unimplemented!()
    }
}
