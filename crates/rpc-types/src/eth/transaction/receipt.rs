#![allow(unknown_lints, non_local_definitions)]

use crate::{Log, WithOtherFields};
use alloy_consensus::{AnyReceiptEnvelope, ReceiptEnvelope, TxType};
use alloy_primitives::{Address, B256};
use serde::{Deserialize, Serialize};

/// Transaction receipt
///
/// This type is generic over an inner [`ReceiptEnvelope`] which contains
/// consensus data and metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    any(test, feature = "arbitrary"),
    derive(proptest_derive::Arbitrary, arbitrary::Arbitrary)
)]
#[serde(rename_all = "camelCase")]
pub struct TransactionReceipt<T = ReceiptEnvelope<Log>> {
    /// The receipt envelope, which contains the consensus receipt data..
    #[serde(flatten)]
    pub inner: T,
    /// Transaction Hash.
    pub transaction_hash: B256,
    /// Index within the block.
    #[serde(
        default,
        with = "alloy_serde::u64_opt_via_ruint",
        skip_serializing_if = "Option::is_none"
    )]
    pub transaction_index: Option<u64>,
    /// Hash of the block this transaction was included within.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_hash: Option<B256>,
    /// Number of the block this transaction was included within.
    #[serde(
        default,
        with = "alloy_serde::u64_opt_via_ruint",
        skip_serializing_if = "Option::is_none"
    )]
    pub block_number: Option<u64>,
    /// Gas used by this transaction alone.
    #[serde(with = "alloy_serde::u128_via_ruint")]
    pub gas_used: u128,
    /// The price paid post-execution by the transaction (i.e. base fee + priority fee). Both
    /// fields in 1559-style transactions are maximums (max fee + max priority fee), the amount
    /// that's actually paid by users can only be determined post-execution
    #[serde(with = "alloy_serde::u128_via_ruint")]
    pub effective_gas_price: u128,
    /// Blob gas used by the eip-4844 transaction
    ///
    /// This is None for non eip-4844 transactions
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "alloy_serde::u128_opt_via_ruint",
        default
    )]
    pub blob_gas_used: Option<u128>,
    /// The price paid by the eip-4844 transaction per blob gas.
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "alloy_serde::u128_opt_via_ruint",
        default
    )]
    pub blob_gas_price: Option<u128>,
    /// Address of the sender
    pub from: Address,
    /// Address of the receiver. None when its a contract creation transaction.
    pub to: Option<Address>,
    /// Contract address created, or None if not a deployment.
    pub contract_address: Option<Address>,
    /// The post-transaction stateroot (pre Byzantium)
    ///
    /// EIP98 makes this optional field, if it's missing then skip serializing it
    #[serde(skip_serializing_if = "Option::is_none", rename = "root")]
    pub state_root: Option<B256>,
}

impl AsRef<ReceiptEnvelope<Log>> for TransactionReceipt {
    fn as_ref(&self) -> &ReceiptEnvelope<Log> {
        &self.inner
    }
}

impl TransactionReceipt {
    /// Returns the status of the transaction.
    pub const fn status(&self) -> bool {
        match &self.inner {
            ReceiptEnvelope::Eip1559(receipt)
            | ReceiptEnvelope::Eip2930(receipt)
            | ReceiptEnvelope::Eip4844(receipt)
            | ReceiptEnvelope::Legacy(receipt) => receipt.receipt.status,
            _ => false,
        }
    }

    /// Returns the transaction type.
    pub const fn transaction_type(&self) -> TxType {
        self.inner.tx_type()
    }

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

impl<T> TransactionReceipt<T> {
    /// Maps the inner receipt value of this receipt.
    pub fn map_inner<U, F>(self, f: F) -> TransactionReceipt<U>
    where
        F: FnOnce(T) -> U,
    {
        TransactionReceipt {
            inner: f(self.inner),
            transaction_hash: self.transaction_hash,
            transaction_index: self.transaction_index,
            block_hash: self.block_hash,
            block_number: self.block_number,
            gas_used: self.gas_used,
            effective_gas_price: self.effective_gas_price,
            blob_gas_used: self.blob_gas_used,
            blob_gas_price: self.blob_gas_price,
            from: self.from,
            to: self.to,
            contract_address: self.contract_address,
            state_root: self.state_root,
        }
    }
}

/// Alias for a catch-all receipt type.
pub type AnyTransactionReceipt = WithOtherFields<TransactionReceipt<AnyReceiptEnvelope<Log>>>;

#[cfg(test)]
mod test {
    use super::*;
    use alloy_consensus::{Receipt, ReceiptWithBloom};
    use alloy_primitives::{address, b256, bloom, Bloom};
    use arbitrary::Arbitrary;
    use rand::Rng;

    #[test]
    fn transaction_receipt_arbitrary() {
        let mut bytes = [0u8; 1024];
        rand::thread_rng().fill(bytes.as_mut_slice());

        let _: TransactionReceipt =
            TransactionReceipt::arbitrary(&mut arbitrary::Unstructured::new(&bytes)).unwrap();
    }

    #[test]
    fn test_sanity() {
        let json_str = r#"{"transactionHash":"0x21f6554c28453a01e7276c1db2fc1695bb512b170818bfa98fa8136433100616","blockHash":"0x4acbdefb861ef4adedb135ca52865f6743451bfbfa35db78076f881a40401a5e","blockNumber":"0x129f4b9","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000200000000000000000040000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000400000800000000000000000000000000000000004000000000000000000800000000100000020000000000000000000080000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000010000000000000000000000000000","gasUsed":"0xbde1","contractAddress":null,"cumulativeGasUsed":"0xa42aec","transactionIndex":"0x7f","from":"0x9a53bfba35269414f3b2d20b52ca01b15932c7b2","to":"0xdac17f958d2ee523a2206206994597c13d831ec7","type":"0x2","effectiveGasPrice":"0xfb0f6e8c9","logs":[{"blockHash":"0x4acbdefb861ef4adedb135ca52865f6743451bfbfa35db78076f881a40401a5e","address":"0xdac17f958d2ee523a2206206994597c13d831ec7","logIndex":"0x118","data":"0x00000000000000000000000000000000000000000052b7d2dcc80cd2e4000000","removed":false,"topics":["0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925","0x0000000000000000000000009a53bfba35269414f3b2d20b52ca01b15932c7b2","0x00000000000000000000000039e5dbb9d2fead31234d7c647d6ce77d85826f76"],"blockNumber":"0x129f4b9","transactionIndex":"0x7f","transactionHash":"0x21f6554c28453a01e7276c1db2fc1695bb512b170818bfa98fa8136433100616"}],"status":"0x1"}"#;

        let receipt: TransactionReceipt = serde_json::from_str(json_str).unwrap();
        assert_eq!(
            receipt.transaction_hash,
            b256!("21f6554c28453a01e7276c1db2fc1695bb512b170818bfa98fa8136433100616")
        );

        const EXPECTED_BLOOM: Bloom = bloom!("00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000200000000000000000040000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000400000800000000000000000000000000000000004000000000000000000800000000100000020000000000000000000080000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000010000000000000000000000000000");
        const EXPECTED_CGU: u128 = 0xa42aec;

        assert!(matches!(
            receipt.inner,
            ReceiptEnvelope::Eip1559(ReceiptWithBloom {
                receipt: Receipt { status: true, cumulative_gas_used: EXPECTED_CGU, .. },
                logs_bloom: EXPECTED_BLOOM
            })
        ));

        let log = receipt.inner.as_receipt().unwrap().logs.first().unwrap();
        assert_eq!(log.address(), address!("dac17f958d2ee523a2206206994597c13d831ec7"));
        assert_eq!(log.log_index, Some(0x118));
        assert_eq!(
            log.topics(),
            vec![
                b256!("8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925"),
                b256!("0000000000000000000000009a53bfba35269414f3b2d20b52ca01b15932c7b2"),
                b256!("00000000000000000000000039e5dbb9d2fead31234d7c647d6ce77d85826f76")
            ],
        );

        assert_eq!(
            serde_json::to_value(&receipt).unwrap(),
            serde_json::from_str::<serde_json::Value>(json_str).unwrap()
        );
    }
}
