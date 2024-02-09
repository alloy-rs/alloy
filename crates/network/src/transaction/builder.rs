use alloy_primitives::{Bytes, ChainId, U256};

use crate::{Network, TxKind};

use super::signer::NetworkSigner;

#[derive(Debug, thiserror::Error)]
/// Error type for transaction builders.
pub enum BuilderError {
    /// A required key is missing.
    #[error("A required key is missing: {0}")]
    MissingKey(&'static str),

    /// Signer cannot produce signature type required for transaction.
    #[error("Signer cannot produce signature type required for transaction")]
    UnsupportedSignatureType,

    /// Signer Error
    #[error(transparent)]
    Signer(#[from] alloy_signer::Error),

    /// A custom error.
    #[error("{0}")]
    Custom(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl BuilderError {
    /// Instantiate a custom error.
    pub fn custom<E>(e: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Custom(Box::new(e))
    }
}

type Result<T, E = BuilderError> = std::result::Result<T, E>;

/// Transaction Builder for a network
pub trait Builder<N: Network>: Sized + Send + Sync + 'static {
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

    /// Get the recipient for the transaction.
    fn to(&self) -> Option<TxKind>;

    /// Set the recipient for the transaction.
    fn set_to(&mut self, to: TxKind);

    /// Builder-pattern method for setting the recipient.
    fn with_to(mut self, to: TxKind) -> Self {
        self.set_to(to);
        self
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

    /// Get the gas price for the transaction.
    fn gas_price(&self) -> Option<u128>;

    /// Set the gas price for the transaction.
    fn set_gas_price(&mut self, gas_price: u128);

    /// Builder-pattern method for setting the gas price.
    fn with_gas_price(mut self, gas_price: u128) -> Self {
        self.set_gas_price(gas_price);
        self
    }

    /// Get the gas limit for the transaction.
    fn gas_limit(&self) -> Option<u64>;

    /// Set the gas limit for the transaction.
    fn set_gas_limit(&mut self, gas_limit: u64);

    /// Builder-pattern method for setting the gas limit.
    fn with_gas_limit(mut self, gas_limit: u64) -> Self {
        self.set_gas_limit(gas_limit);
        self
    }

    /// Build an unsigned, but typed, transaction.
    fn build_unsigned(self) -> Result<N::UnsignedTx>;

    /// Build a signed transaction.
    fn build<S: NetworkSigner<N>>(self, signer: &S) -> Result<N::TxEnvelope>;
}
