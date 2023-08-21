use alloy_json_rpc::RpcResult;
use alloy_networks::{Network, Transaction};
use alloy_primitives::Address;
use alloy_transports::{BoxTransport, RpcClient, Transport, TransportError};

use std::{borrow::Cow, future::Future, pin::Pin};

pub type MwareFut<'a, T, E> = Pin<Box<dyn Future<Output = RpcResult<T, E>> + Send + 'a>>;

/// Middleware is parameterized with a network and a transport. The default
/// transport is type-erased, but you can do `Middleware<N, Http>`.
pub trait Middleware<N: Network, T: Transport = BoxTransport>: Send + Sync {
    fn client(&self) -> &RpcClient<T>;

    /// Return a reference to the inner Middleware.
    ///
    /// Middleware are object safe now :)
    fn inner(&self) -> &dyn Middleware<N, T>;

    fn estimate_gas<'s: 'fut, 'a: 'fut, 'fut>(
        &'s self,
        tx: &'a N::TransactionRequest,
    ) -> MwareFut<'fut, alloy_primitives::U256, TransportError>
    where
        Self: Sync + 'fut,
    {
        self.inner().estimate_gas(tx)
    }

    /// Get the transaction count for an address. Used for finding the
    /// appropriate nonce.
    ///
    /// TODO: block number/hash/tag
    fn get_transaction_count<'s: 'fut, 'a: 'fut, 'fut>(
        &'s self,
        address: Address,
    ) -> MwareFut<'fut, alloy_primitives::U256, TransportError>
    where
        Self: Sync + 'fut,
    {
        self.inner().get_transaction_count(address)
    }

    /// Send a transaction to the network.
    ///
    /// The transaction type is defined by the network.
    fn send_transaction<'s: 'fut, 'a: 'fut, 'fut>(
        &'s self,
        tx: &'a N::TransactionRequest,
    ) -> MwareFut<'fut, N::Receipt, TransportError> {
        self.inner().send_transaction(tx)
    }

    fn populate_gas<'s: 'fut, 'a: 'fut, 'fut>(
        &'s self,
        tx: &'a mut N::TransactionRequest,
    ) -> MwareFut<'fut, (), TransportError>
    where
        Self: Sync,
    {
        Box::pin(async move {
            let gas = self.estimate_gas(&*tx).await;

            gas.map(|gas| tx.set_gas(gas))
        })
    }
}

impl<N: Network, T: Transport + Clone> Middleware<N, T> for RpcClient<T> {
    fn client(&self) -> &RpcClient<T> {
        self
    }

    fn inner(&self) -> &dyn Middleware<N, T> {
        panic!("called inner on <RpcClient as Middleware>")
    }

    fn estimate_gas<'s: 'fut, 'a: 'fut, 'fut>(
        &'s self,
        tx: &'a <N as Network>::TransactionRequest,
    ) -> MwareFut<'fut, alloy_primitives::U256, TransportError> {
        self.prepare("eth_estimateGas", Cow::Borrowed(tx)).boxed()
    }

    fn get_transaction_count<'s: 'fut, 'a: 'fut, 'fut>(
        &'s self,
        address: Address,
    ) -> MwareFut<'fut, alloy_primitives::U256, TransportError>
    where
        Self: Sync + 'fut,
    {
        self.prepare(
            "eth_getTransactionCount",
            Cow::<(Address, &'static str)>::Owned((address, "latest")),
        )
        .boxed()
    }

    fn send_transaction<'s: 'fut, 'a: 'fut, 'fut>(
        &'s self,
        tx: &'a N::TransactionRequest,
    ) -> MwareFut<'fut, N::Receipt, TransportError> {
        self.prepare("eth_sendTransaction", Cow::Borrowed(tx))
            .boxed()
    }
}

/// Middleware use a tower-like Layer abstraction
pub trait MwareLayer<N: Network> {
    type Middleware<T: Transport>: Middleware<N, T>;

    fn layer<M, T>(&self, inner: M) -> Self::Middleware<T>
    where
        M: Middleware<N, T>,
        T: Transport;
}

#[cfg(test)]
mod test {
    use crate::Middleware;
    use alloy_networks::Network;

    fn __compile_check<N: Network>() -> Box<dyn Middleware<N>> {
        unimplemented!()
    }
}
