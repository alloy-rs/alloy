use alloy_primitives::{Address, BlockHash, Bytes, TxHash, U256};
use alloy_serde::WithOtherFields;

/// A receipt response.
///
/// This is distinct from [`TxReceipt`], since this is for JSON-RPC receipts.
///
/// [`TxReceipt`]: alloy_consensus::TxReceipt
pub trait ReceiptResponse {
    /// Address of the created contract, or `None` if the transaction was not a deployment.
    fn contract_address(&self) -> Option<Address>;

    /// Status of the transaction.
    ///
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
    /// [`TxReceipt::status_or_post_state`]: alloy_consensus::TxReceipt::status_or_post_state
    fn status(&self) -> bool;

    /// Hash of the block this transaction was included within.
    fn block_hash(&self) -> Option<BlockHash>;

    /// Number of the block this transaction was included within.
    fn block_number(&self) -> Option<u64>;
}

/// Transaction Response
///
/// This is distinct from [`Transaction`], since this is a JSON-RPC response.
///
/// [`Transaction`]: alloy_consensus::Transaction
pub trait TransactionResponse {
    /// Hash of the transaction
    #[doc(alias = "transaction_hash")]
    fn tx_hash(&self) -> TxHash;

    /// Sender of the transaction
    fn from(&self) -> Address;

    /// Recipient of the transaction
    fn to(&self) -> Option<Address>;

    /// Transferred value
    fn value(&self) -> U256;

    /// Gas limit
    fn gas(&self) -> u128;

    /// Input data
    #[doc(alias = "calldata")]
    fn input(&self) -> &Bytes;
}

impl<T: TransactionResponse> TransactionResponse for WithOtherFields<T> {
    fn tx_hash(&self) -> TxHash {
        self.inner.tx_hash()
    }

    fn from(&self) -> Address {
        self.inner.from()
    }

    fn to(&self) -> Option<Address> {
        self.inner.to()
    }

    fn value(&self) -> U256 {
        self.inner.value()
    }

    fn gas(&self) -> u128 {
        self.inner.gas()
    }

    fn input(&self) -> &Bytes {
        self.inner.input()
    }
}

impl<T: ReceiptResponse> ReceiptResponse for WithOtherFields<T> {
    fn contract_address(&self) -> Option<Address> {
        self.inner.contract_address()
    }

    fn status(&self) -> bool {
        self.inner.status()
    }

    fn block_hash(&self) -> Option<BlockHash> {
        self.inner.block_hash()
    }

    fn block_number(&self) -> Option<u64> {
        self.inner.block_number()
    }
}
