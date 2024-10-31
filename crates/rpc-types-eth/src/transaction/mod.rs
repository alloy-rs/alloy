//! RPC types for transactions

use alloy_consensus::{
    Signed, TxEip1559, TxEip2930, TxEip4844, TxEip4844Variant, TxEip7702, TxEnvelope, TxLegacy,
};
use alloy_eips::eip7702::SignedAuthorization;
use alloy_network_primitives::TransactionResponse;
use alloy_primitives::{Address, BlockHash, Bytes, ChainId, TxKind, B256, U256};

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

pub use alloy_consensus::{
    AnyReceiptEnvelope, Receipt, ReceiptEnvelope, ReceiptWithBloom, Transaction as TransactionTrait,
};

/// Transaction object used in RPC
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
// #[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[doc(alias = "Tx")]
pub struct Transaction<T = TxEnvelope> {
    /// The inner transaction object
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub inner: T,

    /// Hash of block where transaction was included, `None` if pending
    #[cfg_attr(feature = "serde", serde(default))]
    pub block_hash: Option<BlockHash>,

    /// Number of block where transaction was included, `None` if pending
    #[cfg_attr(feature = "serde", serde(default, with = "alloy_serde::quantity::opt"))]
    pub block_number: Option<u64>,

    /// Transaction Index
    #[cfg_attr(feature = "serde", serde(default, with = "alloy_serde::quantity::opt"))]
    pub transaction_index: Option<u64>,

    /// Sender
    pub from: Address,
}

impl<T> AsRef<T> for Transaction<T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

impl<T> Transaction<T>
where
    T: TransactionTrait,
{
    /// Returns true if the transaction is a legacy or 2930 transaction.
    pub fn is_legacy_gas(&self) -> bool {
        self.inner.gas_price().is_some()
    }
}

impl<T> Transaction<T>
where
    T: Into<TransactionRequest>,
{
    /// Converts [Transaction] into [TransactionRequest].
    ///
    /// During this conversion data for [TransactionRequest::sidecar] is not
    /// populated as it is not part of [Transaction].
    pub fn into_request(self) -> TransactionRequest {
        self.inner.into()
    }
}

impl TryFrom<Transaction> for Signed<TxLegacy> {
    type Error = ConversionError;

    fn try_from(tx: Transaction) -> Result<Self, Self::Error> {
        match tx.inner {
            TxEnvelope::Legacy(tx) => Ok(tx),
            _ => {
                Err(ConversionError::Custom(format!("expected Legacy, got {}", tx.inner.tx_type())))
            }
        }
    }
}

impl TryFrom<Transaction> for Signed<TxEip1559> {
    type Error = ConversionError;

    fn try_from(tx: Transaction) -> Result<Self, Self::Error> {
        match tx.inner {
            TxEnvelope::Eip1559(tx) => Ok(tx),
            _ => Err(ConversionError::Custom(format!(
                "expected Eip1559, got {}",
                tx.inner.tx_type()
            ))),
        }
    }
}

impl TryFrom<Transaction> for Signed<TxEip2930> {
    type Error = ConversionError;

    fn try_from(tx: Transaction) -> Result<Self, Self::Error> {
        match tx.inner {
            TxEnvelope::Eip2930(tx) => Ok(tx),
            _ => Err(ConversionError::Custom(format!(
                "expected Eip2930, got {}",
                tx.inner.tx_type()
            ))),
        }
    }
}

impl TryFrom<Transaction> for Signed<TxEip4844> {
    type Error = ConversionError;

    fn try_from(tx: Transaction) -> Result<Self, Self::Error> {
        let tx: Signed<TxEip4844Variant> = tx.try_into()?;

        let (tx, sig, hash) = tx.into_parts();

        Ok(Self::new_unchecked(tx.into(), sig, hash))
    }
}

impl TryFrom<Transaction> for Signed<TxEip4844Variant> {
    type Error = ConversionError;

    fn try_from(tx: Transaction) -> Result<Self, Self::Error> {
        match tx.inner {
            TxEnvelope::Eip4844(tx) => Ok(tx),
            _ => Err(ConversionError::Custom(format!(
                "expected TxEip4844Variant, got {}",
                tx.inner.tx_type()
            ))),
        }
    }
}

impl TryFrom<Transaction> for Signed<TxEip7702> {
    type Error = ConversionError;

    fn try_from(tx: Transaction) -> Result<Self, Self::Error> {
        match tx.inner {
            TxEnvelope::Eip7702(tx) => Ok(tx),
            _ => Err(ConversionError::Custom(format!(
                "expected Eip7702, got {}",
                tx.inner.tx_type()
            ))),
        }
    }
}

impl From<Transaction> for TxEnvelope {
    fn from(tx: Transaction) -> Self {
        tx.inner
    }
}

impl<T: TransactionTrait> TransactionTrait for Transaction<T> {
    fn chain_id(&self) -> Option<ChainId> {
        self.inner.chain_id()
    }

    fn nonce(&self) -> u64 {
        self.inner.nonce()
    }

    fn gas_limit(&self) -> u64 {
        self.inner.gas_limit()
    }

    fn gas_price(&self) -> Option<u128> {
        self.inner.gas_price()
    }

    fn max_fee_per_gas(&self) -> u128 {
        self.inner.max_fee_per_gas()
    }

    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        self.inner.max_priority_fee_per_gas()
    }

    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        self.inner.max_fee_per_blob_gas()
    }

    fn priority_fee_or_price(&self) -> u128 {
        self.inner.priority_fee_or_price()
    }

    fn kind(&self) -> TxKind {
        self.inner.kind()
    }

    fn value(&self) -> U256 {
        self.inner.value()
    }

    fn input(&self) -> &Bytes {
        self.inner.input()
    }

    fn ty(&self) -> u8 {
        self.inner.ty()
    }

    fn access_list(&self) -> Option<&AccessList> {
        self.inner.access_list()
    }

    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        self.inner.blob_versioned_hashes()
    }

    fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
        self.inner.authorization_list()
    }
}

impl<T: TransactionTrait> TransactionResponse for Transaction<T> {
    fn tx_hash(&self) -> B256 {
        Default::default()
        // self.hash
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

    #[test]
    fn serde_tx_from_contract_mod() {
        let rpc_tx = r#"{"hash":"0x018b2331d461a4aeedf6a1f9cc37463377578244e6a35216057a8370714e798f","nonce":"0x1","blockHash":"0x6e4e53d1de650d5a5ebed19b38321db369ef1dc357904284ecf4d89b8834969c","blockNumber":"0x2","transactionIndex":"0x0","from":"0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266","to":"0x5fbdb2315678afecb367f032d93f642f64180aa3","value":"0x0","gasPrice":"0x3a29f0f8","gas":"0x1c9c380","maxFeePerGas":"0xba43b7400","maxPriorityFeePerGas":"0x5f5e100","input":"0xd09de08a","r":"0xd309309a59a49021281cb6bb41d164c96eab4e50f0c1bd24c03ca336e7bc2bb7","s":"0x28a7f089143d0a1355ebeb2a1b9f0e5ad9eca4303021c1400d61bc23c9ac5319","v":"0x0","yParity":"0x0","chainId":"0x7a69","accessList":[],"type":"0x2"}"#;

        let tx = serde_json::from_str::<Transaction>(rpc_tx).unwrap();
        assert_eq!(tx.block_number, Some(2));
    }
}
