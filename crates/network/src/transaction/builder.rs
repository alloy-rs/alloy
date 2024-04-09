use super::signer::NetworkSigner;
use crate::Network;
use alloy_consensus::BlobTransactionSidecar;
use alloy_primitives::{Address, Bytes, ChainId, TxKind, U256};
use alloy_rpc_types::AccessList;
use futures_utils_wasm::impl_future;

/// Error type for transaction builders.
#[derive(Debug, thiserror::Error)]
pub enum TransactionBuilderError {
    /// A required key is missing.
    #[error("A required key is missing: {0}")]
    MissingKey(&'static str),

    /// Signer cannot produce signature type required for transaction.
    #[error("Signer cannot produce signature type required for transaction")]
    UnsupportedSignatureType,

    /// Signer error.
    #[error(transparent)]
    Signer(#[from] alloy_signer::Error),

    /// A custom error.
    #[error("{0}")]
    Custom(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl TransactionBuilderError {
    /// Instantiate a custom error.
    pub fn custom<E>(e: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Custom(Box::new(e))
    }
}

/// [`TransactionBuilder`] result type.
pub type BuilderResult<T, E = TransactionBuilderError> = std::result::Result<T, E>;

/// A Transaction builder for a network.
///
/// Transaction builders are primarily used to construct typed transactions that can be signed with
/// [`TransactionBuilder::build`], or unsigned typed transactions with
/// [`TransactionBuilder::build_unsigned`].
///
/// Transaction builders should be able to construct all available transaction types on a given
/// network.
pub trait TransactionBuilder<N: Network>: Default + Sized + Send + Sync + 'static {
    /// Get the chain ID for the transaction.
    fn chain_id(&self) -> Option<ChainId>;

    /// Set the chain ID for the transaction.
    fn set_chain_id(&mut self, chain_id: ChainId);

    /// Builder-pattern method for setting the chain ID.
    fn with_chain_id(mut self, chain_id: alloy_primitives::ChainId) -> Self {
        self.set_chain_id(chain_id);
        self
    }

    /// Get the nonce for the transaction.
    fn nonce(&self) -> Option<u64>;

    /// Set the nonce for the transaction.
    fn set_nonce(&mut self, nonce: u64);

    /// Builder-pattern method for setting the nonce.
    fn with_nonce(mut self, nonce: u64) -> Self {
        self.set_nonce(nonce);
        self
    }

    /// Get the input data for the transaction.
    fn input(&self) -> Option<&Bytes>;

    /// Set the input data for the transaction.
    fn set_input(&mut self, input: Bytes);

    /// Builder-pattern method for setting the input data.
    fn with_input(mut self, input: Bytes) -> Self {
        self.set_input(input);
        self
    }

    /// Get the sender for the transaction.
    fn from(&self) -> Option<Address>;

    /// Set the sender for the transaction.
    fn set_from(&mut self, from: Address);

    /// Builder-pattern method for setting the sender.
    fn with_from(mut self, from: Address) -> Self {
        self.set_from(from);
        self
    }

    /// Get the recipient for the transaction.
    fn to(&self) -> Option<TxKind>;

    /// Set the recipient for the transaction.
    fn set_to(&mut self, to: TxKind);

    /// Builder-pattern method for setting the recipient.
    fn with_to(mut self, to: TxKind) -> Self {
        self.set_to(to);
        self
    }

    /// Calculates the address that will be created by the transaction, if any.
    ///
    /// Returns `None` if the transaction is not a contract creation (the `to` field is set), or if
    /// the `from` or `nonce` fields are not set.
    fn calculate_create_address(&self) -> Option<Address> {
        if !self.to().is_some_and(|to| to.is_create()) {
            return None;
        }
        let from = self.from()?;
        let nonce = self.nonce()?;
        Some(from.create(nonce))
    }

    /// Get the value for the transaction.
    fn value(&self) -> Option<U256>;

    /// Set the value for the transaction.
    fn set_value(&mut self, value: U256);

    /// Builder-pattern method for setting the value.
    fn with_value(mut self, value: U256) -> Self {
        self.set_value(value);
        self
    }

    /// Get the legacy gas price for the transaction.
    fn gas_price(&self) -> Option<u128>;

    /// Set the legacy gas price for the transaction.
    fn set_gas_price(&mut self, gas_price: u128);

    /// Builder-pattern method for setting the legacy gas price.
    fn with_gas_price(mut self, gas_price: u128) -> Self {
        self.set_gas_price(gas_price);
        self
    }

    /// Get the max fee per gas for the transaction.
    fn max_fee_per_gas(&self) -> Option<u128>;

    /// Set the max fee per gas  for the transaction.
    fn set_max_fee_per_gas(&mut self, max_fee_per_gas: u128);

    /// Builder-pattern method for setting max fee per gas .
    fn with_max_fee_per_gas(mut self, max_fee_per_gas: u128) -> Self {
        self.set_max_fee_per_gas(max_fee_per_gas);
        self
    }

    /// Get the max priority fee per gas for the transaction.
    fn max_priority_fee_per_gas(&self) -> Option<u128>;

    /// Set the max priority fee per gas for the transaction.
    fn set_max_priority_fee_per_gas(&mut self, max_priority_fee_per_gas: u128);

    /// Builder-pattern method for setting max priority fee per gas.
    fn with_max_priority_fee_per_gas(mut self, max_priority_fee_per_gas: u128) -> Self {
        self.set_max_priority_fee_per_gas(max_priority_fee_per_gas);
        self
    }

    /// Get the max fee per blob gas for the transaction.
    fn max_fee_per_blob_gas(&self) -> Option<u128>;

    /// Set the max fee per blob gas  for the transaction.
    fn set_max_fee_per_blob_gas(&mut self, max_fee_per_blob_gas: u128);

    /// Builder-pattern method for setting max fee per blob gas .
    fn with_max_fee_per_blob_gas(mut self, max_fee_per_blob_gas: u128) -> Self {
        self.set_max_fee_per_blob_gas(max_fee_per_blob_gas);
        self
    }

    /// Get the gas limit for the transaction.
    fn gas_limit(&self) -> Option<u128>;

    /// Set the gas limit for the transaction.
    fn set_gas_limit(&mut self, gas_limit: u128);

    /// Builder-pattern method for setting the gas limit.
    fn with_gas_limit(mut self, gas_limit: u128) -> Self {
        self.set_gas_limit(gas_limit);
        self
    }

    /// Get the EIP-2930 access list for the transaction.
    fn access_list(&self) -> Option<&AccessList>;

    /// Sets the EIP-2930 access list.
    fn set_access_list(&mut self, access_list: AccessList);

    /// Builder-pattern method for setting the access list.
    fn with_access_list(mut self, access_list: AccessList) -> Self {
        self.set_access_list(access_list);
        self
    }

    /// Gets the EIP-4844 blob sidecar of the transaction.
    fn blob_sidecar(&self) -> Option<&BlobTransactionSidecar>;

    /// Sets the EIP-4844 blob sidecar of the transaction.
    ///
    /// Note: This will also set the versioned blob hashes accordingly:
    /// [BlobTransactionSidecar::versioned_hashes]
    fn set_blob_sidecar(&mut self, sidecar: BlobTransactionSidecar);

    /// Builder-pattern method for setting the EIP-4844 blob sidecar of the transaction.
    fn with_blob_sidecar(mut self, sidecar: BlobTransactionSidecar) -> Self {
        self.set_blob_sidecar(sidecar);
        self
    }

    /// True if the builder contains all necessary information to be submitted
    /// to the `eth_sendTransaction` endpoint.
    fn can_submit(&self) -> bool;

    /// True if the builder contains all necessary information to be built into
    /// a valid transaction.
    fn can_build(&self) -> bool;

    /// Build an unsigned, but typed, transaction.
    fn build_unsigned(self) -> BuilderResult<N::UnsignedTx>;

    /// Build a signed transaction.
    fn build<S: NetworkSigner<N>>(
        self,
        signer: &S,
    ) -> impl_future!(<Output = BuilderResult<N::TxEnvelope>>);
}
