//! Alloy basic Transaction Request type.
use crate::{eth::transaction::AccessList, other::OtherFields, BlobTransactionSidecar};
use alloy_primitives::{Address, Bytes, B256, U128, U256, U64, U8};
use serde::{Deserialize, Serialize};

/// Represents _all_ transaction requests to/from RPC
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TransactionRequest {
    /// from address
    pub from: Option<Address>,
    /// to address
    pub to: Option<Address>,
    /// legacy, gas Price
    #[serde(default)]
    pub gas_price: Option<U128>,
    /// max base fee per gas sender is willing to pay
    #[serde(default)]
    pub max_fee_per_gas: Option<U128>,
    /// miner tip
    #[serde(default)]
    pub max_priority_fee_per_gas: Option<U128>,
    /// Max Fee per Blob gas for EIP-4844 transactions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fee_per_blob_gas: Option<U256>,
    /// gas
    pub gas: Option<U256>,
    /// value of th tx in wei
    pub value: Option<U256>,
    /// Any additional data sent
    #[serde(default, flatten)]
    pub input: TransactionInput,
    /// Transaction nonce
    pub nonce: Option<U64>,
    /// chain id
    pub chain_id: Option<U64>,
    /// warm storage access pre-payment
    #[serde(default)]
    pub access_list: Option<AccessList>,
    /// EIP-2718 type
    #[serde(rename = "type")]
    pub transaction_type: Option<U8>,
    /// Blob Versioned Hashes for EIP-4844 transactions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_versioned_hashes: Option<Vec<B256>>,
    /// sidecar for EIP-4844 transactions
    pub sidecar: Option<BlobTransactionSidecar>,
    /// Support for arbitrary additional fields.
    #[serde(flatten)]
    pub other: OtherFields,
}

// == impl TransactionRequest ==

impl TransactionRequest {
    /// Returns the configured fee cap, if any.
    ///
    /// The returns `gas_price` (legacy) if set or `max_fee_per_gas` (EIP1559)
    #[inline]
    pub fn fee_cap(&self) -> Option<U128> {
        self.gas_price.or(self.max_fee_per_gas)
    }

    /// Returns true if the request has a `blobVersionedHashes` field but it is empty.
    #[inline]
    pub fn has_empty_blob_hashes(&self) -> bool {
        self.blob_versioned_hashes.as_ref().map(|blobs| blobs.is_empty()).unwrap_or(false)
    }

    /// Sets the `from` field in the call to the provided address
    #[inline]
    pub const fn from(mut self, from: Address) -> Self {
        self.from = Some(from);
        self
    }

    /// Sets the gas limit for the transaction.
    pub fn gas_limit(mut self, gas_limit: u64) -> Self {
        self.gas = Some(U256::from(gas_limit));
        self
    }

    /// Sets the nonce for the transaction.
    pub fn nonce(mut self, nonce: u64) -> Self {
        self.nonce = Some(U64::from(nonce));
        self
    }

    /// Sets the maximum fee per gas for the transaction.
    pub fn max_fee_per_gas(mut self, max_fee_per_gas: u128) -> Self {
        self.max_fee_per_gas = Some(U128::from(max_fee_per_gas));
        self
    }

    /// Sets the maximum priority fee per gas for the transaction.
    pub fn max_priority_fee_per_gas(mut self, max_priority_fee_per_gas: u128) -> Self {
        self.max_priority_fee_per_gas = Some(U128::from(max_priority_fee_per_gas));
        self
    }

    /// Sets the recipient address for the transaction.
    #[inline]
    pub const fn to(mut self, to: Address) -> Self {
        self.to = Some(to);
        self
    }

    /// Sets the value (amount) for the transaction.
    pub fn value(mut self, value: u128) -> Self {
        self.value = Some(U256::from(value));
        self
    }

    /// Sets the access list for the transaction.
    pub fn access_list(mut self, access_list: AccessList) -> Self {
        self.access_list = Some(access_list);
        self
    }

    /// Sets the input data for the transaction.
    pub fn input(mut self, input: TransactionInput) -> Self {
        self.input = input;
        self
    }

    /// Sets the transactions type for the transactions.
    pub fn transaction_type(mut self, transaction_type: u8) -> Self {
        self.transaction_type = Some(U8::from(transaction_type));
        self
    }

    /// Calculates the address that will be created by the transaction, if any.
    ///
    /// Returns `None` if the transaction is not a contract creation (the `to` field is set), or if
    /// the `from` or `nonce` fields are not set.
    pub fn calculate_create_address(&self) -> Option<Address> {
        if self.to.is_some() {
            return None;
        }
        let from = self.from.as_ref()?;
        let nonce = self.nonce?;
        Some(from.create(nonce.to()))
    }
}

/// Helper type that supports both `data` and `input` fields that map to transaction input data.
///
/// This is done for compatibility reasons where older implementations used `data` instead of the
/// newer, recommended `input` field.
///
/// If both fields are set, it is expected that they contain the same value, otherwise an error is
/// returned.
#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct TransactionInput {
    /// Transaction data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Bytes>,
    /// Transaction data
    ///
    /// This is the same as `input` but is used for backwards compatibility: <https://github.com/ethereum/go-ethereum/issues/15628>
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Bytes>,
}

impl TransactionInput {
    /// Creates a new instance with the given input data.
    pub const fn new(data: Bytes) -> Self {
        Self::maybe_input(Some(data))
    }

    /// Creates a new instance with the given input data.
    pub const fn maybe_input(input: Option<Bytes>) -> Self {
        Self { input, data: None }
    }

    /// Consumes the type and returns the optional input data.
    #[inline]
    pub fn into_input(self) -> Option<Bytes> {
        self.input.or(self.data)
    }

    /// Consumes the type and returns the optional input data.
    ///
    /// Returns an error if both `data` and `input` fields are set and not equal.
    #[inline]
    pub fn try_into_unique_input(self) -> Result<Option<Bytes>, TransactionInputError> {
        self.check_unique_input().map(|()| self.into_input())
    }

    /// Returns the optional input data.
    #[inline]
    pub fn input(&self) -> Option<&Bytes> {
        self.input.as_ref().or(self.data.as_ref())
    }

    /// Returns the optional input data.
    ///
    /// Returns an error if both `data` and `input` fields are set and not equal.
    #[inline]
    pub fn unique_input(&self) -> Result<Option<&Bytes>, TransactionInputError> {
        self.check_unique_input().map(|()| self.input())
    }

    fn check_unique_input(&self) -> Result<(), TransactionInputError> {
        if let (Some(input), Some(data)) = (&self.input, &self.data) {
            if input != data {
                return Err(TransactionInputError::default());
            }
        }
        Ok(())
    }
}

impl From<Bytes> for TransactionInput {
    fn from(input: Bytes) -> Self {
        Self { input: Some(input), data: None }
    }
}

impl From<Option<Bytes>> for TransactionInput {
    fn from(input: Option<Bytes>) -> Self {
        Self { input, data: None }
    }
}

/// Error thrown when both `data` and `input` fields are set and not equal.
#[derive(Debug, Default, thiserror::Error)]
#[error("both \"data\" and \"input\" are set and not equal. Please use \"input\" to pass transaction call data")]
#[non_exhaustive]
pub struct TransactionInputError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_tx_request() {
        let s = r#"{"accessList":[],"data":"0x0902f1ac","to":"0xa478c2975ab1ea89e8196811f51a7b7ade33eb11","type":"0x02"}"#;
        let _req = serde_json::from_str::<TransactionRequest>(s).unwrap();
    }

    #[test]
    fn serde_unique_call_input() {
        let s = r#"{"accessList":[],"data":"0x0902f1ac", "input":"0x0902f1ac","to":"0xa478c2975ab1ea89e8196811f51a7b7ade33eb11","type":"0x02"}"#;
        let req = serde_json::from_str::<TransactionRequest>(s).unwrap();
        assert!(req.input.try_into_unique_input().unwrap().is_some());

        let s = r#"{"accessList":[],"data":"0x0902f1ac","to":"0xa478c2975ab1ea89e8196811f51a7b7ade33eb11","type":"0x02"}"#;
        let req = serde_json::from_str::<TransactionRequest>(s).unwrap();
        assert!(req.input.try_into_unique_input().unwrap().is_some());

        let s = r#"{"accessList":[],"input":"0x0902f1ac","to":"0xa478c2975ab1ea89e8196811f51a7b7ade33eb11","type":"0x02"}"#;
        let req = serde_json::from_str::<TransactionRequest>(s).unwrap();
        assert!(req.input.try_into_unique_input().unwrap().is_some());

        let s = r#"{"accessList":[],"data":"0x0902f1ac", "input":"0x0902f1","to":"0xa478c2975ab1ea89e8196811f51a7b7ade33eb11","type":"0x02"}"#;
        let req = serde_json::from_str::<TransactionRequest>(s).unwrap();
        assert!(req.input.try_into_unique_input().is_err());
    }
}
