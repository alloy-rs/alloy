//! Alloy basic Transaction Request type.

use crate::{eth::transaction::AccessList, BlobTransactionSidecar, Transaction};
use alloy_consensus::{
    TxEip1559, TxEip2930, TxEip4844, TxEip4844Variant, TxEip4844WithSidecar, TxEnvelope, TxLegacy,
    TxType, TypedTransaction,
};
use alloy_primitives::{Address, Bytes, ChainId, TxKind, B256, U256};
use serde::{Deserialize, Serialize};
use std::hash::Hash;

/// Represents _all_ transaction requests to/from RPC.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionRequest {
    /// The address of the transaction author.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<Address>,
    /// The destination address of the transaction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<TxKind>,
    /// The legacy gas price.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alloy_serde::num::u128_opt_via_ruint"
    )]
    pub gas_price: Option<u128>,
    /// The max base fee per gas the sender is willing to pay.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alloy_serde::num::u128_opt_via_ruint"
    )]
    pub max_fee_per_gas: Option<u128>,
    /// The max priority fee per gas the sender is willing to pay, also called the miner tip.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alloy_serde::num::u128_opt_via_ruint"
    )]
    pub max_priority_fee_per_gas: Option<u128>,
    /// The max fee per blob gas for EIP-4844 blob transactions.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alloy_serde::num::u128_opt_via_ruint"
    )]
    pub max_fee_per_blob_gas: Option<u128>,
    /// The gas limit for the transaction.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alloy_serde::num::u128_opt_via_ruint"
    )]
    pub gas: Option<u128>,
    /// The value transferred in the transaction, in wei.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<U256>,
    /// Transaction data.
    #[serde(default, flatten)]
    pub input: TransactionInput,
    /// The nonce of the transaction.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alloy_serde::num::u64_opt_via_ruint"
    )]
    pub nonce: Option<u64>,
    /// The chain ID for the transaction.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "alloy_serde::num::u64_opt_via_ruint"
    )]
    pub chain_id: Option<ChainId>,
    /// An EIP-2930 access list, which lowers cost for accessing accounts and storages in the list. See [EIP-2930](https://eips.ethereum.org/EIPS/eip-2930) for more information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_list: Option<AccessList>,
    /// The EIP-2718 transaction type. See [EIP-2718](https://eips.ethereum.org/EIPS/eip-2718) for more information.
    #[serde(
        default,
        rename = "type",
        skip_serializing_if = "Option::is_none",
        with = "alloy_serde::num::u8_opt_via_ruint"
    )]
    pub transaction_type: Option<u8>,
    /// Blob versioned hashes for EIP-4844 transactions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blob_versioned_hashes: Option<Vec<B256>>,
    /// Blob sidecar for EIP-4844 transactions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sidecar: Option<BlobTransactionSidecar>,
}

impl TransactionRequest {
    /// Sets the `from` field in the call to the provided address
    #[inline]
    pub const fn from(mut self, from: Address) -> Self {
        self.from = Some(from);
        self
    }

    /// Sets the transactions type for the transactions.
    pub const fn transaction_type(mut self, transaction_type: u8) -> Self {
        self.transaction_type = Some(transaction_type);
        self
    }

    /// Sets the gas limit for the transaction.
    pub const fn gas_limit(mut self, gas_limit: u128) -> Self {
        self.gas = Some(gas_limit);
        self
    }

    /// Sets the nonce for the transaction.
    pub const fn nonce(mut self, nonce: u64) -> Self {
        self.nonce = Some(nonce);
        self
    }

    /// Sets the maximum fee per gas for the transaction.
    pub const fn max_fee_per_gas(mut self, max_fee_per_gas: u128) -> Self {
        self.max_fee_per_gas = Some(max_fee_per_gas);
        self
    }

    /// Sets the maximum priority fee per gas for the transaction.
    pub const fn max_priority_fee_per_gas(mut self, max_priority_fee_per_gas: u128) -> Self {
        self.max_priority_fee_per_gas = Some(max_priority_fee_per_gas);
        self
    }

    /// Sets the recipient address for the transaction.
    #[inline]
    pub const fn to(mut self, to: Address) -> Self {
        self.to = Some(TxKind::Call(to));
        self
    }

    /// Sets the value (amount) for the transaction.
    pub const fn value(mut self, value: U256) -> Self {
        self.value = Some(value);
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

    /// Returns the configured fee cap, if any.
    ///
    /// The returns `gas_price` (legacy) if set or `max_fee_per_gas` (EIP1559)
    #[inline]
    pub fn fee_cap(&self) -> Option<u128> {
        self.gas_price.or(self.max_fee_per_gas)
    }

    /// Populate the `blob_versioned_hashes` key, if a sidecar exists. No
    /// effect otherwise.
    pub fn populate_blob_hashes(&mut self) {
        if let Some(sidecar) = self.sidecar.as_ref() {
            self.blob_versioned_hashes = Some(sidecar.versioned_hashes().collect())
        }
    }

    /// Gets invalid fields for all transaction types
    pub fn get_invalid_common_fields(&self) -> Vec<&'static str> {
        let mut errors = vec![];

        if self.nonce.is_none() {
            errors.push("nonce");
        }

        if self.gas.is_none() {
            errors.push("gas_limit");
        }

        errors
    }

    /// Gets invalid fields for EIP-1559 transaction type
    pub fn get_invalid_1559_fields(&self) -> Vec<&'static str> {
        let mut errors = vec![];

        if self.max_priority_fee_per_gas.is_none() {
            errors.push("max_priority_fee_per_gas");
        }

        if self.max_fee_per_gas.is_none() {
            errors.push("max_fee_per_gas");
        }

        errors
    }

    /// Build a legacy transaction.
    ///
    /// # Panics
    ///
    /// If required fields are missing. Use `complete_legacy` to check if the
    /// request can be built.
    fn build_legacy(self) -> TxLegacy {
        let checked_to = self.to.expect("checked in complete_legacy.");

        TxLegacy {
            chain_id: self.chain_id,
            nonce: self.nonce.expect("checked in complete_legacy"),
            gas_price: self.gas_price.expect("checked in complete_legacy"),
            gas_limit: self.gas.expect("checked in complete_legacy"),
            to: checked_to,
            value: self.value.unwrap_or_default(),
            input: self.input.into_input().unwrap_or_default(),
        }
    }

    /// Build an EIP-1559 transaction.
    ///
    /// # Panics
    ///
    /// If required fields are missing. Use `complete_1559` to check if the
    /// request can be built.
    fn build_1559(self) -> TxEip1559 {
        let checked_to = self.to.expect("checked in complete_1559.");

        TxEip1559 {
            chain_id: self.chain_id.unwrap_or(1),
            nonce: self.nonce.expect("checked in invalid_common_fields"),
            max_priority_fee_per_gas: self
                .max_priority_fee_per_gas
                .expect("checked in invalid_1559_fields"),
            max_fee_per_gas: self.max_fee_per_gas.expect("checked in invalid_1559_fields"),
            gas_limit: self.gas.expect("checked in invalid_common_fields"),
            to: checked_to,
            value: self.value.unwrap_or_default(),
            input: self.input.into_input().unwrap_or_default(),
            access_list: self.access_list.unwrap_or_default(),
        }
    }

    /// Build an EIP-2930 transaction.
    ///
    /// # Panics
    ///
    /// If required fields are missing. Use `complete_2930` to check if the
    /// request can be built.
    fn build_2930(self) -> TxEip2930 {
        let checked_to = self.to.expect("checked in complete_2930.");

        TxEip2930 {
            chain_id: self.chain_id.unwrap_or(1),
            nonce: self.nonce.expect("checked in complete_2930"),
            gas_price: self.gas_price.expect("checked in complete_2930"),
            gas_limit: self.gas.expect("checked in complete_2930"),
            to: checked_to,
            value: self.value.unwrap_or_default(),
            input: self.input.into_input().unwrap_or_default(),
            access_list: self.access_list.unwrap_or_default(),
        }
    }

    /// Build an EIP-4844 transaction.
    ///
    /// # Panics
    ///
    /// If required fields are missing. Use `complete_4844` to check if the
    /// request can be built.
    fn build_4844(mut self) -> TxEip4844WithSidecar {
        self.populate_blob_hashes();

        let checked_to = self.to.expect("checked in complete_4844.");
        let to_address = match checked_to {
            TxKind::Create => panic!("the field `to` can only be of type TxKind::Call(Account). Please change it accordingly."),
            TxKind::Call(to) => to,
        };

        TxEip4844WithSidecar {
            sidecar: self.sidecar.expect("checked in complete_4844"),
            tx: TxEip4844 {
                chain_id: self.chain_id.unwrap_or(1),
                nonce: self.nonce.expect("checked in complete_4844"),
                gas_limit: self.gas.expect("checked in complete_4844"),
                max_fee_per_gas: self.max_fee_per_gas.expect("checked in complete_4844"),
                max_priority_fee_per_gas: self
                    .max_priority_fee_per_gas
                    .expect("checked in complete_4844"),
                to: to_address,
                value: self.value.unwrap_or_default(),
                access_list: self.access_list.unwrap_or_default(),
                blob_versioned_hashes: self
                    .blob_versioned_hashes
                    .expect("populated at top of block"),
                max_fee_per_blob_gas: self.max_fee_per_blob_gas.expect("checked in complete_4844"),
                input: self.input.into_input().unwrap_or_default(),
            },
        }
    }

    fn check_reqd_fields(&self) -> Vec<&'static str> {
        let mut missing = Vec::with_capacity(12);
        if self.nonce.is_none() {
            missing.push("nonce");
        }
        if self.gas.is_none() {
            missing.push("gas_limit");
        }
        if self.to.is_none() {
            missing.push("to");
        }
        missing
    }

    fn check_legacy_fields(&self, missing: &mut Vec<&'static str>) {
        if self.gas_price.is_none() {
            missing.push("gas_price");
        }
    }

    fn check_1559_fields(&self, missing: &mut Vec<&'static str>) {
        if self.max_fee_per_gas.is_none() {
            missing.push("max_fee_per_gas");
        }
        if self.max_priority_fee_per_gas.is_none() {
            missing.push("max_priority_fee_per_gas");
        }
    }

    /// Trim field conflicts, based on the preferred type
    ///
    /// This is used to ensure that the request will not be rejected by the
    /// server due to conflicting keys, and should only be called before
    /// submission via rpc.
    pub fn trim_conflicting_keys(&mut self) {
        match self.preferred_type() {
            TxType::Legacy | TxType::Eip2930 => {
                self.max_fee_per_gas = None;
                self.max_priority_fee_per_gas = None;
                self.max_fee_per_blob_gas = None;
                self.access_list = None;
                self.blob_versioned_hashes = None;
                self.sidecar = None;
            }
            TxType::Eip1559 => {
                self.gas_price = None;
                self.access_list = None;
                self.blob_versioned_hashes = None;
                self.sidecar = None;
            }
            TxType::Eip4844 => {
                self.gas_price = None;
                self.access_list = None;
            }
        }
    }

    /// Check this builder's preferred type, based on the fields that are set.
    ///
    /// Types are preferred as follows:
    /// - EIP-4844 if sidecar or max_blob_fee_per_gas is set
    /// - EIP-2930 if access_list is set
    /// - Legacy if gas_price is set and access_list is unset
    /// - EIP-1559 in all other cases
    pub const fn preferred_type(&self) -> TxType {
        if self.sidecar.is_some() || self.max_fee_per_blob_gas.is_some() {
            TxType::Eip4844
        } else if self.access_list.is_some() {
            TxType::Eip2930
        } else if self.gas_price.is_some() {
            TxType::Legacy
        } else {
            TxType::Eip1559
        }
    }

    /// Check if all necessary keys are present to build a transaction.
    ///
    /// # Returns
    ///
    /// - Ok(type) if all necessary keys are present to build the preferred
    /// type.
    /// - Err((type, missing)) if some keys are missing to build the preferred
    /// type.
    pub fn missing_keys(&self) -> Result<TxType, (TxType, Vec<&'static str>)> {
        let pref = self.preferred_type();
        if let Err(missing) = match pref {
            TxType::Legacy => self.complete_legacy(),
            TxType::Eip2930 => self.complete_2930(),
            TxType::Eip1559 => self.complete_1559(),
            TxType::Eip4844 => self.complete_4844(),
        } {
            Err((pref, missing))
        } else {
            Ok(pref)
        }
    }

    /// Check if all necessary keys are present to build a 4844 transaction,
    /// returning a list of keys that are missing.
    pub fn complete_4844(&self) -> Result<(), Vec<&'static str>> {
        let mut missing = self.check_reqd_fields();
        self.check_1559_fields(&mut missing);

        if self.to.is_none() {
            missing.push("to");
        }

        if self.sidecar.is_none() {
            missing.push("sidecar");
        }

        if self.max_fee_per_blob_gas.is_none() {
            missing.push("max_fee_per_blob_gas");
        }

        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }

    /// Check if all necessary keys are present to build a 1559 transaction,
    /// returning a list of keys that are missing.
    pub fn complete_1559(&self) -> Result<(), Vec<&'static str>> {
        let mut missing = self.check_reqd_fields();
        self.check_1559_fields(&mut missing);
        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }

    /// Check if all necessary keys are present to build a 2930 transaction,
    /// returning a list of keys that are missing.
    pub fn complete_2930(&self) -> Result<(), Vec<&'static str>> {
        let mut missing = self.check_reqd_fields();
        self.check_legacy_fields(&mut missing);

        if self.access_list.is_none() {
            missing.push("access_list");
        }

        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }

    /// Check if all necessary keys are present to build a legacy transaction,
    /// returning a list of keys that are missing.
    pub fn complete_legacy(&self) -> Result<(), Vec<&'static str>> {
        let mut missing = self.check_reqd_fields();
        self.check_legacy_fields(&mut missing);

        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }

    /// Return the tx type this request can be built as. Computed by checking
    /// the preferred type, and then checking for completeness.
    pub fn buildable_type(&self) -> Option<TxType> {
        let pref = self.preferred_type();
        match pref {
            TxType::Legacy => self.complete_legacy().ok(),
            TxType::Eip2930 => self.complete_2930().ok(),
            TxType::Eip1559 => self.complete_1559().ok(),
            TxType::Eip4844 => self.complete_4844().ok(),
        }?;
        Some(pref)
    }

    /// Build an [`TypedTransaction`]
    pub fn build_typed_tx(self) -> Result<TypedTransaction, Self> {
        let tx_type = self.buildable_type();

        if tx_type.is_none() {
            return Err(self);
        }

        Ok(match tx_type.expect("checked") {
            TxType::Legacy => self.build_legacy().into(),
            TxType::Eip2930 => self.build_2930().into(),
            TxType::Eip1559 => self.build_1559().into(),
            TxType::Eip4844 => self.build_4844().into(),
        })
    }
}

/// Helper type that supports both `data` and `input` fields that map to transaction input data.
///
/// This is done for compatibility reasons where older implementations used `data` instead of the
/// newer, recommended `input` field.
///
/// If both fields are set, it is expected that they contain the same value, otherwise an error is
/// returned.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransactionInput {
    /// Transaction data
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<Bytes>,
    /// Transaction data
    ///
    /// This is the same as `input` but is used for backwards compatibility: <https://github.com/ethereum/go-ethereum/issues/15628>
    #[serde(default, skip_serializing_if = "Option::is_none")]
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

impl From<Vec<u8>> for TransactionInput {
    fn from(input: Vec<u8>) -> Self {
        Self { input: Some(input.into()), data: None }
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

impl From<Transaction> for TransactionRequest {
    fn from(tx: Transaction) -> TransactionRequest {
        tx.into_request()
    }
}

impl From<TxLegacy> for TransactionRequest {
    fn from(tx: TxLegacy) -> TransactionRequest {
        TransactionRequest {
            from: None,
            to: if let TxKind::Call(to) = tx.to { Some(TxKind::Call(to)) } else { None },
            gas_price: Some(tx.gas_price),
            gas: Some(tx.gas_limit),
            value: Some(tx.value),
            input: TransactionInput::from(tx.input),
            nonce: Some(tx.nonce),
            chain_id: tx.chain_id,
            transaction_type: Some(0),
            ..Default::default()
        }
    }
}

impl From<TxEip2930> for TransactionRequest {
    fn from(tx: TxEip2930) -> TransactionRequest {
        TransactionRequest {
            from: None,
            to: if let TxKind::Call(to) = tx.to { Some(TxKind::Call(to)) } else { None },
            gas_price: Some(tx.gas_price),
            gas: Some(tx.gas_limit),
            value: Some(tx.value),
            input: TransactionInput::from(tx.input),
            nonce: Some(tx.nonce),
            chain_id: Some(tx.chain_id),
            access_list: Some(tx.access_list),
            transaction_type: Some(1),
            ..Default::default()
        }
    }
}

impl From<TxEip1559> for TransactionRequest {
    fn from(tx: TxEip1559) -> TransactionRequest {
        TransactionRequest {
            from: None,
            to: if let TxKind::Call(to) = tx.to { Some(TxKind::Call(to)) } else { None },
            max_fee_per_gas: Some(tx.max_fee_per_gas),
            max_priority_fee_per_gas: Some(tx.max_priority_fee_per_gas),
            gas: Some(tx.gas_limit),
            value: Some(tx.value),
            input: TransactionInput::from(tx.input),
            nonce: Some(tx.nonce),
            chain_id: Some(tx.chain_id),
            access_list: Some(tx.access_list),
            transaction_type: Some(2),
            ..Default::default()
        }
    }
}

impl From<TxEip4844> for TransactionRequest {
    fn from(tx: TxEip4844) -> TransactionRequest {
        TransactionRequest {
            from: None,
            to: Some(TxKind::Call(tx.to)),
            max_fee_per_blob_gas: Some(tx.max_fee_per_blob_gas),
            gas: Some(tx.gas_limit),
            max_fee_per_gas: Some(tx.max_fee_per_gas),
            max_priority_fee_per_gas: Some(tx.max_priority_fee_per_gas),
            value: Some(tx.value),
            input: TransactionInput::from(tx.input),
            nonce: Some(tx.nonce),
            chain_id: Some(tx.chain_id),
            access_list: Some(tx.access_list),
            blob_versioned_hashes: Some(tx.blob_versioned_hashes),
            transaction_type: Some(3),
            ..Default::default()
        }
    }
}

impl From<TxEip4844WithSidecar> for TransactionRequest {
    fn from(tx: TxEip4844WithSidecar) -> TransactionRequest {
        let sidecar = tx.sidecar;
        let tx = tx.tx;
        TransactionRequest {
            from: None,
            to: Some(TxKind::Call(tx.to)),
            max_fee_per_blob_gas: Some(tx.max_fee_per_blob_gas),
            gas: Some(tx.gas_limit),
            max_fee_per_gas: Some(tx.max_fee_per_gas),
            max_priority_fee_per_gas: Some(tx.max_priority_fee_per_gas),
            value: Some(tx.value),
            input: TransactionInput::from(tx.input),
            nonce: Some(tx.nonce),
            chain_id: Some(tx.chain_id),
            access_list: Some(tx.access_list),
            blob_versioned_hashes: Some(tx.blob_versioned_hashes),
            sidecar: Some(sidecar),
            transaction_type: Some(3),
            ..Default::default()
        }
    }
}

impl From<TxEip4844Variant> for TransactionRequest {
    fn from(tx: TxEip4844Variant) -> TransactionRequest {
        match tx {
            TxEip4844Variant::TxEip4844(tx) => tx.into(),
            TxEip4844Variant::TxEip4844WithSidecar(tx) => tx.into(),
        }
    }
}

impl From<TypedTransaction> for TransactionRequest {
    fn from(tx: TypedTransaction) -> TransactionRequest {
        match tx {
            TypedTransaction::Legacy(tx) => tx.into(),
            TypedTransaction::Eip2930(tx) => tx.into(),
            TypedTransaction::Eip1559(tx) => tx.into(),
            TypedTransaction::Eip4844(tx) => tx.into(),
        }
    }
}

impl From<TxEnvelope> for TransactionRequest {
    fn from(envelope: TxEnvelope) -> TransactionRequest {
        match envelope {
            TxEnvelope::Legacy(tx) => {
                #[cfg(feature = "k256")]
                {
                    let from = tx.recover_signer().ok();
                    let tx: TransactionRequest = tx.strip_signature().into();
                    if let Some(from) = from {
                        tx.from(from)
                    } else {
                        tx
                    }
                }

                #[cfg(not(feature = "k256"))]
                {
                    tx.strip_signature().into()
                }
            }
            TxEnvelope::Eip2930(tx) => {
                #[cfg(feature = "k256")]
                {
                    let from = tx.recover_signer().ok();
                    let tx: TransactionRequest = tx.strip_signature().into();
                    if let Some(from) = from {
                        tx.from(from)
                    } else {
                        tx
                    }
                }

                #[cfg(not(feature = "k256"))]
                {
                    tx.strip_signature().into()
                }
            }
            TxEnvelope::Eip1559(tx) => {
                #[cfg(feature = "k256")]
                {
                    let from = tx.recover_signer().ok();
                    let tx: TransactionRequest = tx.strip_signature().into();
                    if let Some(from) = from {
                        tx.from(from)
                    } else {
                        tx
                    }
                }

                #[cfg(not(feature = "k256"))]
                {
                    tx.strip_signature().into()
                }
            }
            TxEnvelope::Eip4844(tx) => {
                #[cfg(feature = "k256")]
                {
                    let from = tx.recover_signer().ok();
                    let tx: TransactionRequest = tx.strip_signature().into();
                    if let Some(from) = from {
                        tx.from(from)
                    } else {
                        tx
                    }
                }

                #[cfg(not(feature = "k256"))]
                {
                    tx.strip_signature().into()
                }
            }
            _ => Default::default(),
        }
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
    use crate::WithOtherFields;
    use alloy_primitives::b256;

    // <https://github.com/paradigmxyz/reth/issues/6670>
    #[test]
    fn serde_from_to() {
        let s = r#"{"from":"0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266", "to":"0x70997970C51812dc3A010C7d01b50e0d17dc79C8" }"#;
        let req = serde_json::from_str::<TransactionRequest>(s).unwrap();
        assert!(req.input.check_unique_input().is_ok())
    }

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

    #[test]
    fn serde_tx_request_additional_fields() {
        let s = r#"{"accessList":[],"data":"0x0902f1ac","to":"0xa478c2975ab1ea89e8196811f51a7b7ade33eb11","type":"0x02","sourceHash":"0xbf7e331f7f7c1dd2e05159666b3bf8bc7a8a3a9eb1d518969eab529dd9b88c1a"}"#;
        let req = serde_json::from_str::<WithOtherFields<TransactionRequest>>(s).unwrap();
        assert_eq!(
            req.other.get_deserialized::<B256>("sourceHash").unwrap().unwrap(),
            b256!("bf7e331f7f7c1dd2e05159666b3bf8bc7a8a3a9eb1d518969eab529dd9b88c1a")
        );
    }

    #[test]
    fn serde_tx_chain_id_field() {
        let chain_id: u64 = 12345678;

        let chain_id_as_num = format!(r#"{{"chainId": {} }}"#, chain_id);
        let req1 = serde_json::from_str::<TransactionRequest>(&chain_id_as_num).unwrap();
        assert_eq!(req1.chain_id.unwrap(), chain_id);

        let chain_id_as_hex = format!(r#"{{"chainId": "0x{:x}" }}"#, chain_id);
        let req2 = serde_json::from_str::<TransactionRequest>(&chain_id_as_hex).unwrap();
        assert_eq!(req2.chain_id.unwrap(), chain_id);
    }

    #[test]
    fn serde_empty() {
        let tx = TransactionRequest::default();
        let serialized = serde_json::to_string(&tx).unwrap();
        assert_eq!(serialized, "{}");
    }
}
