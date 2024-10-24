//! RPC types for transactions

use alloy_consensus::{
    SignableTransaction, Signed, TxEip1559, TxEip2930, TxEip4844, TxEip4844Variant, TxEip7702,
    TxEnvelope, TxLegacy, TxType,
};
use alloy_eips::eip7702::SignedAuthorization;
use alloy_network_primitives::TransactionResponse;
use alloy_primitives::{Address, BlockHash, Bytes, ChainId, TxHash, TxKind, B256, U256};

use alloc::vec::Vec;

pub use alloy_consensus::BlobTransactionSidecar;
pub use alloy_eips::{
    eip2930::{AccessList, AccessListItem, AccessListResult},
    eip7702::Authorization,
};

mod common;
pub use common::TransactionInfo;

mod error;
pub use error::ConversionError;

mod receipt;
pub use receipt::TransactionReceipt;

#[cfg(feature = "serde")]
pub use receipt::AnyTransactionReceipt;

pub mod request;
pub use request::{TransactionInput, TransactionRequest};

mod signature;
pub use signature::{Parity, Signature};

pub use alloy_consensus::{AnyReceiptEnvelope, Receipt, ReceiptEnvelope, ReceiptWithBloom};

/// Transaction object used in RPC
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[doc(alias = "Tx")]
pub struct Transaction {
    /// Hash
    pub hash: TxHash,
    /// Nonce
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub nonce: u64,
    /// Block hash
    #[cfg_attr(feature = "serde", serde(default))]
    pub block_hash: Option<BlockHash>,
    /// Block number
    #[cfg_attr(feature = "serde", serde(default, with = "alloy_serde::quantity::opt"))]
    pub block_number: Option<u64>,
    /// Transaction Index
    #[cfg_attr(feature = "serde", serde(default, with = "alloy_serde::quantity::opt"))]
    pub transaction_index: Option<u64>,
    /// Sender
    pub from: Address,
    /// Recipient
    pub to: Option<Address>,
    /// Transferred value
    pub value: U256,
    /// Gas Price
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "alloy_serde::quantity::opt"
        )
    )]
    pub gas_price: Option<u128>,
    /// Gas amount
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub gas: u64,
    /// Max BaseFeePerGas the user is willing to pay.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "alloy_serde::quantity::opt"
        )
    )]
    pub max_fee_per_gas: Option<u128>,
    /// The miner's tip.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "alloy_serde::quantity::opt"
        )
    )]
    pub max_priority_fee_per_gas: Option<u128>,
    /// Configured max fee per blob gas for eip-4844 transactions
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "alloy_serde::quantity::opt"
        )
    )]
    pub max_fee_per_blob_gas: Option<u128>,
    /// Data
    pub input: Bytes,
    /// All _flattened_ fields of the transaction signature.
    ///
    /// Note: this is an option so special transaction types without a signature (e.g. <https://github.com/ethereum-optimism/optimism/blob/0bf643c4147b43cd6f25a759d331ef3a2a61a2a3/specs/deposits.md#the-deposited-transaction-type>) can be supported.
    #[cfg_attr(feature = "serde", serde(flatten, skip_serializing_if = "Option::is_none"))]
    pub signature: Option<Signature>,
    /// The chain id of the transaction, if any.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "alloy_serde::quantity::opt"
        )
    )]
    pub chain_id: Option<ChainId>,
    /// Contains the blob hashes for eip-4844 transactions.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub blob_versioned_hashes: Option<Vec<B256>>,
    /// EIP2930
    ///
    /// Pre-pay to warm storage access.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub access_list: Option<AccessList>,
    /// EIP2718
    ///
    /// Transaction type,
    /// Some(4) for EIP-7702 transaction, Some(3) for EIP-4844 transaction, Some(2) for EIP-1559
    /// transaction, Some(1) for AccessList transaction, None or Some(0) for Legacy
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            rename = "type",
            skip_serializing_if = "Option::is_none",
            with = "alloy_serde::quantity::opt"
        )
    )]
    #[doc(alias = "tx_type")]
    pub transaction_type: Option<u8>,
    /// The signed authorization list is a list of tuples that store the address to code which the
    /// signer desires to execute in the context of their EOA and their signature.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub authorization_list: Option<Vec<SignedAuthorization>>,
}

impl Transaction {
    /// Returns true if the transaction is a legacy or 2930 transaction.
    pub const fn is_legacy_gas(&self) -> bool {
        self.gas_price.is_none()
    }

    /// Converts [Transaction] into [TransactionRequest].
    ///
    /// During this conversion data for [TransactionRequest::sidecar] is not populated as it is not
    /// part of [Transaction].
    pub fn into_request(self) -> TransactionRequest {
        let gas_price = match (self.gas_price, self.max_fee_per_gas) {
            (Some(gas_price), None) => Some(gas_price),
            // EIP-1559 transactions include deprecated `gasPrice` field displaying gas used by
            // transaction.
            // Setting this field for resulted tx request will result in it being invalid
            (_, Some(_)) => None,
            // unreachable
            (None, None) => None,
        };

        let to = self.to.map(TxKind::Call);

        TransactionRequest {
            from: Some(self.from),
            to,
            gas: Some(self.gas),
            gas_price,
            value: Some(self.value),
            input: self.input.into(),
            nonce: Some(self.nonce),
            chain_id: self.chain_id,
            access_list: self.access_list,
            transaction_type: self.transaction_type,
            max_fee_per_gas: self.max_fee_per_gas,
            max_priority_fee_per_gas: self.max_priority_fee_per_gas,
            max_fee_per_blob_gas: self.max_fee_per_blob_gas,
            blob_versioned_hashes: self.blob_versioned_hashes,
            sidecar: None,
            authorization_list: self.authorization_list,
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
            gas_price: tx.gas_price.ok_or(ConversionError::MissingGasPrice)?,
            gas_limit: tx.gas,
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
            max_fee_per_gas: tx.max_fee_per_gas.ok_or(ConversionError::MissingMaxFeePerGas)?,
            max_priority_fee_per_gas: tx
                .max_priority_fee_per_gas
                .ok_or(ConversionError::MissingMaxPriorityFeePerGas)?,
            gas_limit: tx.gas,
            to: tx.to.into(),
            value: tx.value,
            input: tx.input,
            access_list: tx.access_list.unwrap_or_default(),
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
            gas_price: tx.gas_price.ok_or(ConversionError::MissingGasPrice)?,
            gas_limit: tx.gas,
            to: tx.to.into(),
            value: tx.value,
            input: tx.input,
            access_list: tx.access_list.ok_or(ConversionError::MissingAccessList)?,
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
            max_fee_per_gas: tx.max_fee_per_gas.ok_or(ConversionError::MissingMaxFeePerGas)?,
            max_priority_fee_per_gas: tx
                .max_priority_fee_per_gas
                .ok_or(ConversionError::MissingMaxPriorityFeePerGas)?,
            gas_limit: tx.gas,
            to: tx.to.ok_or(ConversionError::MissingTo)?,
            value: tx.value,
            input: tx.input,
            access_list: tx.access_list.unwrap_or_default(),
            blob_versioned_hashes: tx
                .blob_versioned_hashes
                .ok_or(ConversionError::MissingBlobVersionedHashes)?,
            max_fee_per_blob_gas: tx
                .max_fee_per_blob_gas
                .ok_or(ConversionError::MissingMaxFeePerBlobGas)?,
        };
        Ok(tx.into_signed(signature))
    }
}

impl TryFrom<Transaction> for Signed<TxEip4844Variant> {
    type Error = ConversionError;

    fn try_from(tx: Transaction) -> Result<Self, Self::Error> {
        let tx: Signed<TxEip4844> = tx.try_into()?;
        let (inner, signature, _) = tx.into_parts();
        let tx: TxEip4844Variant = inner.into();

        Ok(tx.into_signed(signature))
    }
}

impl TryFrom<Transaction> for Signed<TxEip7702> {
    type Error = ConversionError;

    fn try_from(tx: Transaction) -> Result<Self, Self::Error> {
        let signature = tx.signature.ok_or(ConversionError::MissingSignature)?.try_into()?;
        let tx = TxEip7702 {
            chain_id: tx.chain_id.ok_or(ConversionError::MissingChainId)?,
            nonce: tx.nonce,
            gas_limit: tx.gas,
            max_fee_per_gas: tx.max_fee_per_gas.ok_or(ConversionError::MissingMaxFeePerGas)?,
            max_priority_fee_per_gas: tx
                .max_priority_fee_per_gas
                .ok_or(ConversionError::MissingMaxPriorityFeePerGas)?,
            to: tx.to.ok_or(ConversionError::MissingTo)?,
            value: tx.value,
            access_list: tx.access_list.ok_or(ConversionError::MissingAccessList)?,
            authorization_list: tx
                .authorization_list
                .ok_or(ConversionError::MissingAuthorizationList)?,
            input: tx.input,
        };
        Ok(tx.into_signed(signature))
    }
}

impl TryFrom<Transaction> for TxEnvelope {
    type Error = ConversionError;

    fn try_from(tx: Transaction) -> Result<Self, Self::Error> {
        match tx.transaction_type.unwrap_or_default().try_into()? {
            TxType::Legacy => Ok(Self::Legacy(tx.try_into()?)),
            TxType::Eip1559 => Ok(Self::Eip1559(tx.try_into()?)),
            TxType::Eip2930 => Ok(Self::Eip2930(tx.try_into()?)),
            TxType::Eip4844 => Ok(Self::Eip4844(tx.try_into()?)),
            TxType::Eip7702 => Ok(Self::Eip7702(tx.try_into()?)),
        }
    }
}

impl alloy_consensus::Transaction for Transaction {
    fn chain_id(&self) -> Option<ChainId> {
        self.chain_id
    }

    fn nonce(&self) -> u64 {
        self.nonce
    }

    fn gas_limit(&self) -> u64 {
        self.gas
    }

    fn gas_price(&self) -> Option<u128> {
        self.gas_price
    }

    fn max_fee_per_gas(&self) -> u128 {
        self.max_fee_per_gas.unwrap_or_else(|| self.gas_price.unwrap_or_default())
    }

    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        self.max_priority_fee_per_gas
    }

    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        self.max_fee_per_blob_gas
    }

    fn priority_fee_or_price(&self) -> u128 {
        debug_assert!(
            self.max_fee_per_gas.is_some() || self.gas_price.is_some(),
            "mutually exclusive fields"
        );
        self.max_fee_per_gas.unwrap_or_else(|| self.gas_price.unwrap_or_default())
    }

    fn kind(&self) -> TxKind {
        self.to.into()
    }

    fn value(&self) -> U256 {
        self.value
    }

    fn input(&self) -> &Bytes {
        &self.input
    }

    fn ty(&self) -> u8 {
        self.transaction_type.unwrap_or_default()
    }

    fn access_list(&self) -> Option<&AccessList> {
        self.access_list.as_ref()
    }

    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        self.blob_versioned_hashes.as_deref()
    }

    fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
        self.authorization_list.as_deref()
    }
}

impl TransactionResponse for Transaction {
    fn tx_hash(&self) -> B256 {
        self.hash
    }

    fn block_hash(&self) -> Option<BlockHash> {
        self.block_hash
    }

    fn block_number(&self) -> Option<u64> {
        self.block_number
    }

    fn transaction_index(&self) -> Option<u64> {
        self.transaction_index
    }

    fn from(&self) -> Address {
        self.from
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::Signature as AlloySignature;
    use arbitrary::Arbitrary;
    use core::str::FromStr;
    use rand::Rng;
    use similar_asserts::assert_eq;

    #[test]
    fn arbitrary_transaction() {
        let mut bytes = [0u8; 1024];
        rand::thread_rng().fill(bytes.as_mut_slice());
        let _: Transaction =
            Transaction::arbitrary(&mut arbitrary::Unstructured::new(&bytes)).unwrap();
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_transaction() {
        let transaction = Transaction {
            hash: B256::with_last_byte(1),
            nonce: 2,
            block_hash: Some(B256::with_last_byte(3)),
            block_number: Some(4),
            transaction_index: Some(5),
            from: Address::with_last_byte(6),
            to: Some(Address::with_last_byte(7)),
            value: U256::from(8),
            gas_price: Some(9),
            gas: 10,
            input: vec![11, 12, 13].into(),
            signature: Some(Signature {
                v: U256::from(14),
                r: U256::from(14),
                s: U256::from(14),
                y_parity: None,
            }),
            chain_id: Some(17),
            blob_versioned_hashes: None,
            access_list: None,
            transaction_type: Some(20),
            max_fee_per_gas: Some(21),
            max_priority_fee_per_gas: Some(22),
            max_fee_per_blob_gas: None,
            authorization_list: Some(vec![(Authorization {
                chain_id: 1,
                address: Address::left_padding_from(&[6]),
                nonce: 1u64,
            })
            .into_signed(AlloySignature::from_str("48b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353efffd310ac743f371de3b9f7f9cb56c0b28ad43601b4ab949f53faa07bd2c8041b").unwrap())]),
        };
        let serialized = serde_json::to_string(&transaction).unwrap();
        assert_eq!(
            serialized,
            r#"{"hash":"0x0000000000000000000000000000000000000000000000000000000000000001","nonce":"0x2","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000003","blockNumber":"0x4","transactionIndex":"0x5","from":"0x0000000000000000000000000000000000000006","to":"0x0000000000000000000000000000000000000007","value":"0x8","gasPrice":"0x9","gas":"0xa","maxFeePerGas":"0x15","maxPriorityFeePerGas":"0x16","input":"0x0b0c0d","r":"0xe","s":"0xe","v":"0xe","chainId":"0x11","type":"0x14","authorizationList":[{"chainId":"0x1","address":"0x0000000000000000000000000000000000000006","nonce":"0x1","yParity":"0x0","r":"0x48b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353","s":"0xefffd310ac743f371de3b9f7f9cb56c0b28ad43601b4ab949f53faa07bd2c804"}]}"#
        );
        let deserialized: Transaction = serde_json::from_str(&serialized).unwrap();
        assert_eq!(transaction, deserialized);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_transaction_with_parity_bit() {
        let transaction = Transaction {
            hash: B256::with_last_byte(1),
            nonce: 2,
            block_hash: Some(B256::with_last_byte(3)),
            block_number: Some(4),
            transaction_index: Some(5),
            from: Address::with_last_byte(6),
            to: Some(Address::with_last_byte(7)),
            value: U256::from(8),
            gas_price: Some(9),
            gas: 10,
            input: vec![11, 12, 13].into(),
            signature: Some(Signature {
                v: U256::from(14),
                r: U256::from(14),
                s: U256::from(14),
                y_parity: Some(Parity(true)),
            }),
            chain_id: Some(17),
            blob_versioned_hashes: None,
            access_list: None,
            transaction_type: Some(20),
            max_fee_per_gas: Some(21),
            max_priority_fee_per_gas: Some(22),
            max_fee_per_blob_gas: None,
            authorization_list: Some(vec![(Authorization {
                chain_id: 1,
                address: Address::left_padding_from(&[6]),
                nonce: 1u64,
            })
            .into_signed(AlloySignature::from_str("48b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353efffd310ac743f371de3b9f7f9cb56c0b28ad43601b4ab949f53faa07bd2c8041b").unwrap())]),
        };
        let serialized = serde_json::to_string(&transaction).unwrap();
        assert_eq!(
            serialized,
            r#"{"hash":"0x0000000000000000000000000000000000000000000000000000000000000001","nonce":"0x2","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000003","blockNumber":"0x4","transactionIndex":"0x5","from":"0x0000000000000000000000000000000000000006","to":"0x0000000000000000000000000000000000000007","value":"0x8","gasPrice":"0x9","gas":"0xa","maxFeePerGas":"0x15","maxPriorityFeePerGas":"0x16","input":"0x0b0c0d","r":"0xe","s":"0xe","v":"0xe","yParity":"0x1","chainId":"0x11","type":"0x14","authorizationList":[{"chainId":"0x1","address":"0x0000000000000000000000000000000000000006","nonce":"0x1","yParity":"0x0","r":"0x48b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353","s":"0xefffd310ac743f371de3b9f7f9cb56c0b28ad43601b4ab949f53faa07bd2c804"}]}"#
        );
        let deserialized: Transaction = serde_json::from_str(&serialized).unwrap();
        assert_eq!(transaction, deserialized);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_minimal_transaction() {
        let transaction = Transaction {
            hash: B256::with_last_byte(1),
            nonce: 2,
            from: Address::with_last_byte(6),
            value: U256::from(8),
            gas: 10,
            input: vec![11, 12, 13].into(),
            ..Default::default()
        };
        let serialized = serde_json::to_string(&transaction).unwrap();
        assert_eq!(
            serialized,
            r#"{"hash":"0x0000000000000000000000000000000000000000000000000000000000000001","nonce":"0x2","blockHash":null,"blockNumber":null,"transactionIndex":null,"from":"0x0000000000000000000000000000000000000006","to":null,"value":"0x8","gas":"0xa","input":"0x0b0c0d"}"#
        );
        let deserialized: Transaction = serde_json::from_str(&serialized).unwrap();
        assert_eq!(transaction, deserialized);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn into_request_legacy() {
        // cast rpc eth_getTransactionByHash
        // 0xe9e91f1ee4b56c0df2e9f06c2b8c27c6076195a88a7b8537ba8313d80e6f124e --rpc-url mainnet
        let rpc_tx = r#"{"blockHash":"0x8e38b4dbf6b11fcc3b9dee84fb7986e29ca0a02cecd8977c161ff7333329681e","blockNumber":"0xf4240","hash":"0xe9e91f1ee4b56c0df2e9f06c2b8c27c6076195a88a7b8537ba8313d80e6f124e","transactionIndex":"0x1","type":"0x0","nonce":"0x43eb","input":"0x","r":"0x3b08715b4403c792b8c7567edea634088bedcd7f60d9352b1f16c69830f3afd5","s":"0x10b9afb67d2ec8b956f0e1dbc07eb79152904f3a7bf789fc869db56320adfe09","chainId":"0x0","v":"0x1c","gas":"0xc350","from":"0x32be343b94f860124dc4fee278fdcbd38c102d88","to":"0xdf190dc7190dfba737d7777a163445b7fff16133","value":"0x6113a84987be800","gasPrice":"0xdf8475800"}"#;

        let tx = serde_json::from_str::<Transaction>(rpc_tx).unwrap();
        let request = tx.into_request();
        assert!(request.gas_price.is_some());
        assert!(request.max_fee_per_gas.is_none());
    }

    #[test]
    #[cfg(feature = "serde")]
    fn into_request_eip1559() {
        // cast rpc eth_getTransactionByHash
        // 0x0e07d8b53ed3d91314c80e53cf25bcde02084939395845cbb625b029d568135c --rpc-url mainnet
        let rpc_tx = r#"{"blockHash":"0x883f974b17ca7b28cb970798d1c80f4d4bb427473dc6d39b2a7fe24edc02902d","blockNumber":"0xe26e6d","hash":"0x0e07d8b53ed3d91314c80e53cf25bcde02084939395845cbb625b029d568135c","accessList":[],"transactionIndex":"0xad","type":"0x2","nonce":"0x16d","input":"0x5ae401dc00000000000000000000000000000000000000000000000000000000628ced5b000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000016000000000000000000000000000000000000000000000000000000000000000e442712a6700000000000000000000000000000000000000000000b3ff1489674e11c40000000000000000000000000000000000000000000000000000004a6ed55bbcc18000000000000000000000000000000000000000000000000000000000000000800000000000000000000000003cf412d970474804623bb4e3a42de13f9bca54360000000000000000000000000000000000000000000000000000000000000002000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc20000000000000000000000003a75941763f31c930b19c041b709742b0b31ebb600000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000412210e8a00000000000000000000000000000000000000000000000000000000","r":"0x7f2153019a74025d83a73effdd91503ceecefac7e35dd933adc1901c875539aa","s":"0x334ab2f714796d13c825fddf12aad01438db3a8152b2fe3ef7827707c25ecab3","chainId":"0x1","v":"0x0","gas":"0x46a02","maxPriorityFeePerGas":"0x59682f00","from":"0x3cf412d970474804623bb4e3a42de13f9bca5436","to":"0x68b3465833fb72a70ecdf485e0e4c7bd8665fc45","maxFeePerGas":"0x7fc1a20a8","value":"0x4a6ed55bbcc180","gasPrice":"0x50101df3a"}"#;

        let tx = serde_json::from_str::<Transaction>(rpc_tx).unwrap();
        let request = tx.into_request();
        assert!(request.gas_price.is_none());
        assert!(request.max_fee_per_gas.is_some());
    }
}
