use alloy_primitives::U256;
use alloy_rlp::{Decodable, Encodable};
use alloy_transports::RpcObject;

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

pub trait Network {
    // argument for `eth_sendTransaction` & return for `eth_getTransaction`
    type Transaction: Transaction + RpcObject;
    // return for `eth_getTransactionReceipt`
    type Receipt: RpcObject;
}

pub trait Middleware<N> {
    type Inner: Middleware<N>;
}

#[cfg(test)]
mod tests {}
