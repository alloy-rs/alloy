use crate::{Network, ReceiptResponse, TransactionResponse};

mod builder;

mod signer;
pub use signer::EthereumSigner;

/// Types for a mainnet-like Ethereum network.
#[derive(Clone, Copy, Debug)]
pub struct Ethereum {
    _private: (),
}

impl Network for Ethereum {
    type TxType = alloy_consensus::TxType;

    type TxEnvelope = alloy_consensus::TxEnvelope;

    type UnsignedTx = alloy_consensus::TypedTransaction;

    type ReceiptEnvelope = alloy_consensus::ReceiptEnvelope;

    type Header = alloy_consensus::Header;

    type TransactionRequest = alloy_rpc_types_eth::transaction::TransactionRequest;

    type TransactionResponse = alloy_rpc_types_eth::Transaction;

    type ReceiptResponse = alloy_rpc_types_eth::TransactionReceipt;

    type HeaderResponse = alloy_rpc_types_eth::Header;
}

impl ReceiptResponse for alloy_rpc_types_eth::TransactionReceipt {
    fn contract_address(&self) -> Option<alloy_primitives::Address> {
        self.contract_address
    }

    /// ## Note
    ///
    /// Caution must be taken when using this method for deep-historical
    /// receipts, as it may not accurately reflect the status of the
    /// transaction. The transaction status is not knowable from the receipt
    /// for transactions before [EIP-658].
    ///
    /// This can be handled using [`TxReceipt::status_or_post_state`].
    ///
    /// [EIP-658]: https://eips.ethereum.org/EIPS/eip-658
    fn status(&self) -> bool {
        self.inner.status()
    }
}

impl TransactionResponse for alloy_rpc_types_eth::Transaction {
    #[doc(alias = "transaction_hash")]
    fn tx_hash(&self) -> alloy_primitives::B256 {
        self.hash
    }

    fn from(&self) -> alloy_primitives::Address {
        self.from
    }

    fn to(&self) -> Option<alloy_primitives::Address> {
        self.to
    }

    fn value(&self) -> alloy_primitives::U256 {
        self.value
    }

    fn gas(&self) -> u128 {
        self.gas
    }
}
