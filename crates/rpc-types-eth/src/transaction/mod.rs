//! RPC types for transactions
use core::{
    str::{self, FromStr},
    u128,
};

use alloc::vec::Vec;
use alloy_consensus::{
    SignableTransaction, Signed, TxEip1559, TxEip2930, TxEip4844, TxEip4844Variant, TxEip7702,
    TxEnvelope, TxLegacy, TxType,
};
use alloy_eips::eip7702::SignedAuthorization;
use alloy_network_primitives::TransactionResponse;
use alloy_primitives::{
    address, b256, Address, BlockHash, Bytes, ChainId, Parity as yParity, Signature as ySignature,
    TxHash, TxKind, B256, U256,
};

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
use serde::{
    de::{MapAccess, Visitor},
    ser::SerializeStruct,
    Deserialize, Deserializer,
};
pub use signature::{Parity, Signature};

pub use alloy_consensus::{AnyReceiptEnvelope, Receipt, ReceiptEnvelope, ReceiptWithBloom};

/// Transaction object used in RPC
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[doc(alias = "Tx")]
pub struct Transaction {
    /// Hash
    pub hash: TxHash,
    /// Nonce
    pub nonce: u64,
    /// Block hash
    pub block_hash: Option<BlockHash>,
    /// Block number
    pub block_number: Option<u64>,
    /// Transaction Index
    pub transaction_index: Option<u64>,
    /// Sender
    pub from: Address,
    /// Recipient
    pub to: Option<Address>,
    /// Transferred value
    pub value: U256,
    /// Gas Price
    pub gas_price: Option<u128>,
    /// Gas amount
    pub gas: u64,
    /// Max BaseFeePerGas the user is willing to pay.
    pub max_fee_per_gas: Option<u128>,
    /// The miner's tip.
    pub max_priority_fee_per_gas: Option<u128>,
    /// Configured max fee per blob gas for eip-4844 transactions
    pub max_fee_per_blob_gas: Option<u128>,
    /// Data
    pub input: Bytes,
    /// All _flattened_ fields of the transaction signature.
    ///
    /// Note: this is an option so special transaction types without a signature (e.g. <https://github.com/ethereum-optimism/optimism/blob/0bf643c4147b43cd6f25a759d331ef3a2a61a2a3/specs/deposits.md#the-deposited-transaction-type>) can be supported.
    pub signature: Option<Signature>,
    /// The chain id of the transaction, if any.
    pub chain_id: Option<ChainId>,
    /// Contains the blob hashes for eip-4844 transactions.
    pub blob_versioned_hashes: Option<Vec<B256>>,
    /// EIP2930
    ///
    /// Pre-pay to warm storage access.
    pub access_list: Option<AccessList>,
    /// EIP2718
    ///
    /// Transaction type,
    /// Some(4) for EIP-7702 transaction, Some(3) for EIP-4844 transaction, Some(2) for EIP-1559
    /// transaction, Some(1) for AccessList transaction, None or Some(0) for Legacy
    #[doc(alias = "tx_type")]
    pub transaction_type: Option<u8>,
    /// The signed authorization list is a list of tuples that store the address to code which the
    /// signer desires to execute in the context of their EOA and their signature.
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

impl serde::Serialize for Transaction {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = ser.serialize_struct("Transaction", std::mem::size_of::<Transaction>())?;

        let tx_type = self.transaction_type.as_ref().unwrap_or(&0);
        let hash = &self.hash;
        let nonce = &self.nonce;
        let block_hash = &self.block_hash.unwrap_or_default();
        let block_number = &self.block_number.unwrap_or_default();
        let tx_id = &self.transaction_index.unwrap_or_default();
        let chain_id = &self.chain_id.unwrap_or(1);
        let gas = &self.gas;
        let to = &self.to.unwrap_or_default();
        let from = &self.from;
        let value = &self.value;
        let input = &self.input;
        let sig = &self.signature.as_ref();
        let access_list = &self.access_list.as_ref();
        let max_fee_per_gas = &self.max_fee_per_gas.unwrap_or_default();
        let max_priority_fee_per_gas = &self.max_priority_fee_per_gas.unwrap_or_default();
        let max_fee_per_blob_gas = &self.max_fee_per_blob_gas.unwrap_or_default();
        let blob_version_hashes = &self.blob_versioned_hashes.as_ref();
        let auth_list = &self.authorization_list.as_ref();

        state.serialize_field("hash", hash)?;
        state.serialize_field("nonce", &format!("{nonce:#x}"))?;
        state.serialize_field("blockHash", block_hash)?;
        state.serialize_field("blockNumber", &format!("{block_number:#x}"))?;
        state.serialize_field("transactionIndex", &format!("{tx_id:#x}"))?;
        state.serialize_field("from", from)?;
        state.serialize_field("to", to)?;
        state.serialize_field("value", value)?;
        if *tx_type < 2 {
            if self.gas_price.is_some() {
                let price = &self.gas_price.unwrap();
                state.serialize_field("gasPrice", &format!("{price:#x}"))?;
            }
        }
        state.serialize_field("gasLimit", &format!("{gas:#x}"))?;
        if *tx_type >= 2 {
            state.serialize_field("maxFeePerGas", &format!("{max_fee_per_gas:#x}"))?;
            state.serialize_field(
                "maxPriorityFeePerGas",
                &format!("{max_priority_fee_per_gas:#x}"),
            )?;
        }
        if *tx_type == 3 {
            state.serialize_field("blobVersionedHashes", blob_version_hashes.unwrap())?;
            state.serialize_field("maxFeePerBlobGas", &format!("{max_fee_per_blob_gas:#x}"))?;
        }
        state.serialize_field("input", input)?;

        if sig.is_some() {
            let Signature { r, s, v, y_parity } = sig.unwrap();
            state.serialize_field("r", &r)?;
            state.serialize_field("s", &s)?;
            state.serialize_field("v", &v)?;
            if *tx_type > 0 && y_parity.is_some() {
                state.serialize_field("yParity", &y_parity.unwrap())?;
            }
        }

        state.serialize_field("chainId", &format!("{chain_id:#x}"))?;

        if *tx_type >= 1 && access_list.is_some() {
            state.serialize_field("accessList", &access_list)?;
        }
        state.serialize_field("transactionType", &format!("{tx_type:#x}"))?;
        if *tx_type == 4 && auth_list.is_some() {
            state.serialize_field("authorizationList", &auth_list.unwrap())?;
        }

        state.end()
    }
}

impl<'de> Deserialize<'de> for Transaction {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TxVisitor;

        const FIELDS: &'static [&'static str] = &[
            "hash",
            "nonce",
            "block_hash",
            "block_number",
            "transaction_index",
            "from",
            "to",
            "value",
            "gas_price",
            "gas",
            "max_fee_per_gas",
            "max_priority_fee_per_gas",
            "max_fee_per_blob_gas",
            "input",
            "r",
            "s",
            "v",
            "yParity",
            "chain_id",
            "blob_versioned_hashes",
            "access_list",
            "transaction_type",
            "authorization_list",
        ];

        impl<'de> Visitor<'de> for TxVisitor {
            type Value = Transaction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct Transaction")
            }

            fn visit_map<V: MapAccess<'de>>(self, mut map: V) -> Result<Transaction, V::Error> {
                let mut hash =
                    b256!("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421");
                let mut nonce = u64::MIN;
                let mut block_hash: Option<BlockHash> = Option::None;
                let mut block_number: Option<u64> = Option::None;
                let mut transaction_index: Option<u64> = Option::None;
                let mut from: Address = address!("d8da6bf26964af9d7eed9e03e53415d37aa96045");
                let mut to: Option<Address> = Option::None;
                let mut value: U256 = U256::ZERO;
                let mut gas_price: Option<u128> = Option::None;
                let mut gas: u64 = u64::MIN;
                let mut max_fee_per_gas: Option<u128> = Option::None;
                let mut max_priority_fee_per_gas: Option<u128> = Option::None;
                let mut max_fee_per_blob_gas: Option<u128> = Option::None;
                let mut input: Bytes = Bytes::new();
                let mut r: U256 = U256::MIN;
                let mut s: U256 = U256::MIN;
                let mut v: U256 = U256::MIN;
                let mut y_parity: Option<Parity> = Option::None;
                let mut chain_id: Option<ChainId> = Option::None;
                let mut blob_versioned_hashes: Option<Vec<B256>> = Option::None;
                let mut access_list: Option<AccessList> = Option::None;
                let mut transaction_type: Option<u8> = Option::None;
                let mut authorization_list: Option<Vec<SignedAuthorization>> = Option::None;

                while let Some(key) = map.next_key::<&str>()? {
                    match key {
                        "hash" => {
                            let hash_str = map.next_value::<&str>()?;
                            hash = TxHash::from_str(hash_str).unwrap_or_default();
                        }
                        "nonce" => {
                            let n = map.next_value::<&str>()?;
                            nonce = u64::from_str_radix(&n[2..], 16).unwrap_or_default();
                        }
                        "blockHash" => {
                            let b = map.next_value::<&str>()?;
                            block_hash = Option::Some(BlockHash::from_str(b).unwrap_or_default());
                        }
                        "blockNumber" => {
                            let b = map.next_value::<&str>()?;
                            block_number =
                                Option::Some(u64::from_str_radix(&b[2..], 16).unwrap_or_default());
                        }
                        "transactionIndex" => {
                            let idx = map.next_value::<&str>()?;
                            transaction_index = Option::Some(
                                u64::from_str_radix(&idx[2..], 16).unwrap_or_default(),
                            );
                        }
                        "from" => {
                            let f = map.next_value::<&str>()?;
                            from = Address::from_str(f).unwrap_or_default();
                        }
                        "to" => {
                            let t = map.next_value::<&str>()?;
                            to = Option::Some(Address::from_str(t).unwrap_or_default());
                        }
                        "value" => {
                            let v = map.next_value::<&str>()?;
                            value = U256::from_str(v).unwrap_or_default();
                        }
                        "gasPrice" => {
                            let g = map.next_value::<&str>()?;
                            gas_price = u128::from_str_radix(&g[2..], 16).ok();
                        }
                        "gasLimit" => {
                            let g = map.next_value::<&str>()?;
                            gas = u64::from_str_radix(&g[2..], 16).unwrap();
                        }
                        "maxFeePerGas" => {
                            let m = map.next_value::<&str>()?;
                            max_fee_per_gas = u128::from_str_radix(&m[2..], 16).ok();
                        }
                        "maxPriorityFeePerGas" => {
                            let m = map.next_value::<&str>()?;
                            max_priority_fee_per_gas = u128::from_str_radix(&m[2..], 16).ok();
                        }
                        "maxFeePerBlobGas" => {
                            let m = map.next_value::<&str>()?;
                            max_fee_per_blob_gas = u128::from_str_radix(&m[2..], 16).ok();
                        }
                        "input" => {
                            let i = map.next_value::<&str>()?;
                            input = Bytes::from_str(i).unwrap_or(Bytes::new());
                        }
                        "r" => {
                            let st = map.next_value::<&str>()?;
                            r = U256::from_str_radix(&st[2..], 16).unwrap_or_default();
                        }
                        "s" => {
                            let st = map.next_value::<&str>()?;
                            s = U256::from_str_radix(&st[2..], 16).unwrap_or_default();
                        }
                        "v" => {
                            let st = map.next_value::<&str>()?;
                            v = U256::from_str_radix(&st[2..], 16).unwrap_or_default();
                        }
                        "yParity" => {
                            let st = map.next_value::<&str>()?;
                            let num = u8::from_str_radix(&st[2..], 16).unwrap();
                            let b = num != 0;
                            y_parity = Option::Some(Parity(b));
                        }
                        "chainId" => {
                            let st = String::from(map.next_value::<&str>()?);
                            chain_id = ChainId::from_str_radix(&st[2..], 16).ok();
                        }
                        "blobVersionedHashes" => {
                            blob_versioned_hashes = Option::Some(map.next_value()?);
                        }
                        "accessList" => {
                            access_list = Option::Some(map.next_value()?);
                        }
                        "transactionType" => {
                            let t = map.next_value::<&str>()?;
                            transaction_type = u8::from_str_radix(&t[2..], 16).ok();
                        }
                        "authorizationList" => {
                            authorization_list = Option::Some(map.next_value()?);
                        }
                        _ => {
                            continue;
                        }
                    }
                }
                Ok(Transaction {
                    hash,
                    nonce,
                    block_hash,
                    block_number,
                    transaction_index,
                    from,
                    to,
                    value,
                    gas_price,
                    gas,
                    max_fee_per_gas,
                    max_priority_fee_per_gas,
                    max_fee_per_blob_gas,
                    input,
                    signature: Option::Some(Signature { r, s, v, y_parity }),
                    chain_id,
                    blob_versioned_hashes,
                    access_list,
                    transaction_type,
                    authorization_list,
                })
            }
        }
        deserializer.deserialize_struct("Transaction", FIELDS, TxVisitor)
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
    use alloy_primitives::{address, b256, Signature as AlloySignature, B256};
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
    fn serialize_all_tx_types() {
        let blob_1 = b256!("bcd76a82d8a6c5f7a9ffde451c3e916f5d90eab90b8ab8f7e6fdd4e48f4c7cb9");
        let blob_2 = b256!("d5a9c3f147b4b81d263f4c4f93e56e9c13b08434f9f4b5a3e8dc2e3b7d2f8a29");
        let blob_3 = b256!("a3e9fb72f7e51b5fa9c4e35f9de9cddc9a0db82c9e4d47a4f8db3ea47f8b6c12");
        let address = address!("de0b295669a9fd93d5f28d9ec85e40f4cb697bae");
        let key_1 = b256!("0000000000000000000000000000000000000000000000000000000000000003");
        let key_2 = b256!("0000000000000000000000000000000000000000000000000000000000000007");
        let storage_keys = vec![key_1, key_2];

        let mut tx = Transaction {
            hash: B256::with_last_byte(1),
            nonce: 2,
            block_hash: Some(B256::with_last_byte(3)),
            block_number: Some(4),
            transaction_index: Some(5),
            from: Address::with_last_byte(6),
            to: Some(Address::with_last_byte(7)),
            value: U256::from(59612),
            gas_price: Some(9),
            gas: 10,
            input: vec![11, 12, 13].into(),
            signature: Some(Signature {
                v: U256::from(14),
                r: U256::from(14),
                s: U256::from(14),
                y_parity: Option::Some(Parity(true)),
            }),
            chain_id: Some(8),
            blob_versioned_hashes: Option::Some(vec![
                blob_1, blob_2, blob_3
              ]),
            access_list: Option::Some(AccessList(vec![AccessListItem {
                address, storage_keys
            }])),
            transaction_type: Option::None,
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

        let serialized = serde_json::to_string(&tx).unwrap();
        let expected = r#"{"hash":"0x0000000000000000000000000000000000000000000000000000000000000001","nonce":"0x2","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000003","blockNumber":"0x4","transactionIndex":"0x5","from":"0x0000000000000000000000000000000000000006","to":"0x0000000000000000000000000000000000000007","value":"0xe8dc","gasPrice":"0x9","gasLimit":"0xa","input":"0x0b0c0d","r":"0xe","s":"0xe","v":"0xe","chainId":"0x8","transactionType":"0x0"}"#;
        assert_eq!(serialized, expected);

        tx.transaction_type = Option::Some(0);
        let serialized = serde_json::to_string(&tx).unwrap();
        let expected = r#"{"hash":"0x0000000000000000000000000000000000000000000000000000000000000001","nonce":"0x2","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000003","blockNumber":"0x4","transactionIndex":"0x5","from":"0x0000000000000000000000000000000000000006","to":"0x0000000000000000000000000000000000000007","value":"0xe8dc","gasPrice":"0x9","gasLimit":"0xa","input":"0x0b0c0d","r":"0xe","s":"0xe","v":"0xe","chainId":"0x8","transactionType":"0x0"}"#;
        assert_eq!(serialized, expected);

        tx.transaction_type = Option::Some(1);
        let serialized = serde_json::to_string(&tx).unwrap();
        let expected = r#"{"hash":"0x0000000000000000000000000000000000000000000000000000000000000001","nonce":"0x2","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000003","blockNumber":"0x4","transactionIndex":"0x5","from":"0x0000000000000000000000000000000000000006","to":"0x0000000000000000000000000000000000000007","value":"0xe8dc","gasPrice":"0x9","gasLimit":"0xa","input":"0x0b0c0d","r":"0xe","s":"0xe","v":"0xe","yParity":"0x1","chainId":"0x8","accessList":[{"address":"0xde0b295669a9fd93d5f28d9ec85e40f4cb697bae","storageKeys":["0x0000000000000000000000000000000000000000000000000000000000000003","0x0000000000000000000000000000000000000000000000000000000000000007"]}],"transactionType":"0x1"}"#;
        assert_eq!(serialized, expected);

        tx.transaction_type = Option::Some(2);
        let serialized = serde_json::to_string(&tx).unwrap();
        let expected = r#"{"hash":"0x0000000000000000000000000000000000000000000000000000000000000001","nonce":"0x2","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000003","blockNumber":"0x4","transactionIndex":"0x5","from":"0x0000000000000000000000000000000000000006","to":"0x0000000000000000000000000000000000000007","value":"0xe8dc","gasLimit":"0xa","maxFeePerGas":"0x15","maxPriorityFeePerGas":"0x16","input":"0x0b0c0d","r":"0xe","s":"0xe","v":"0xe","yParity":"0x1","chainId":"0x8","accessList":[{"address":"0xde0b295669a9fd93d5f28d9ec85e40f4cb697bae","storageKeys":["0x0000000000000000000000000000000000000000000000000000000000000003","0x0000000000000000000000000000000000000000000000000000000000000007"]}],"transactionType":"0x2"}"#;
        assert_eq!(serialized, expected);

        tx.transaction_type = Option::Some(3);
        let serialized = serde_json::to_string(&tx).unwrap();
        let expected = r#"{"hash":"0x0000000000000000000000000000000000000000000000000000000000000001","nonce":"0x2","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000003","blockNumber":"0x4","transactionIndex":"0x5","from":"0x0000000000000000000000000000000000000006","to":"0x0000000000000000000000000000000000000007","value":"0xe8dc","gasLimit":"0xa","maxFeePerGas":"0x15","maxPriorityFeePerGas":"0x16","blobVersionedHashes":["0xbcd76a82d8a6c5f7a9ffde451c3e916f5d90eab90b8ab8f7e6fdd4e48f4c7cb9","0xd5a9c3f147b4b81d263f4c4f93e56e9c13b08434f9f4b5a3e8dc2e3b7d2f8a29","0xa3e9fb72f7e51b5fa9c4e35f9de9cddc9a0db82c9e4d47a4f8db3ea47f8b6c12"],"maxFeePerBlobGas":"0x0","input":"0x0b0c0d","r":"0xe","s":"0xe","v":"0xe","yParity":"0x1","chainId":"0x8","accessList":[{"address":"0xde0b295669a9fd93d5f28d9ec85e40f4cb697bae","storageKeys":["0x0000000000000000000000000000000000000000000000000000000000000003","0x0000000000000000000000000000000000000000000000000000000000000007"]}],"transactionType":"0x3"}"#;
        assert_eq!(serialized, expected);

        tx.transaction_type = Option::Some(4);
        let serialized = serde_json::to_string(&tx).unwrap();
        let expected = r#"{"hash":"0x0000000000000000000000000000000000000000000000000000000000000001","nonce":"0x2","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000003","blockNumber":"0x4","transactionIndex":"0x5","from":"0x0000000000000000000000000000000000000006","to":"0x0000000000000000000000000000000000000007","value":"0xe8dc","gasLimit":"0xa","maxFeePerGas":"0x15","maxPriorityFeePerGas":"0x16","input":"0x0b0c0d","r":"0xe","s":"0xe","v":"0xe","yParity":"0x1","chainId":"0x8","accessList":[{"address":"0xde0b295669a9fd93d5f28d9ec85e40f4cb697bae","storageKeys":["0x0000000000000000000000000000000000000000000000000000000000000003","0x0000000000000000000000000000000000000000000000000000000000000007"]}],"transactionType":"0x4","authorizationList":[{"chainId":"0x1","address":"0x0000000000000000000000000000000000000006","nonce":"0x1","yParity":"0x0","r":"0x48b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353","s":"0xefffd310ac743f371de3b9f7f9cb56c0b28ad43601b4ab949f53faa07bd2c804"}]}"#;
        assert_eq!(serialized, expected);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn deserialize_all_tx_types() {
        let blob_1 = b256!("bcd76a82d8a6c5f7a9ffde451c3e916f5d90eab90b8ab8f7e6fdd4e48f4c7cb9");
        let blob_2 = b256!("d5a9c3f147b4b81d263f4c4f93e56e9c13b08434f9f4b5a3e8dc2e3b7d2f8a29");
        let blob_3 = b256!("a3e9fb72f7e51b5fa9c4e35f9de9cddc9a0db82c9e4d47a4f8db3ea47f8b6c12");

        let mut serialized = r#"{"hash":"0x0000000000000000000000000000000000000000000000000000000000000001","nonce":"0x2","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000003","blockNumber":"0x4","transactionIndex":"0x5","from":"0x0000000000000000000000000000000000000006","to":"0x0000000000000000000000000000000000000007","value":"0xe8dc","gasPrice":"0x9","gasLimit":"0xa","input":"0x0b0c0d","r":"0xe","s":"0xe","v":"0xe","chainId":"0x2","transactionType":"0x0"}"#;
        let mut deserialized = serde_json::from_str::<Transaction>(serialized).unwrap();
        let mut expected = Transaction {
            hash: B256::from_str(
                "0x0000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
            nonce: 2,
            block_hash: Option::Some(
                B256::from_str(
                    "0x0000000000000000000000000000000000000000000000000000000000000003",
                )
                .unwrap(),
            ),
            block_number: Option::Some(4),
            transaction_index: Option::Some(5),
            from: Address::from_str("0x0000000000000000000000000000000000000006").unwrap(),
            to: Option::Some(
                Address::from_str("0x0000000000000000000000000000000000000007").unwrap(),
            ),
            value: U256::from(59612),
            gas_price: Option::Some(9),
            gas: 10,
            max_fee_per_gas: Option::None,
            max_priority_fee_per_gas: Option::None,
            max_fee_per_blob_gas: Option::None,
            input: vec![11, 12, 13].into(),
            signature: Option::Some(Signature {
                v: U256::from(14),
                r: U256::from(14),
                s: U256::from(14),
                y_parity: Option::None,
            }),
            chain_id: Option::Some(2),
            blob_versioned_hashes: Option::None,
            access_list: Option::None,
            transaction_type: Option::Some(0),
            authorization_list: Option::None,
        };
        assert_eq!(deserialized, expected);

        serialized = r#"{"hash":"0x0000000000000000000000000000000000000000000000000000000000000001","nonce":"0x2","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000003","blockNumber":"0x4","transactionIndex":"0x5","from":"0x0000000000000000000000000000000000000006","to":"0x0000000000000000000000000000000000000007","value":"0xe8dc","gasPrice":"0x9","gasLimit":"0xa","input":"0x0b0c0d","r":"0xe","s":"0xe","v":"0xe","yParity":"0x1","chainId":"0x1","accessList":[{"address":"0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe","storageKeys":["0x0000000000000000000000000000000000000000000000000000000000000003","0x0000000000000000000000000000000000000000000000000000000000000007"]}],"transactionType":"0x1"}"#;
        deserialized = serde_json::from_str::<Transaction>(serialized).unwrap();
        expected = Transaction {
            hash: TxHash::from_str(
                "0x0000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
            nonce: 2,
            block_hash: Option::Some(
                BlockHash::from_str(
                    "0x0000000000000000000000000000000000000000000000000000000000000003",
                )
                .unwrap(),
            ),
            block_number: Option::Some(4),
            transaction_index: Option::Some(5),
            from: Address::from_str("0x0000000000000000000000000000000000000006").unwrap(),
            to: Option::Some(
                Address::from_str("0x0000000000000000000000000000000000000007").unwrap(),
            ),
            value: U256::from(59612),
            gas_price: Option::Some(9),
            gas: 10,
            max_fee_per_gas: Option::None,
            max_priority_fee_per_gas: Option::None,
            max_fee_per_blob_gas: Option::None,
            input: vec![11, 12, 13].into(),
            signature: Some(Signature {
                v: U256::from(14),
                r: U256::from(14),
                s: U256::from(14),
                y_parity: Option::Some(Parity(true)),
            }),
            chain_id: Option::Some(1),
            blob_versioned_hashes: None,
            access_list: Some(AccessList(Vec::from([AccessListItem {
                address: Address::from_str("0xde0b295669a9fd93d5f28d9ec85e40f4cb697bae").unwrap(),
                storage_keys: Vec::from([
                    B256::from_str(
                        "0x0000000000000000000000000000000000000000000000000000000000000003",
                    )
                    .unwrap(),
                    B256::from_str(
                        "0x0000000000000000000000000000000000000000000000000000000000000007",
                    )
                    .unwrap(),
                ]),
            }]))),
            transaction_type: Option::Some(1),
            authorization_list: Option::None,
        };
        assert_eq!(deserialized, expected);

        serialized = r#"{"hash":"0x0000000000000000000000000000000000000000000000000000000000000001","nonce":"0x2","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000003","blockNumber":"0x4","transactionIndex":"0x5","from":"0x0000000000000000000000000000000000000006","to":"0x0000000000000000000000000000000000000007","value":"0xe8dc","gasLimit":"0xa","maxFeePerGas":"0x15","maxPriorityFeePerGas":"0x16","input":"0x0b0c0d","r":"0xe","s":"0xe","v":"0xe","yParity":"0x1","chainId":"0x8","accessList":[{"address":"0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe","storageKeys":["0x0000000000000000000000000000000000000000000000000000000000000003","0x0000000000000000000000000000000000000000000000000000000000000007"]}],"transactionType":"0x2"}"#;
        deserialized = serde_json::from_str::<Transaction>(serialized).unwrap();
        expected = Transaction {
            hash: TxHash::from_str(
                "0x0000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
            nonce: 2,
            block_hash: Option::Some(
                BlockHash::from_str(
                    "0x0000000000000000000000000000000000000000000000000000000000000003",
                )
                .unwrap(),
            ),
            block_number: Option::Some(4),
            transaction_index: Option::Some(5),
            from: Address::from_str("0x0000000000000000000000000000000000000006").unwrap(),
            to: Option::Some(
                Address::from_str("0x0000000000000000000000000000000000000007").unwrap(),
            ),
            value: U256::from(59612),
            gas_price: Option::None,
            gas: 10,
            max_fee_per_gas: Option::Some(21),
            max_priority_fee_per_gas: Option::Some(22),
            max_fee_per_blob_gas: Option::None,
            input: vec![11, 12, 13].into(),
            signature: Some(Signature {
                v: U256::from(14),
                r: U256::from(14),
                s: U256::from(14),
                y_parity: Option::Some(Parity(true)),
            }),
            chain_id: Option::Some(8),
            blob_versioned_hashes: None,
            access_list: Some(AccessList(Vec::from([AccessListItem {
                address: Address::from_str("0xde0b295669a9fd93d5f28d9ec85e40f4cb697bae").unwrap(),
                storage_keys: Vec::from([
                    B256::from_str(
                        "0x0000000000000000000000000000000000000000000000000000000000000003",
                    )
                    .unwrap(),
                    B256::from_str(
                        "0x0000000000000000000000000000000000000000000000000000000000000007",
                    )
                    .unwrap(),
                ]),
            }]))),
            transaction_type: Option::Some(2),
            authorization_list: Option::None,
        };
        assert_eq!(deserialized, expected);

        serialized = r#"{"hash":"0x0000000000000000000000000000000000000000000000000000000000000001","nonce":"0x2","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000003","blockNumber":"0x4","transactionIndex":"0x5","from":"0x0000000000000000000000000000000000000006","to":"0x0000000000000000000000000000000000000007","value":"0xe8dc","gasLimit":"0xa","maxFeePerGas":"0x15","maxPriorityFeePerGas":"0x16","blobVersionedHashes":["0xbcd76a82d8a6c5f7a9ffde451c3e916f5d90eab90b8ab8f7e6fdd4e48f4c7cb9","0xd5a9c3f147b4b81d263f4c4f93e56e9c13b08434f9f4b5a3e8dc2e3b7d2f8a29","0xa3e9fb72f7e51b5fa9c4e35f9de9cddc9a0db82c9e4d47a4f8db3ea47f8b6c12"],"maxFeePerBlobGas":"0x37","input":"0x0b0c0d","r":"0xe","s":"0xe","v":"0xe","yParity":"0x1","chainId":"0x8","accessList":[{"address":"0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe","storageKeys":["0x0000000000000000000000000000000000000000000000000000000000000003","0x0000000000000000000000000000000000000000000000000000000000000007"]}],"transactionType":"0x3"}"#;
        deserialized = serde_json::from_str::<Transaction>(serialized).unwrap();
        expected = Transaction {
            hash: TxHash::from_str(
                "0x0000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
            nonce: 2,
            block_hash: Option::Some(
                BlockHash::from_str(
                    "0x0000000000000000000000000000000000000000000000000000000000000003",
                )
                .unwrap(),
            ),
            block_number: Option::Some(4),
            transaction_index: Option::Some(5),
            from: Address::from_str("0x0000000000000000000000000000000000000006").unwrap(),
            to: Option::Some(
                Address::from_str("0x0000000000000000000000000000000000000007").unwrap(),
            ),
            value: U256::from(59612),
            gas_price: Option::None,
            gas: 10,
            max_fee_per_gas: Option::Some(21),
            max_priority_fee_per_gas: Option::Some(22),
            max_fee_per_blob_gas: Option::Some(55),
            input: vec![11, 12, 13].into(),
            signature: Some(Signature {
                v: U256::from(14),
                r: U256::from(14),
                s: U256::from(14),
                y_parity: Option::Some(Parity(true)),
            }),
            chain_id: Option::Some(8),
            blob_versioned_hashes: Option::Some(vec![blob_1, blob_2, blob_3]),
            access_list: Some(AccessList(Vec::from([AccessListItem {
                address: Address::from_str("0xde0b295669a9fd93d5f28d9ec85e40f4cb697bae").unwrap(),
                storage_keys: Vec::from([
                    B256::from_str(
                        "0x0000000000000000000000000000000000000000000000000000000000000003",
                    )
                    .unwrap(),
                    B256::from_str(
                        "0x0000000000000000000000000000000000000000000000000000000000000007",
                    )
                    .unwrap(),
                ]),
            }]))),
            transaction_type: Option::Some(3),
            authorization_list: Option::None,
        };
        assert_eq!(deserialized, expected);

        serialized = r#"{"hash":"0x0000000000000000000000000000000000000000000000000000000000000001","nonce":"0x2","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000003","blockNumber":"0x4","transactionIndex":"0x5","from":"0x0000000000000000000000000000000000000006","to":"0x0000000000000000000000000000000000000007","value":"0xe8dc","gasLimit":"0xa","maxFeePerGas":"0x15","maxPriorityFeePerGas":"0x16","input":"0x0b0c0d","r":"0xe","s":"0xe","v":"0xe","yParity":"0x1","chainId":"0x8","accessList":[{"address":"0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe","storageKeys":["0x0000000000000000000000000000000000000000000000000000000000000003","0x0000000000000000000000000000000000000000000000000000000000000007"]}],"transactionType":"0x4","authorizationList":[{"chainId":"0x1","address":"0x0000000000000000000000000000000000000006","nonce":"0x1","yParity":"0x0","r":"0x48b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353","s":"0xefffd310ac743f371de3b9f7f9cb56c0b28ad43601b4ab949f53faa07bd2c804"}]}"#;
        deserialized = serde_json::from_str::<Transaction>(serialized).unwrap();
        expected = Transaction { hash: TxHash::from_str("0x0000000000000000000000000000000000000000000000000000000000000001").unwrap(), nonce: 2, block_hash: Option::Some(BlockHash::from_str("0x0000000000000000000000000000000000000000000000000000000000000003").unwrap()), block_number: Option::Some(4), transaction_index: Option::Some(5), from: Address::from_str("0x0000000000000000000000000000000000000006").unwrap(), to: Option::Some(Address::from_str("0x0000000000000000000000000000000000000007").unwrap()), value: U256::from(59612), gas_price: Option::None, gas: 10, max_fee_per_gas: Option::Some(21), max_priority_fee_per_gas: Option::Some(22), max_fee_per_blob_gas: Option::None, input: vec![11, 12, 13].into(), signature: Some(Signature {
            v: U256::from(14),
            r: U256::from(14),
            s: U256::from(14),
            y_parity: Option::Some(Parity(true)),
        }), chain_id: Option::Some(8), blob_versioned_hashes: Option::None, access_list: Some(AccessList(Vec::from([AccessListItem { address: Address::from_str("0xde0b295669a9fd93d5f28d9ec85e40f4cb697bae").unwrap(), storage_keys: Vec::from([B256::from_str("0x0000000000000000000000000000000000000000000000000000000000000003").unwrap(), B256::from_str("0x0000000000000000000000000000000000000000000000000000000000000007").unwrap()]) }]))), transaction_type: Option::Some(4), authorization_list: Option::Some(vec![(Authorization {
            chain_id: 1u64,
            address: Address::left_padding_from(&[6]),
            nonce: 1u64,
        }).into_signed(AlloySignature::from_str("48b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353efffd310ac743f371de3b9f7f9cb56c0b28ad43601b4ab949f53faa07bd2c8041b").unwrap())])};
        assert_eq!(deserialized, expected);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn into_request_legacy() {
        // cast rpc eth_getTransactionByHash
        // 0xe9e91f1ee4b56c0df2e9f06c2b8c27c6076195a88a7b8537ba8313d80e6f124e --rpc-url mainnet
        let rpc_tx = r#"{"hash":"0x0000000000000000000000000000000000000000000000000000000000000001","nonce":"0x2","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000003","blockNumber":"0x4","transactionIndex":"0x5","from":"0x0000000000000000000000000000000000000006","to":"0x0000000000000000000000000000000000000007","value":"0xe8dc","gasPrice":"0x9","gasLimit":"0xa","input":"0x0b0c0d","r":"0xe","s":"0xe","v":"0xe","chainId":"0x2","transactionType":"0x0"}"#;

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
        let rpc_tx = r#"{"hash":"0x0000000000000000000000000000000000000000000000000000000000000001","nonce":"0x2","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000003","blockNumber":"0x4","transactionIndex":"0x5","from":"0x0000000000000000000000000000000000000006","to":"0x0000000000000000000000000000000000000007","value":"0xe8dc","gasLimit":"0xa","maxFeePerGas":"0x15","maxPriorityFeePerGas":"0x16","input":"0x0b0c0d","r":"0xe","s":"0xe","v":"0xe","yParity":"0x1","chainId":"0x8","accessList":[{"address":"0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe","storageKeys":["0x0000000000000000000000000000000000000000000000000000000000000003","0x0000000000000000000000000000000000000000000000000000000000000007"]}],"transactionType":"0x2"}"#;

        let tx = serde_json::from_str::<Transaction>(rpc_tx).unwrap();
        let request = tx.into_request();
        assert!(request.gas_price.is_none());
        assert!(request.max_fee_per_gas.is_some());
    }
}
