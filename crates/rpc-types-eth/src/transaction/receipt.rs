use crate::Log;
use alloc::vec::Vec;
use alloy_consensus::{ReceiptEnvelope, TxReceipt, TxType};
use alloy_eips::eip7702::SignedAuthorization;
use alloy_network_primitives::ReceiptResponse;
use alloy_primitives::{Address, BlockHash, TxHash, B256};

/// Transaction receipt
///
/// This type is generic over an inner [`ReceiptEnvelope`] which contains
/// consensus data and metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[doc(alias = "TxReceipt")]
pub struct TransactionReceipt<T = ReceiptEnvelope<Log>> {
    /// The receipt envelope, which contains the consensus receipt data..
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub inner: T,
    /// Transaction Hash.
    #[doc(alias = "tx_hash")]
    pub transaction_hash: TxHash,
    /// Index within the block.
    #[cfg_attr(feature = "serde", serde(default, with = "alloy_serde::quantity::opt"))]
    #[doc(alias = "tx_index")]
    pub transaction_index: Option<u64>,
    /// Hash of the block this transaction was included within.
    #[cfg_attr(feature = "serde", serde(default))]
    pub block_hash: Option<BlockHash>,
    /// Number of the block this transaction was included within.
    #[cfg_attr(feature = "serde", serde(default, with = "alloy_serde::quantity::opt"))]
    pub block_number: Option<u64>,
    /// Gas used by this transaction alone.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub gas_used: u128,
    /// The price paid post-execution by the transaction (i.e. base fee + priority fee). Both
    /// fields in 1559-style transactions are maximums (max fee + max priority fee), the amount
    /// that's actually paid by users can only be determined post-execution
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub effective_gas_price: u128,
    /// Blob gas used by the eip-4844 transaction
    ///
    /// This is None for non eip-4844 transactions
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing_if = "Option::is_none",
            with = "alloy_serde::quantity::opt",
            default
        )
    )]
    pub blob_gas_used: Option<u128>,
    /// The price paid by the eip-4844 transaction per blob gas.
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing_if = "Option::is_none",
            with = "alloy_serde::quantity::opt",
            default
        )
    )]
    pub blob_gas_price: Option<u128>,
    /// Address of the sender
    pub from: Address,
    /// Address of the receiver. None when its a contract creation transaction.
    pub to: Option<Address>,
    /// Contract address created, or None if not a deployment.
    pub contract_address: Option<Address>,
    /// The authorization list is a list of tuples that store the address to code which the signer
    /// desires to execute in the context of their EOA.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub authorization_list: Option<Vec<SignedAuthorization>>,
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
            | ReceiptEnvelope::Eip7702(receipt)
            | ReceiptEnvelope::Legacy(receipt) => receipt.receipt.status.coerce_status(),
            _ => false,
        }
    }

    /// Returns the transaction type.
    #[doc(alias = "tx_type")]
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
            authorization_list: self.authorization_list,
        }
    }
}

/// Alias for a catch-all receipt type.
#[doc(alias = "AnyTxReceipt")]
#[cfg(feature = "serde")]
pub type AnyTransactionReceipt =
    alloy_serde::WithOtherFields<TransactionReceipt<alloy_consensus::AnyReceiptEnvelope<Log>>>;

impl<T: TxReceipt<Log>> ReceiptResponse for TransactionReceipt<T> {
    fn contract_address(&self) -> Option<Address> {
        self.contract_address
    }

    fn status(&self) -> bool {
        self.inner.status()
    }

    fn block_hash(&self) -> Option<BlockHash> {
        self.block_hash
    }

    fn block_number(&self) -> Option<u64> {
        self.block_number
    }

    fn transaction_hash(&self) -> TxHash {
        self.transaction_hash
    }

    fn transaction_index(&self) -> Option<u64> {
        self.transaction_index
    }

    fn gas_used(&self) -> u128 {
        self.gas_used
    }

    fn effective_gas_price(&self) -> u128 {
        self.effective_gas_price
    }

    fn blob_gas_used(&self) -> Option<u128> {
        self.blob_gas_used
    }

    fn blob_gas_price(&self) -> Option<u128> {
        self.blob_gas_price
    }

    fn from(&self) -> Address {
        self.from
    }

    fn to(&self) -> Option<Address> {
        self.to
    }

    fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
        self.authorization_list.as_deref()
    }

    fn cumulative_gas_used(&self) -> u128 {
        self.inner.cumulative_gas_used()
    }

    fn state_root(&self) -> Option<B256> {
        self.inner.status_or_post_state().as_post_state()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::TransactionReceipt;
    use alloy_consensus::{Eip658Value, Receipt, ReceiptWithBloom};
    use alloy_primitives::{address, b256, bloom, Bloom};
    use arbitrary::Arbitrary;
    use rand::Rng;
    use similar_asserts::assert_eq;

    #[test]
    fn transaction_receipt_arbitrary() {
        let mut bytes = [0u8; 1024];
        rand::thread_rng().fill(bytes.as_mut_slice());

        let _: TransactionReceipt =
            TransactionReceipt::arbitrary(&mut arbitrary::Unstructured::new(&bytes)).unwrap();
    }

    #[test]
    #[cfg(feature = "serde")]
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
                receipt: Receipt {
                    status: Eip658Value::Eip658(true),
                    cumulative_gas_used: EXPECTED_CGU,
                    ..
                },
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

    #[test]
    #[cfg(feature = "serde")]
    fn deserialize_tx_receipt_op() {
        // OtherFields for Optimism
        #[derive(Debug, serde::Deserialize)]
        struct OpOtherFields {
            #[serde(rename = "l1BaseFeeScalar")]
            l1_base_fee_scalar: String,
            #[serde(rename = "l1BlobBaseFee")]
            l1_blob_base_fee: String,
            #[serde(rename = "l1BlobBaseFeeScalar")]
            l1_blob_base_fee_scalar: String,
            #[serde(rename = "l1Fee")]
            l1_fee: String,
            #[serde(rename = "l1GasPrice")]
            l1_gas_price: String,
            #[serde(rename = "l1GasUsed")]
            l1_gas_used: String,
        }

        let receipt_json = r#"
        {
            "status": "0x1",
            "cumulativeGasUsed": "0xf1740",
            "logs": [
                {
                "address": "0x4200000000000000000000000000000000000006",
                "topics": [
                    "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
                    "0x0000000000000000000000005112996d3ae99f0b5360cea1a620ffcd78e8ff83",
                    "0x00000000000000000000000077e7c5cbeaad915cf5462064b02984e16a902e67"
                ],
                "data": "0x000000000000000000000000000000000000000000000000001c66f6e8b40c00",
                "blockHash": "0x88e07a0d797b84bd122d6993a6faf5a59ada7f40c181c553c191dd400d3d1583",
                "blockNumber": "0x73a43e1",
                "transactionHash": "0x2bc7cb4648e847712e39abd42178e35214a70bb15c568d604687661b9539b4c2",
                "transactionIndex": "0x9",
                "logIndex": "0x16",
                "removed": false
                }
            ],
            "logsBloom": "0x00000000000000000000000000000000000000000000000000040000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000008000000100000000000000000100000000000000000000010000020000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000000000200000000000000000000002000000000000000000001000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "type": "0x0",
            "transactionHash": "0x2bc7cb4648e847712e39abd42178e35214a70bb15c568d604687661b9539b4c2",
            "transactionIndex": "0x9",
            "blockHash": "0x88e07a0d797b84bd122d6993a6faf5a59ada7f40c181c553c191dd400d3d1583",
            "blockNumber": "0x73a43e1",
            "gasUsed": "0x85b9",
            "effectiveGasPrice": "0x3ac9e84",
            "from": "0x5112996d3ae99f0b5360cea1a620ffcd78e8ff83",
            "to": "0x4200000000000000000000000000000000000006",
            "contractAddress": null,
            "l1BaseFeeScalar": "0x558",
            "l1BlobBaseFee": "0x1",
            "l1BlobBaseFeeScalar": "0xc5fc5",
            "l1Fee": "0x105d4b2024",
            "l1GasPrice": "0x5d749a07e",
            "l1GasUsed": "0x800"
        }
        "#;
        let receipt = serde_json::from_str::<AnyTransactionReceipt>(receipt_json).unwrap();

        assert_eq!(
            receipt.transaction_hash,
            b256!("2bc7cb4648e847712e39abd42178e35214a70bb15c568d604687661b9539b4c2")
        );

        let other: OpOtherFields = receipt.other.deserialize_into().unwrap();
        assert_eq!(other.l1_base_fee_scalar, "0x558");
        assert_eq!(other.l1_blob_base_fee, "0x1");
        assert_eq!(other.l1_blob_base_fee_scalar, "0xc5fc5");
        assert_eq!(other.l1_fee, "0x105d4b2024");
        assert_eq!(other.l1_gas_price, "0x5d749a07e");
        assert_eq!(other.l1_gas_used, "0x800");
    }

    #[test]
    #[cfg(feature = "serde")]
    fn deserialize_tx_receipt_arb() {
        // OtherFields for Arbitrum
        #[derive(Debug, serde::Deserialize)]
        struct ArbOtherFields {
            #[serde(rename = "gasUsedForL1")]
            gas_used_for_l1: String,
            #[serde(rename = "l1BlockNumber")]
            l1_block_number: String,
        }

        let receipt_json = r#"
        {
            "status": "0x1",
            "cumulativeGasUsed": "0x27ebb8",
            "logs": [
                {
                "address": "0x912ce59144191c1204e64559fe8253a0e49e6548",
                "topics": [
                    "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
                    "0x000000000000000000000000e487d95426e55a29f2266e6788ab55608ebb829b",
                    "0x0000000000000000000000009855134ed0c8b71266d9f3e15c0a518c07be5baf"
                ],
                "data": "0x00000000000000000000000000000000000000000000000009d40825d5ee8000",
                "blockHash": "0x83ddb8850803238bd58615680bc3718686ec1e3deaea0bc5f67c07c8577547f5",
                "blockNumber": "0xd288ac5",
                "transactionHash": "0x5aeca744e0c1f6d7f68641aedd394ac4b6e18cbeac3f8b3c81056c0e51a61cf3",
                "transactionIndex": "0x7",
                "logIndex": "0x7",
                "removed": false
                }
            ],
            "logsBloom": "0x00000000000000000000000000000000000000000000000000000000005000020000000000000000000000000000000000000000000000000000000000000000000000000000000000000008000100000000000001000000000000000000000000000000000000000000020000000000000000000004400000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "type": "0x0",
            "transactionHash": "0x5aeca744e0c1f6d7f68641aedd394ac4b6e18cbeac3f8b3c81056c0e51a61cf3",
            "transactionIndex": "0x7",
            "blockHash": "0x83ddb8850803238bd58615680bc3718686ec1e3deaea0bc5f67c07c8577547f5",
            "blockNumber": "0xd288ac5",
            "gasUsed": "0x3ad89",
            "effectiveGasPrice": "0x989680",
            "from": "0xe487d95426e55a29f2266e6788ab55608ebb829b",
            "to": "0x912ce59144191c1204e64559fe8253a0e49e6548",
            "contractAddress": null,
            "gasUsedForL1": "0x2c906",
            "l1BlockNumber": "0x1323b96"
        }
        "#;
        let receipt = serde_json::from_str::<AnyTransactionReceipt>(receipt_json).unwrap();

        assert_eq!(
            receipt.transaction_hash,
            b256!("5aeca744e0c1f6d7f68641aedd394ac4b6e18cbeac3f8b3c81056c0e51a61cf3")
        );

        let other: ArbOtherFields = receipt.other.deserialize_into().unwrap();
        assert_eq!(other.gas_used_for_l1, "0x2c906");
        assert_eq!(other.l1_block_number, "0x1323b96");
    }

    #[test]
    #[cfg(feature = "serde")]
    fn deserialize_pre_eip658_receipt() {
        let receipt_json = r#"
        {
            "transactionHash": "0xea1093d492a1dcb1bef708f771a99a96ff05dcab81ca76c31940300177fcf49f",
            "blockHash": "0x8e38b4dbf6b11fcc3b9dee84fb7986e29ca0a02cecd8977c161ff7333329681e",
            "blockNumber": "0xf4240",
            "logsBloom": "0x00000000000000000000000000000000000800000000000000000000000800000000000000000400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000400000000000000000",
            "gasUsed": "0x723c",
            "root": "0x284d35bf53b82ef480ab4208527325477439c64fb90ef518450f05ee151c8e10",
            "contractAddress": null,
            "cumulativeGasUsed": "0x723c",
            "transactionIndex": "0x0",
            "from": "0x39fa8c5f2793459d6622857e7d9fbb4bd91766d3",
            "to": "0xc083e9947cf02b8ffc7d3090ae9aea72df98fd47",
            "type": "0x0",
            "effectiveGasPrice": "0x12bfb19e60",
            "logs": [
                {
                    "blockHash": "0x8e38b4dbf6b11fcc3b9dee84fb7986e29ca0a02cecd8977c161ff7333329681e",
                    "address": "0xc083e9947cf02b8ffc7d3090ae9aea72df98fd47",
                    "logIndex": "0x0",
                    "data": "0x00000000000000000000000039fa8c5f2793459d6622857e7d9fbb4bd91766d30000000000000000000000000000000000000000000000056bc75e2d63100000",
                    "removed": false,
                    "topics": [
                    "0xe1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c"
                    ],
                    "blockNumber": "0xf4240",
                    "transactionIndex": "0x0",
                    "transactionHash": "0xea1093d492a1dcb1bef708f771a99a96ff05dcab81ca76c31940300177fcf49f"
                }
            ]
        }
        "#;

        let receipt = serde_json::from_str::<TransactionReceipt>(receipt_json).unwrap();

        assert_eq!(
            receipt.transaction_hash,
            b256!("ea1093d492a1dcb1bef708f771a99a96ff05dcab81ca76c31940300177fcf49f")
        );
    }
}
