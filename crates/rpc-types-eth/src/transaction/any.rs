use alloy_consensus::{
    SignableTransaction, Signed, TxEip1559, TxEip2930, TxEip4844, TxEip4844Variant, TxEip7702,
    TxEnvelope, TxLegacy, TxType,
};
use alloy_eips::{eip2930::AccessList, eip7702::SignedAuthorization};
use alloy_primitives::{Address, BlockHash, Bytes, ChainId, Signature, TxHash, B256, U256};
use serde::{Deserialize, Serialize};

use super::ConversionError;

/// Transaction object containing fields which might be present in Ethereum transaction types.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[serde(rename_all = "camelCase")]
pub struct AnyTxEnvelope {
    /// Hash
    pub hash: TxHash,
    /// Nonce
    #[serde(with = "alloy_serde::quantity")]
    pub nonce: u64,
    /// Block hash
    #[serde(default)]
    pub block_hash: Option<BlockHash>,
    /// Block number
    #[serde(default, with = "alloy_serde::quantity::opt")]
    pub block_number: Option<u64>,
    /// Transaction Index
    #[serde(default, with = "alloy_serde::quantity::opt")]
    pub transaction_index: Option<u64>,
    /// Sender
    pub from: Address,
    /// Recipient
    pub to: Option<Address>,
    /// Transferred value
    pub value: U256,
    /// Gas Price
    #[serde(default, skip_serializing_if = "Option::is_none", with = "alloy_serde::quantity::opt")]
    pub gas_price: Option<u128>,
    /// Gas amount
    #[serde(with = "alloy_serde::quantity")]
    pub gas: u128,
    /// Max BaseFeePerGas the user is willing to pay.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "alloy_serde::quantity::opt")]
    pub max_fee_per_gas: Option<u128>,
    /// The miner's tip.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "alloy_serde::quantity::opt")]
    pub max_priority_fee_per_gas: Option<u128>,
    /// Configured max fee per blob gas for eip-4844 transactions
    #[serde(default, skip_serializing_if = "Option::is_none", with = "alloy_serde::quantity::opt")]
    pub max_fee_per_blob_gas: Option<u128>,
    /// Data
    pub input: Bytes,
    /// All _flattened_ fields of the transaction signature.
    ///
    /// Note: this is an option so special transaction types without a signature (e.g. <https://github.com/ethereum-optimism/optimism/blob/0bf643c4147b43cd6f25a759d331ef3a2a61a2a3/specs/deposits.md#the-deposited-transaction-type>) can be supported.
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub signature: Option<Signature>,
    /// The chain id of the transaction, if any.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "alloy_serde::quantity::opt")]
    pub chain_id: Option<ChainId>,
    /// Contains the blob hashes for eip-4844 transactions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_versioned_hashes: Option<Vec<B256>>,
    /// EIP2930
    ///
    /// Pre-pay to warm storage access.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_list: Option<AccessList>,
    /// EIP2718
    ///
    /// Transaction type,
    /// Some(4) for EIP-7702 transaction, Some(3) for EIP-4844 transaction, Some(2) for EIP-1559
    /// transaction, Some(1) for AccessList transaction, None or Some(0) for Legacy
    #[serde(
        default,
        rename = "type",
        skip_serializing_if = "Option::is_none",
        with = "alloy_serde::quantity::opt"
    )]
    #[doc(alias = "tx_type")]
    pub transaction_type: Option<u8>,
    /// The signed authorization list is a list of tuples that store the address to code which the
    /// signer desires to execute in the context of their EOA and their signature.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorization_list: Option<Vec<SignedAuthorization>>,
}

impl<TX> TryFrom<AnyTxEnvelope> for Signed<TX>
where
    TX: SignableTransaction<Signature> + TryFrom<AnyTxEnvelope, Error = ConversionError>,
{
    type Error = ConversionError;

    fn try_from(value: AnyTxEnvelope) -> Result<Self, Self::Error> {
        let signature = value.signature.ok_or(ConversionError::MissingSignature)?;
        let tx = TX::try_from(value)?;
        Ok(tx.into_signed(signature))
    }
}

impl TryFrom<AnyTxEnvelope> for TxLegacy {
    type Error = ConversionError;

    fn try_from(tx: AnyTxEnvelope) -> Result<Self, Self::Error> {
        Ok(Self {
            chain_id: tx.chain_id,
            nonce: tx.nonce,
            gas_price: tx.gas_price.ok_or(ConversionError::MissingGasPrice)?,
            gas_limit: tx.gas,
            to: tx.to.into(),
            value: tx.value,
            input: tx.input,
        })
    }
}

impl TryFrom<AnyTxEnvelope> for TxEip1559 {
    type Error = ConversionError;

    fn try_from(tx: AnyTxEnvelope) -> Result<Self, Self::Error> {
        Ok(Self {
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
        })
    }
}

impl TryFrom<AnyTxEnvelope> for TxEip2930 {
    type Error = ConversionError;

    fn try_from(tx: AnyTxEnvelope) -> Result<Self, Self::Error> {
        Ok(Self {
            chain_id: tx.chain_id.ok_or(ConversionError::MissingChainId)?,
            nonce: tx.nonce,
            gas_price: tx.gas_price.ok_or(ConversionError::MissingGasPrice)?,
            gas_limit: tx.gas,
            to: tx.to.into(),
            value: tx.value,
            input: tx.input,
            access_list: tx.access_list.ok_or(ConversionError::MissingAccessList)?,
        })
    }
}

impl TryFrom<AnyTxEnvelope> for TxEip4844 {
    type Error = ConversionError;

    fn try_from(tx: AnyTxEnvelope) -> Result<Self, Self::Error> {
        Ok(Self {
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
        })
    }
}

impl TryFrom<AnyTxEnvelope> for TxEip4844Variant {
    type Error = ConversionError;

    fn try_from(tx: AnyTxEnvelope) -> Result<Self, Self::Error> {
        let tx: TxEip4844 = tx.try_into()?;
        Ok(tx.into())
    }
}

impl TryFrom<AnyTxEnvelope> for TxEip7702 {
    type Error = ConversionError;

    fn try_from(tx: AnyTxEnvelope) -> Result<Self, Self::Error> {
        Ok(Self {
            chain_id: tx.chain_id.ok_or(ConversionError::MissingChainId)?,
            nonce: tx.nonce,
            gas_limit: tx.gas,
            max_fee_per_gas: tx.max_fee_per_gas.ok_or(ConversionError::MissingMaxFeePerGas)?,
            max_priority_fee_per_gas: tx
                .max_priority_fee_per_gas
                .ok_or(ConversionError::MissingMaxPriorityFeePerGas)?,
            to: tx.to.into(),
            value: tx.value,
            access_list: tx.access_list.ok_or(ConversionError::MissingAccessList)?,
            authorization_list: tx
                .authorization_list
                .ok_or(ConversionError::MissingAuthorizationList)?,
            input: tx.input,
        })
    }
}

impl TryFrom<AnyTxEnvelope> for TxEnvelope {
    type Error = ConversionError;

    fn try_from(tx: AnyTxEnvelope) -> Result<Self, Self::Error> {
        match tx.transaction_type.unwrap_or_default().try_into()? {
            TxType::Legacy => Ok(Self::Legacy(tx.try_into()?)),
            TxType::Eip1559 => Ok(Self::Eip1559(tx.try_into()?)),
            TxType::Eip2930 => Ok(Self::Eip2930(tx.try_into()?)),
            TxType::Eip4844 => Ok(Self::Eip4844(tx.try_into()?)),
            TxType::Eip7702 => Ok(Self::Eip7702(tx.try_into()?)),
        }
    }
}
