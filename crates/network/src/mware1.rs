use crate::{Network, Transaction};

use alloy_primitives::{TxHash, U256};
use alloy_transports::{CallWithPost, Connection, TransportError};

pub(crate) type MwareCall<'a, 'process, 'transform, M, N, Resp, T = Resp> = CallWithPost<
    'process,
    'transform,
    &'a <M as Middleware<N>>::Connection,
    <M as Middleware<N>>::Connection,
    Resp,
    T,
>;

// TODO: replace these with things that aren't Box<dyn Future>
pub trait Middleware<N>: Send + Sync + std::fmt::Debug
where
    N: Network,
{
    type Connection: Connection;
    type Inner: Middleware<N, Connection = Self::Connection>;
    type Error: std::error::Error + From<TransportError> + Send + Sync + 'static; // TODO

    fn inner(&self) -> &Self::Inner;

    fn connection(&self) -> &Self::Connection {
        self.inner().connection()
    }

    fn get_transaction(&self, tx_hash: TxHash) -> MwareCall<Self, N, N::TransactionRespose> {
        self.inner().get_transaction(tx_hash)
    }

    fn estimate_gas<'a, 'b>(&'a self, tx: &'b N::Transaction) -> MwareCall<Self, N, U256>
    where
        'a: 'b,
    {
        self.inner().estimate_gas(tx)
    }

    fn populate_gas<'a, 'b>(
        &'a self,
        tx: &'b mut N::Transaction,
    ) -> MwareCall<'a, 'b, 'b, Self, N, U256, ()>
    where
        'a: 'b,
    {
        self.estimate_gas(tx)
            .and(|gas| {
                tx.set_gas(gas);
                gas
            })
            .and_transform(|_| ())
    }
}

impl<N, T> Middleware<N> for T
where
    T: Connection,
    N: Network,
{
    type Connection = Self;
    type Inner = Self;
    type Error = TransportError;

    fn inner(&self) -> &Self::Inner {
        self
    }

    fn connection(&self) -> &Self::Connection {
        self
    }

    fn get_transaction(
        &self,
        tx_hash: TxHash,
    ) -> MwareCall<Self, N, <N as Network>::TransactionRespose> {
        self.request("eth_getTransactionByHash", tx_hash).into()
    }

    fn estimate_gas<'a, 'b>(&'a self, tx: &'b N::Transaction) -> MwareCall<Self, N, U256>
    where
        'a: 'b,
    {
        self.request("eth_estimateGas", tx).into()
    }
}
