//! RPC types for transactions

use crate::eth::other::OtherFields;
pub use access_list::{AccessList, AccessListItem, AccessListWithGasUsed};
use alloy_consensus::{
    SignableTransaction, Signed, TxEip1559, TxEip2930, TxEip4844, TxEip4844Variant, TxEnvelope,
    TxLegacy, TxType,
};
use alloy_primitives::{Address, Bytes, B256, U256, U8};
pub use blob::BlobTransactionSidecar;
pub use common::TransactionInfo;
pub use error::ConversionError;
pub use optimism::OptimismTransactionReceiptFields;
pub use receipt::TransactionReceipt;
pub use request::{TransactionInput, TransactionRequest};
use serde::{Deserialize, Serialize};
pub use signature::{Parity, Signature};

mod access_list;
mod blob;
mod common;
mod error;
pub mod kzg;
pub mod optimism;
mod receipt;
pub mod request;
mod signature;

/// Transaction object used in RPC
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    /// Hash
    pub hash: B256,
    /// Nonce
    #[serde(with = "alloy_serde::num::u64_hex")]
    pub nonce: u64,
    /// Block hash
    pub block_hash: Option<B256>,
    /// Block number
    pub block_number: Option<U256>,
    /// Transaction Index
    pub transaction_index: Option<U256>,
    /// Sender
    pub from: Address,
    /// Recipient
    pub to: Option<Address>,
    /// Transferred value
    pub value: U256,
    /// Gas Price
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_price: Option<U256>,
    /// Gas amount
    pub gas: U256,
    /// Max BaseFeePerGas the user is willing to pay.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fee_per_gas: Option<U256>,
    /// The miner's tip.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_priority_fee_per_gas: Option<U256>,
    /// Configured max fee per blob gas for eip-4844 transactions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fee_per_blob_gas: Option<U256>,
    /// Data
    pub input: Bytes,
    /// All _flattened_ fields of the transaction signature.
    ///
    /// Note: this is an option so special transaction types without a signature (e.g. <https://github.com/ethereum-optimism/optimism/blob/0bf643c4147b43cd6f25a759d331ef3a2a61a2a3/specs/deposits.md#the-deposited-transaction-type>) can be supported.
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub signature: Option<Signature>,
    /// The chain id of the transaction, if any.
    #[serde(with = "alloy_serde::u64_hex_opt")]
    pub chain_id: Option<u64>,
    /// Contains the blob hashes for eip-4844 transactions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blob_versioned_hashes: Vec<B256>,
    /// EIP2930
    ///
    /// Pre-pay to warm storage access.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_list: Option<AccessList>,
    /// EIP2718
    ///
    /// Transaction type, Some(2) for EIP-1559 transaction,
    /// Some(1) for AccessList transaction, None for Legacy
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub transaction_type: Option<U8>,

    /// Arbitrary extra fields.
    ///
    /// This captures fields that are not native to ethereum but included in ethereum adjacent networks, for example fields the [optimism `eth_getTransactionByHash` request](https://docs.alchemy.com/alchemy/apis/optimism/eth-gettransactionbyhash) returns additional fields that this type will capture
    #[serde(flatten)]
    pub other: OtherFields,
}

impl Transaction {
    /// Converts [Transaction] into [TransactionRequest].
    ///
    /// During this conversion data for [TransactionRequest::sidecar] is not populated as it is not
    /// part of [Transaction].
    pub fn into_request(self) -> TransactionRequest {
        TransactionRequest {
            from: Some(self.from),
            to: self.to,
            gas: Some(self.gas),
            gas_price: self.gas_price,
            value: Some(self.value),
            input: self.input.into(),
            nonce: Some(self.nonce),
            chain_id: self.chain_id,
            access_list: self.access_list,
            transaction_type: self.transaction_type,
            max_fee_per_gas: self.max_fee_per_gas,
            max_priority_fee_per_gas: self.max_priority_fee_per_gas,
            max_fee_per_blob_gas: self.max_fee_per_blob_gas,
            blob_versioned_hashes: Some(self.blob_versioned_hashes),
            sidecar: None,
            other: OtherFields::default(),
        }
    }
}

impl TryFrom<Transaction> for Signed<TxLegacy> {
    type Error = ConversionError;

    fn try_from(tx: Transaction) -> Result<Self, Self::Error> {
        let signature = tx.signature.ok_or(ConversionError::MissingSignature)?.try_into()?;

        let tx = TxLegacy {
            chain_id: tx.chain_id,
            nonce: tx.nonce,
            gas_price: tx.gas_price.ok_or(ConversionError::MissingGasPrice)?.to(),
            gas_limit: tx.gas.to(),
            to: tx.to.into(),
            value: tx.value,
            input: tx.input,
        };
        Ok(tx.into_signed(signature))
    }
}

impl TryFrom<Transaction> for Signed<TxEip1559> {
    type Error = ConversionError;

    fn try_from(tx: Transaction) -> Result<Self, Self::Error> {
        let signature = tx.signature.ok_or(ConversionError::MissingSignature)?.try_into()?;

        let tx = TxEip1559 {
            chain_id: tx.chain_id.ok_or(ConversionError::MissingChainId)?,
            nonce: tx.nonce,
            max_fee_per_gas: tx.max_fee_per_gas.ok_or(ConversionError::MissingMaxFeePerGas)?.to(),
            max_priority_fee_per_gas: tx
                .max_priority_fee_per_gas
                .ok_or(ConversionError::MissingMaxPriorityFeePerGas)?
                .to(),
            gas_limit: tx.gas.to(),
            to: tx.to.into(),
            value: tx.value,
            input: tx.input,
            access_list: tx.access_list.unwrap_or_default().into(),
        };
        Ok(tx.into_signed(signature))
    }
}

impl TryFrom<Transaction> for Signed<TxEip2930> {
    type Error = ConversionError;

    fn try_from(tx: Transaction) -> Result<Self, Self::Error> {
        let signature = tx.signature.ok_or(ConversionError::MissingSignature)?.try_into()?;

        let tx = TxEip2930 {
            chain_id: tx.chain_id.ok_or(ConversionError::MissingChainId)?,
            nonce: tx.nonce,
            gas_price: tx.gas_price.ok_or(ConversionError::MissingGasPrice)?.to(),
            gas_limit: tx.gas.to(),
            to: tx.to.into(),
            value: tx.value,
            input: tx.input,
            access_list: tx.access_list.ok_or(ConversionError::MissingAccessList)?.into(),
        };
        Ok(tx.into_signed(signature))
    }
}

impl TryFrom<Transaction> for Signed<TxEip4844> {
    type Error = ConversionError;

    fn try_from(tx: Transaction) -> Result<Self, Self::Error> {
        let signature = tx.signature.ok_or(ConversionError::MissingSignature)?.try_into()?;
        let tx = TxEip4844 {
            chain_id: tx.chain_id.ok_or(ConversionError::MissingChainId)?,
            nonce: tx.nonce,
            max_fee_per_gas: tx.max_fee_per_gas.ok_or(ConversionError::MissingMaxFeePerGas)?.to(),
            max_priority_fee_per_gas: tx
                .max_priority_fee_per_gas
                .ok_or(ConversionError::MissingMaxPriorityFeePerGas)?
                .to(),
            gas_limit: tx.gas.to(),
            to: tx.to.ok_or(ConversionError::MissingTo)?,
            value: tx.value,
            input: tx.input,
            access_list: tx.access_list.unwrap_or_default().into(),
            blob_versioned_hashes: tx.blob_versioned_hashes,
            max_fee_per_blob_gas: tx
                .max_fee_per_blob_gas
                .ok_or(ConversionError::MissingMaxFeePerBlobGas)?
                .to(),
        };
        Ok(tx.into_signed(signature))
    }
}

impl TryFrom<Transaction> for Signed<TxEip4844Variant> {
    type Error = ConversionError;

    fn try_from(tx: Transaction) -> Result<Self, Self::Error> {
        let tx: Signed<TxEip4844> = tx.try_into()?;
        let (inner, signature, _) = tx.into_parts();
        let tx = TxEip4844Variant::TxEip4844(inner);

        Ok(tx.into_signed(signature))
    }
}

impl TryFrom<Transaction> for TxEnvelope {
    type Error = ConversionError;

    fn try_from(tx: Transaction) -> Result<Self, Self::Error> {
        match tx.transaction_type.unwrap_or_default().to::<u8>().try_into()? {
            TxType::Legacy => Ok(Self::Legacy(tx.try_into()?)),
            TxType::Eip1559 => Ok(Self::Eip1559(tx.try_into()?)),
            TxType::Eip2930 => Ok(Self::Eip2930(tx.try_into()?)),
            TxType::Eip4844 => Ok(Self::Eip4844(tx.try_into()?)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_transaction() {
        let transaction = Transaction {
            hash: B256::with_last_byte(1),
            nonce: 2,
            block_hash: Some(B256::with_last_byte(3)),
            block_number: Some(U256::from(4)),
            transaction_index: Some(U256::from(5)),
            from: Address::with_last_byte(6),
            to: Some(Address::with_last_byte(7)),
            value: U256::from(8),
            gas_price: Some(U256::from(9)),
            gas: U256::from(10),
            input: Bytes::from(vec![11, 12, 13]),
            signature: Some(Signature {
                v: U256::from(14),
                r: U256::from(14),
                s: U256::from(14),
                y_parity: None,
            }),
            chain_id: Some(17),
            blob_versioned_hashes: vec![],
            access_list: None,
            transaction_type: Some(U8::from(20)),
            max_fee_per_gas: Some(U256::from(21)),
            max_priority_fee_per_gas: Some(U256::from(22)),
            max_fee_per_blob_gas: None,
            other: Default::default(),
        };
        let serialized = serde_json::to_string(&transaction).unwrap();
        assert_eq!(
            serialized,
            r#"{"hash":"0x0000000000000000000000000000000000000000000000000000000000000001","nonce":"0x2","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000003","blockNumber":"0x4","transactionIndex":"0x5","from":"0x0000000000000000000000000000000000000006","to":"0x0000000000000000000000000000000000000007","value":"0x8","gasPrice":"0x9","gas":"0xa","maxFeePerGas":"0x15","maxPriorityFeePerGas":"0x16","input":"0x0b0c0d","r":"0xe","s":"0xe","v":"0xe","chainId":"0x11","type":"0x14"}"#
        );
        let deserialized: Transaction = serde_json::from_str(&serialized).unwrap();
        assert_eq!(transaction, deserialized);
    }

    #[test]
    fn serde_transaction_with_parity_bit() {
        let transaction = Transaction {
            hash: B256::with_last_byte(1),
            nonce: 2,
            block_hash: Some(B256::with_last_byte(3)),
            block_number: Some(U256::from(4)),
            transaction_index: Some(U256::from(5)),
            from: Address::with_last_byte(6),
            to: Some(Address::with_last_byte(7)),
            value: U256::from(8),
            gas_price: Some(U256::from(9)),
            gas: U256::from(10),
            input: Bytes::from(vec![11, 12, 13]),
            signature: Some(Signature {
                v: U256::from(14),
                r: U256::from(14),
                s: U256::from(14),
                y_parity: Some(Parity(true)),
            }),
            chain_id: Some(17),
            blob_versioned_hashes: vec![],
            access_list: None,
            transaction_type: Some(U8::from(20)),
            max_fee_per_gas: Some(U256::from(21)),
            max_priority_fee_per_gas: Some(U256::from(22)),
            max_fee_per_blob_gas: None,
            other: Default::default(),
        };
        let serialized = serde_json::to_string(&transaction).unwrap();
        assert_eq!(
            serialized,
            r#"{"hash":"0x0000000000000000000000000000000000000000000000000000000000000001","nonce":"0x2","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000003","blockNumber":"0x4","transactionIndex":"0x5","from":"0x0000000000000000000000000000000000000006","to":"0x0000000000000000000000000000000000000007","value":"0x8","gasPrice":"0x9","gas":"0xa","maxFeePerGas":"0x15","maxPriorityFeePerGas":"0x16","input":"0x0b0c0d","r":"0xe","s":"0xe","v":"0xe","yParity":"0x1","chainId":"0x11","type":"0x14"}"#
        );
        let deserialized: Transaction = serde_json::from_str(&serialized).unwrap();
        assert_eq!(transaction, deserialized);
    }
}
