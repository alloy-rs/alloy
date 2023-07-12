use std::{future::Future, pin::Pin};

use alloy_primitives::{TxHash, U256};
use alloy_rlp::{Decodable, Encodable};
use alloy_transports::{Connection, RpcCall, RpcObject, RpcResult, TransportError};

pub trait Transaction: Encodable + Decodable {
    // VALUE
    fn get_value(&self) -> U256;
    fn set_value(&mut self, value: U256);
    fn value(self, value: U256) -> Self;

    // GAS PRICE
    fn get_gas_price(&self) -> U256;
    // set and builder are omitted due to eip1559 interaction.

    // GAS AMOUNT
    fn get_gas(&self) -> U256;
    fn set_gas(&mut self, gas: U256);
    fn gas(self, gas: U256) -> Self;

    // DATA
    fn get_data(&self) -> &[u8];
    fn set_data(&mut self, data: Vec<u8>);
    fn data(self, data: Vec<u8>) -> Self;

    // TO
    fn get_to(&self) -> Option<&[u8]>;
    fn set_to(&mut self, to: Option<Vec<u8>>);
    fn to(self, to: Option<Vec<u8>>) -> Self;
}

pub trait Eip1559Transaction: Transaction {
    // MAX FEE PER GAS
    fn get_max_fee_per_gas(&self) -> U256;
    fn set_max_fee_per_gas(&mut self, max_fee_per_gas: U256);
    fn max_fee_per_gas(self, max_fee_per_gas: U256) -> Self;

    // MAX PRIORITY FEE PER GAS
    fn get_max_priority_fee_per_gas(&self) -> U256;
    fn set_max_priority_fee_per_gas(&mut self, max_priority_fee_per_gas: U256);
    fn max_priority_fee_per_gas(self, max_priority_fee_per_gas: U256) -> Self;
}

pub trait Network: Sized + Send + Sync + 'static {
    const __ENFORCE_ZST: () = assert!(
        // This ensures that the network is a zero-sized type by checking that
        // its pointer is thin
        std::mem::size_of::<Self>() == 0,
        "Network must be a zero-sized type"
    );

    // argument for `eth_sendTransaction`
    type Transaction: Transaction + RpcObject;

    // return for `eth_getTransaction`
    type TransactionRespose: RpcObject;

    // return for `eth_getTransactionReceipt`
    type Receipt: RpcObject;
}

type MwareCall<'a, M, N, Params, Resp> =
    RpcCall<&'a <M as Middleware<N>>::Connection, <M as Middleware<N>>::Connection, Params, Resp>;

type MwareFut<'a, M, N, T> =
    Pin<Box<dyn Future<Output = RpcResult<T, <M as Middleware<N>>::Error>> + Send + 'a>>;

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

    fn get_transaction(
        &self,
        tx_hash: TxHash,
    ) -> MwareCall<Self, N, TxHash, N::TransactionRespose> {
        self.inner().get_transaction(tx_hash)
    }

    fn estimate_gas(&self, tx: &N::Transaction) -> MwareCall<Self, N, N::Transaction, U256> {
        self.inner().estimate_gas(tx)
    }

    fn populate_gas<'a>(&'a self, tx: &'a mut N::Transaction) -> MwareFut<'a, Self, N, ()> {
        let est = self.estimate_gas(tx);
        Box::pin(async move {
            let res = est.await;
            res.map(|gas| tx.set_gas(gas)).convert_err()
        })
    }
}

#[cfg(test)]
mod tests {}
