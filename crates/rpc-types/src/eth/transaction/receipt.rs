use crate::{other::OtherFields, Log};
use alloy_primitives::{Address, Bloom, B256, U128, U256, U64, U8};
use serde::{Deserialize, Serialize};

/// Transaction receipt
#[derive(Clone, Default, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionReceipt {
    /// Transaction Hash.
    pub transaction_hash: B256,
    /// Index within the block.
    pub transaction_index: U64,
    /// Hash of the block this transaction was included within.
    pub block_hash: Option<B256>,
    /// Number of the block this transaction was included within.
    pub block_number: Option<U256>,
    /// Cumulative gas used within the block after this was executed.
    pub cumulative_gas_used: U256,
    /// Gas used by this transaction alone.
    pub gas_used: Option<U256>,
    /// The price paid post-execution by the transaction (i.e. base fee + priority fee). Both
    /// fields in 1559-style transactions are maximums (max fee + max priority fee), the amount
    /// that's actually paid by users can only be determined post-execution
    pub effective_gas_price: U128,
    /// Blob gas used by the eip-4844 transaction
    ///
    /// This is None for non eip-4844 transactions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_gas_used: Option<U128>,
    /// The price paid by the eip-4844 transaction per blob gas.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_gas_price: Option<U128>,
    /// Address of the sender
    pub from: Address,
    /// Address of the receiver. None when its a contract creation transaction.
    pub to: Option<Address>,
    /// Contract address created, or None if not a deployment.
    pub contract_address: Option<Address>,
    /// Logs emitted by this transaction.
    pub logs: Vec<Log>,
    /// Logs bloom
    pub logs_bloom: Bloom,
    /// The post-transaction stateroot (pre Byzantium)
    ///
    /// EIP98 makes this optional field, if it's missing then skip serializing it
    #[serde(skip_serializing_if = "Option::is_none", rename = "root")]
    pub state_root: Option<B256>,
    /// Status: either 1 (success) or 0 (failure). Only present after activation of EIP-658
    #[serde(skip_serializing_if = "Option::is_none", rename = "status")]
    pub status_code: Option<U64>,
    /// EIP-2718 Transaction type.
    ///
    /// For legacy transactions this returns `0`. For EIP-2718 transactions this returns the type.
    #[serde(rename = "type")]
    pub transaction_type: U8,
    /// Arbitrary extra fields.
    #[serde(flatten)]
    pub other: OtherFields,
}

impl TransactionReceipt {
    /// Calculates the address that will be created by the transaction, if any.
    ///
    /// Returns `None` if the transaction is not a contract creation (the `to` field is set), or if
    /// the `from` field is not set.
    pub fn calculate_create_address(&self, nonce: u64) -> Option<Address> {
        if self.to.is_some() {
            return None;
        }
        Some(self.from.create(nonce))
    }
}

#[cfg(any(test, feature = "arbitrary"))]
impl<'a> arbitrary::Arbitrary<'a> for TransactionReceipt {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self {
            transaction_hash: B256::arbitrary(u)?,
            transaction_index: U64::arbitrary(u)?,
            block_hash: Option::<B256>::arbitrary(u)?,
            block_number: Option::<U256>::arbitrary(u)?,
            cumulative_gas_used: U256::arbitrary(u)?,
            gas_used: Option::<U256>::arbitrary(u)?,
            effective_gas_price: U128::arbitrary(u)?,
            blob_gas_used: Option::<U128>::arbitrary(u)?,
            blob_gas_price: Option::<U128>::arbitrary(u)?,
            from: Address::arbitrary(u)?,
            to: Option::<Address>::arbitrary(u)?,
            contract_address: Option::<Address>::arbitrary(u)?,
            logs: Vec::<Log>::arbitrary(u)?,
            logs_bloom: Bloom::arbitrary(u)?,
            state_root: Option::<B256>::arbitrary(u)?,
            status_code: Option::<U64>::arbitrary(u)?,
            transaction_type: U8::arbitrary(u)?,
            other: OtherFields::arbitrary(u)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbitrary::Arbitrary;
    use rand::Rng;

    #[test]
    fn transaction_receipt_arbitrary() {
        let mut bytes = [0u8; 1024];
        rand::thread_rng().fill(bytes.as_mut_slice());

        let _ = TransactionReceipt::arbitrary(&mut arbitrary::Unstructured::new(&bytes)).unwrap();
    }
}
