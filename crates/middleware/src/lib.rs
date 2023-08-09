use std::{future::Future, pin::Pin};

use alloy_json_rpc::RpcResult;
use alloy_networks::Network;
use alloy_transports::{BoxTransport, RpcClient, Transport, TransportError};

pub type MwareFut<'a, T, E> = Pin<Box<dyn Future<Output = RpcResult<T, E>> + Send + 'a>>;

pub trait Middleware<N: Network, T: Transport = BoxTransport> {
    fn client(&self) -> &RpcClient<T>;

    fn inner(&self) -> &dyn Middleware<N, T>;

    fn send_transaction<'a>(
        &self,
        tx: &'a N::TransactionRequest,
    ) -> MwareFut<'a, N::Receipt, TransportError> {
        self.inner().send_transaction(tx)
    }
}

impl<N: Network, T: Transport> Middleware<N, T> for RpcClient<T> {
    fn client(&self) -> &RpcClient<T> {
        self
    }

    fn inner(&self) -> &dyn Middleware<N, T> {
        panic!("called inner on <RpcClient as Middleware>")
    }

    fn send_transaction<'a>(
        &self,
        tx: &'a N::TransactionRequest,
    ) -> MwareFut<'a, N::Receipt, TransportError> {
        self.prepare("eth_sendTransaction", tx).box_pin()
    }
}

pub trait MwareLayer<N: Network> {
    type Middleware<T: Transport>: Middleware<N, T>;

    fn layer<M, T>(&self, inner: M) -> Self::Middleware<T>
    where
        M: Middleware<N, T>,
        T: Transport;
}
