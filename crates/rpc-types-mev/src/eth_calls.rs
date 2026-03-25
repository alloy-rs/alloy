use crate::{u256_numeric_string, Privacy, Validity};

use alloy_eips::{eip2718::Encodable2718, BlockNumberOrTag};
use alloy_primitives::{keccak256, Address, Bytes, Keccak256, TxHash, B256, U256};
use alloy_rpc_types_eth::TransactionIndex;
use alloy_serde::OtherFields;
use serde::{Deserialize, Serialize};

/// Bundle of transactions for `eth_callBundle`
///
/// <https://docs.flashbots.net/flashbots-auction/searchers/advanced/rpc-endpoint#eth_callBundle>
/// <https://github.com/flashbots/mev-geth/blob/fddf97beec5877483f879a77b7dea2e58a58d653/internal/ethapi/api.go#L2049>
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EthCallBundle {
    /// A list of hex-encoded signed transactions
    pub txs: Vec<Bytes>,
    /// hex encoded block number for which this bundle is valid on
    #[serde(with = "alloy_serde::quantity")]
    pub block_number: u64,
    /// Either a hex encoded number or a block tag for which state to base this simulation on
    pub state_block_number: BlockNumberOrTag,
    /// Inclusive number of tx to replay in block. -1 means replay all
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transaction_index: Option<TransactionIndex>,
    /// the coinbase to use for this bundle simulation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coinbase: Option<Address>,
    /// the timestamp to use for this bundle simulation, in seconds since the unix epoch
    #[serde(default, with = "alloy_serde::quantity::opt", skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
    /// the timeout to apply to execution of this bundle, in milliseconds
    #[serde(default, with = "alloy_serde::quantity::opt", skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    /// gas limit of the block to use for this simulation
    #[serde(default, with = "alloy_serde::quantity::opt", skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<u64>,
    /// difficulty of the block to use for this simulation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<U256>,
    /// basefee of the block to use for this simulation
    #[serde(default, with = "alloy_serde::quantity::opt", skip_serializing_if = "Option::is_none")]
    pub base_fee: Option<u128>,
}

impl EthCallBundle {
    /// Creates a new bundle from the given [`Encodable2718`] transactions.
    pub fn from_2718<I, T>(txs: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Encodable2718,
    {
        Self::from_raw_txs(txs.into_iter().map(|tx| tx.encoded_2718()))
    }

    /// Creates a new bundle with the given transactions.
    pub fn from_raw_txs<I, T>(txs: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<Bytes>,
    {
        Self { txs: txs.into_iter().map(Into::into).collect(), ..Default::default() }
    }

    /// Adds an [`Encodable2718`] transaction to the bundle.
    pub fn append_2718_tx(self, tx: impl Encodable2718) -> Self {
        self.append_raw_tx(tx.encoded_2718())
    }

    /// Adds an EIP-2718 envelope to the bundle.
    pub fn append_raw_tx(mut self, tx: impl Into<Bytes>) -> Self {
        self.txs.push(tx.into());
        self
    }

    /// Adds multiple [`Encodable2718`] transactions to the bundle.
    pub fn extend_2718_txs<I, T>(self, tx: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Encodable2718,
    {
        self.extend_raw_txs(tx.into_iter().map(|tx| tx.encoded_2718()))
    }

    /// Adds multiple calls to the block.
    pub fn extend_raw_txs<I, T>(mut self, txs: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<Bytes>,
    {
        self.txs.extend(txs.into_iter().map(Into::into));
        self
    }

    /// Sets the block number for the bundle.
    pub const fn with_block_number(mut self, block_number: u64) -> Self {
        self.block_number = block_number;
        self
    }

    /// Sets the state block number for the bundle.
    pub fn with_state_block_number(
        mut self,
        state_block_number: impl Into<BlockNumberOrTag>,
    ) -> Self {
        self.state_block_number = state_block_number.into();
        self
    }

    /// Sets the coinbase for the bundle.
    pub const fn with_coinbase(mut self, coinbase: Address) -> Self {
        self.coinbase = Some(coinbase);
        self
    }

    /// Sets the timestamp for the bundle.
    pub const fn with_timestamp(mut self, timestamp: u64) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Sets the timeout for the bundle.
    pub const fn with_timeout(mut self, timeout: u64) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets the gas limit for the bundle.
    pub const fn with_gas_limit(mut self, gas_limit: u64) -> Self {
        self.gas_limit = Some(gas_limit);
        self
    }

    /// Sets the difficulty for the bundle.
    pub const fn with_difficulty(mut self, difficulty: U256) -> Self {
        self.difficulty = Some(difficulty);
        self
    }

    /// Sets the base fee for the bundle.
    pub const fn with_base_fee(mut self, base_fee: u128) -> Self {
        self.base_fee = Some(base_fee);
        self
    }
}

/// Response for `eth_callBundle`
///
/// <https://docs.flashbots.net/flashbots-auction/advanced/rpc-endpoint#eth_callbundle>
/// <https://github.com/flashbots/mev-geth/blob/fddf97beec5877483f879a77b7dea2e58a58d653/internal/ethapi/api.go#L2212-L2220>
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EthCallBundleResponse {
    /// The hash of the bundle bodies.
    pub bundle_hash: B256,
    /// The gas price of the entire bundle
    #[serde(with = "u256_numeric_string")]
    pub bundle_gas_price: U256,
    /// The difference in Ether sent to the coinbase after all transactions in the bundle
    #[serde(with = "u256_numeric_string")]
    pub coinbase_diff: U256,
    /// The total amount of Ether sent to the coinbase after all transactions in the bundle
    #[serde(with = "u256_numeric_string")]
    pub eth_sent_to_coinbase: U256,
    /// The total gas fees paid for all transactions in the bundle
    #[serde(with = "u256_numeric_string")]
    pub gas_fees: U256,
    /// Results of individual transactions within the bundle
    pub results: Vec<EthCallBundleTransactionResult>,
    /// The block number used as a base for this simulation
    pub state_block_number: u64,
    /// The total gas used by all transactions in the bundle
    pub total_gas_used: u64,
}

/// Result of a single transaction in a bundle for `eth_callBundle`
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EthCallBundleTransactionResult {
    /// The difference in Ether sent to the coinbase after the transaction
    #[serde(with = "u256_numeric_string")]
    pub coinbase_diff: U256,
    /// The amount of Ether sent to the coinbase after the transaction
    #[serde(with = "u256_numeric_string")]
    pub eth_sent_to_coinbase: U256,
    /// The address from which the transaction originated
    pub from_address: Address,
    /// The gas fees paid for the transaction
    #[serde(with = "u256_numeric_string")]
    pub gas_fees: U256,
    /// The gas price used for the transaction
    #[serde(with = "u256_numeric_string")]
    pub gas_price: U256,
    /// The amount of gas used by the transaction
    pub gas_used: u64,
    /// The address to which the transaction is sent (optional)
    pub to_address: Option<Address>,
    /// The transaction hash
    pub tx_hash: B256,
    /// Contains the return data if the transaction succeeded
    ///
    /// Note: this is mutually exclusive with `revert`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Bytes>,
    /// Contains the return data if the transaction reverted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revert: Option<Bytes>,
}

/// Request for `eth_cancelBundle`
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EthCancelBundle {
    /// The replacement UUID of the bundle to be canceled
    pub replacement_uuid: String,
}

/// Request for `eth_cancelPrivateTransaction`
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EthCancelPrivateTransaction {
    /// Transaction hash of the transaction to be canceled
    pub tx_hash: B256,
}

/// Bundle of transactions for `eth_sendBundle`
///
/// Note: this is for `eth_sendBundle` and not `mev_sendBundle`
///
/// This implement the refund capabilities from here:
/// <https://buildernet.org/docs/api#eth_sendbundle>
/// But keeps compatibility with original Flashbots API:
/// <https://docs.flashbots.net/flashbots-auction/searchers/advanced/rpc-endpoint#eth_sendbundle>
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EthSendBundle {
    /// A list of hex-encoded signed transactions
    pub txs: Vec<Bytes>,
    /// hex-encoded block number for which this bundle is valid
    #[serde(with = "alloy_serde::quantity")]
    pub block_number: u64,
    /// unix timestamp when this bundle becomes active
    #[serde(
        default,
        deserialize_with = "alloy_serde::quantity::opt::deserialize",
        skip_serializing_if = "Option::is_none"
    )]
    pub min_timestamp: Option<u64>,
    /// unix timestamp how long this bundle stays valid
    #[serde(
        default,
        deserialize_with = "alloy_serde::quantity::opt::deserialize",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_timestamp: Option<u64>,
    /// list of hashes of possibly reverting txs
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reverting_tx_hashes: Vec<TxHash>,
    /// UUID that can be used to cancel/replace this bundle
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement_uuid: Option<String>,
    /// A list of tx hashes that are allowed to be discarded
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dropping_tx_hashes: Vec<TxHash>,
    /// The percent that should be refunded to refund recipient
    #[serde(
        default,
        deserialize_with = "alloy_serde::quantity::opt::deserialize",
        skip_serializing_if = "Option::is_none"
    )]
    pub refund_percent: Option<u8>,
    /// The address that receives the refund
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refund_recipient: Option<Address>,
    /// A list of tx hashes used to determine the refund
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refund_tx_hashes: Vec<TxHash>,
    /// Additional fields that are specific to the builder
    #[serde(flatten, default)]
    pub extra_fields: OtherFields,
}

impl EthSendBundle {
    /// Returns the bundle hash.
    ///
    /// This is the keccak256 hash of the transaction hashes of the
    /// transactions in the bundle.
    ///
    /// ## Note
    ///
    /// Logic for calculating the bundle hash is as follows:
    /// - Calculate the hash of each transaction in the bundle
    /// - Concatenate the hashes, in bundle order
    /// - Calculate the keccak256 hash of the concatenated hashes
    ///
    /// See the [flashbots impl].
    ///
    /// This function will not verify transaction correctness. If the bundle
    /// `txs` contains invalid transactions, the bundle hash will still be
    /// calculated.
    ///
    /// [flashbots impl]: https://github.com/flashbots/mev-geth/blob/fddf97beec5877483f879a77b7dea2e58a58d653/internal/ethapi/api.go#L2067
    pub fn bundle_hash(&self) -> B256 {
        let mut hasher = Keccak256::default();
        for tx in &self.txs {
            // NB: the txs must contain envelopes, so the tx_hash is just the
            // keccak256 hash of the envelope. no need to deserialize the tx
            hasher.update(keccak256(tx));
        }
        hasher.finalize()
    }
}

/// Response from the matchmaker after sending a bundle.
#[derive(Deserialize, Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EthBundleHash {
    /// Hash of the bundle bodies.
    pub bundle_hash: B256,
}

/// Request for `eth_sendPrivateTransaction`
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EthSendPrivateTransaction {
    /// raw signed transaction
    pub tx: Bytes,
    /// Hex-encoded number string, optional. Highest block number in which the transaction should
    /// be included.
    #[serde(default, with = "alloy_serde::quantity::opt", skip_serializing_if = "Option::is_none")]
    pub max_block_number: Option<u64>,
    /// Preferences for private transaction.
    #[serde(default, skip_serializing_if = "PrivateTransactionPreferences::is_empty")]
    pub preferences: PrivateTransactionPreferences,
}

impl EthSendPrivateTransaction {
    /// Creates new [`EthSendPrivateTransaction`] from the given encodable transaction.
    pub fn new<T: Encodable2718>(tx: &T) -> Self {
        Self {
            tx: tx.encoded_2718().into(),
            max_block_number: None,
            preferences: Default::default(),
        }
    }

    /// Apply a function to the request, returning the modified request.
    pub fn apply<F>(self, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        f(self)
    }

    /// Sets private tx's max block number.
    pub const fn max_block_number(mut self, num: u64) -> Self {
        self.max_block_number = Some(num);
        self
    }

    /// Sets private tx's preferences.
    pub fn with_preferences(mut self, preferences: PrivateTransactionPreferences) -> Self {
        self.preferences = preferences;
        self
    }
}

impl<T: Encodable2718> From<T> for EthSendPrivateTransaction {
    fn from(envelope: T) -> Self {
        Self::new(&envelope)
    }
}

/// Additional preferences for `eth_sendPrivateTransaction`
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct PrivateTransactionPreferences {
    /// Indicates whether the transaction should be processed in fast mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fast: Option<bool>,
    /// Requirements for the bundle to be included in the block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validity: Option<Validity>,
    /// Preferences on what data should be shared about the bundle and its transactions
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privacy: Option<Privacy>,
}

impl PrivateTransactionPreferences {
    /// Returns true if the preferences are empty.
    pub const fn is_empty(&self) -> bool {
        self.fast.is_none() && self.validity.is_none() && self.privacy.is_none()
    }

    /// Sets the `fast` option to true
    pub const fn into_fast(self) -> Self {
        self.with_fast_mode(true)
    }

    /// Sets mode of tx execution
    pub const fn with_fast_mode(mut self, fast: bool) -> Self {
        self.fast = Some(fast);
        self
    }

    /// Sets tx's validity
    pub fn with_validity(mut self, validity: Validity) -> Self {
        self.validity = Some(validity);
        self
    }

    /// Sets tx's privacy
    pub fn with_privacy(mut self, privacy: Privacy) -> Self {
        self.privacy = Some(privacy);
        self
    }
}

/// Bundle of blob transaction permutations for `eth_sendBlobs`
///
/// Sends multiple blob transaction permutations with the same nonce. These are conflicting
/// transactions with different amounts of blobs where only one may be included.
///
/// For more details see:
/// <https://docs.titanbuilder.xyz/api/eth_sendblobs>
/// See also EIP-7925 draft:
/// <https://ethereum-magicians.org/t/23333>
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EthSendBlobs {
    /// A list of hex-encoded signed blob transactions (permutations with same nonce, different
    /// blob counts)
    pub txs: Vec<Bytes>,
    /// Hex-encoded number string, optional. Highest block number in which one of the transactions
    /// should be included.
    #[serde(default, with = "alloy_serde::quantity::opt", skip_serializing_if = "Option::is_none")]
    pub max_block_number: Option<u64>,
}

/// Bundle of transactions for `eth_sendEndOfBlockBundle`
///
/// For more details see:
/// <https://docs.titanbuilder.xyz/api/eth_sendendofblockbundle> or
/// <https://docs.quasar.win/eth_sendendofblockbundle>
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EthSendEndOfBlockBundle {
    /// A list of hex-encoded signed transactions
    pub txs: Vec<Bytes>,
    /// Hex-encoded block number for which this bundle is valid on
    #[serde(default, with = "alloy_serde::quantity::opt", skip_serializing_if = "Option::is_none")]
    pub block_number: Option<u64>,
    /// List of hashes of possibly reverting txs
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reverting_tx_hashes: Vec<TxHash>,
    /// Pool addresses targeted by the bundle
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub target_pools: Vec<Address>,
}

#[cfg(test)]
mod tests {
    use super::EthCallBundleResponse;
    use crate::EthSendBundle;
    use alloy_primitives::{address, b256, bytes};
    use alloy_serde::OtherFields;
    use serde_json::json;

    #[test]
    fn can_deserialize_eth_call_resp() {
        let s = r#"{ "bundleGasPrice": "476190476193",
"bundleHash": "0x73b1e258c7a42fd0230b2fd05529c5d4b6fcb66c227783f8bece8aeacdd1db2e",
"coinbaseDiff": "20000000000126000",
"ethSentToCoinbase": "20000000000000000",
"gasFees": "126000",
"results": [
  {
    "coinbaseDiff": "10000000000063000",
    "ethSentToCoinbase": "10000000000000000",
    "fromAddress": "0x02a727155aef8609c9f7f2179b2a1f560b39f5a0",
    "gasFees": "63000",
    "gasPrice": "476190476193",
    "gasUsed": 21000,
    "toAddress": "0x73625f59cadc5009cb458b751b3e7b6b48c06f2c",
    "txHash": "0x669b4704a7d993a946cdd6e2f95233f308ce0c4649d2e04944e8299efcaa098a",
    "value": "0x"
  },
  {
    "coinbaseDiff": "10000000000063000",
    "ethSentToCoinbase": "10000000000000000",
    "fromAddress": "0x02a727155aef8609c9f7f2179b2a1f560b39f5a0",
    "gasFees": "63000",
    "gasPrice": "476190476193",
    "gasUsed": 21000,
    "toAddress": "0x73625f59cadc5009cb458b751b3e7b6b48c06f2c",
    "txHash": "0xa839ee83465657cac01adc1d50d96c1b586ed498120a84a64749c0034b4f19fa",
    "value": "0x"
  }
],
"stateBlockNumber": 5221585,
"totalGasUsed": 42000
}"#;

        let response = serde_json::from_str::<EthCallBundleResponse>(s).unwrap();
        let json: serde_json::Value = serde_json::from_str(s).unwrap();
        similar_asserts::assert_eq!(json, serde_json::to_value(response).unwrap());
    }

    #[test]
    fn can_deserialize_eth_send_bundle() {
        let s = r#"{
                "txs": ["0x1234"],
                "blockNumber": 1,
                "minTimestamp": 2,
                "maxTimestamp": 3,
                "revertingTxHashes": ["0x1111111111111111111111111111111111111111111111111111111111111111"],
                "replacementUuid": "11111111-1111-4111-8111-111111111111",
                "droppingTxHashes": ["0x2222222222222222222222222222222222222222222222222222222222222222"],
                "refundPercent": 4,
                "refundRecipient": "0x3333333333333333333333333333333333333333",
                "refundTxHashes": ["0x4444444444444444444444444444444444444444444444444444444444444444"],
                "customField": 42
            }"#;
        let bundle = serde_json::from_str::<EthSendBundle>(s).unwrap();
        assert_eq!(bundle.txs.len(), 1);
        assert_eq!(bundle.txs.first().unwrap(), &bytes!("0x1234"));
        assert_eq!(bundle.block_number, 1);
        assert_eq!(bundle.min_timestamp, Some(2));
        assert_eq!(bundle.max_timestamp, Some(3));
        assert_eq!(bundle.reverting_tx_hashes.len(), 1);
        assert_eq!(
            bundle.reverting_tx_hashes.first().unwrap(),
            &b256!("0x1111111111111111111111111111111111111111111111111111111111111111")
        );
        assert_eq!(
            bundle.replacement_uuid,
            Some("11111111-1111-4111-8111-111111111111".to_string())
        );
        assert_eq!(bundle.dropping_tx_hashes.len(), 1);
        assert_eq!(
            bundle.dropping_tx_hashes.first().unwrap(),
            &b256!("0x2222222222222222222222222222222222222222222222222222222222222222")
        );
        assert_eq!(bundle.refund_percent, Some(4));
        assert_eq!(
            bundle.refund_recipient,
            Some(address!("0x3333333333333333333333333333333333333333"))
        );
        assert_eq!(bundle.refund_tx_hashes.len(), 1);
        assert_eq!(
            bundle.refund_tx_hashes.first().unwrap(),
            &b256!("0x4444444444444444444444444444444444444444444444444444444444444444")
        );
        assert_eq!(bundle.extra_fields.get("customField"), Some(&json!(42)));
    }

    #[test]
    fn can_deserialize_eth_send_bundle_with_hex_numbers() {
        let s = r#"{
                "txs": ["0x1234"],
                "blockNumber": "0x1",
                "minTimestamp": "0x2",
                "maxTimestamp": "0x3",
                "refundPercent": "0x4"
            }"#;
        let bundle = serde_json::from_str::<EthSendBundle>(s).unwrap();
        assert_eq!(bundle.block_number, 1);
        assert_eq!(bundle.min_timestamp, Some(2));
        assert_eq!(bundle.max_timestamp, Some(3));
        assert_eq!(bundle.refund_percent, Some(4));
    }

    #[test]
    fn can_serialize_eth_send_bundle() {
        let bundle = EthSendBundle {
            txs: vec![bytes!("0x1234")],
            block_number: 1,
            min_timestamp: Some(2),
            max_timestamp: Some(3),
            reverting_tx_hashes: vec![b256!(
                "0x1111111111111111111111111111111111111111111111111111111111111111"
            )],
            replacement_uuid: Some("11111111-1111-4111-8111-111111111111".to_string()),
            dropping_tx_hashes: vec![b256!(
                "0x2222222222222222222222222222222222222222222222222222222222222222"
            )],
            refund_percent: Some(4),
            refund_recipient: Some(address!("0x3333333333333333333333333333333333333333")),
            refund_tx_hashes: vec![b256!(
                "0x4444444444444444444444444444444444444444444444444444444444444444"
            )],
            extra_fields: OtherFields::from_iter([("customField", json!(42))]),
        };
        let s = r#"
            {
                "txs": ["0x1234"],
                "blockNumber": "0x1",
                "minTimestamp": 2,
                "maxTimestamp": 3,
                "revertingTxHashes": ["0x1111111111111111111111111111111111111111111111111111111111111111"],
                "replacementUuid": "11111111-1111-4111-8111-111111111111",
                "droppingTxHashes": ["0x2222222222222222222222222222222222222222222222222222222222222222"],
                "refundPercent": 4,
                "refundRecipient": "0x3333333333333333333333333333333333333333",
                "refundTxHashes": ["0x4444444444444444444444444444444444444444444444444444444444444444"],
                "customField": 42
            }
            "#;
        let expected: serde_json::Value = serde_json::from_str(s).unwrap();
        let value = serde_json::to_value(&bundle).unwrap();

        assert_eq!(value, expected);
    }

    #[test]
    fn skip_serialize_for_optional_fields() {
        let bundle =
            EthSendBundle { txs: vec![bytes!("0x1234")], block_number: 1, ..Default::default() };
        let s = serde_json::to_string(&bundle).unwrap();
        assert_eq!(s, r#"{"txs":["0x1234"],"blockNumber":"0x1"}"#);
    }
}
