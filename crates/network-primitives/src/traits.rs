use alloy_consensus::Signed;
use alloy_primitives::{Address, BlockHash, Bytes, TxHash, B256, U256};
use alloy_serde::WithOtherFields;

/// Receipt JSON-RPC response.
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
    fn status(&self) -> bool;

    /// Hash of the block this transaction was included within.
    fn block_hash(&self) -> Option<BlockHash>;

    /// Number of the block this transaction was included within.
    fn block_number(&self) -> Option<u64>;
}

/// Transaction JSON-RPC response.
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

    /// Returns the gas price formatted for the RPC response.
    fn gas_price(tx: &impl alloy_consensus::Transaction, base_fee: Option<u64>) -> u128;

    /// Returns the max fee per gas.
    fn max_fee_per_gas(tx: impl alloy_consensus::Transaction) -> Option<u128>;

    /// Assemble from signed transaction.
    fn fill(
        signed_tx: Signed<impl alloy_consensus::Transaction>,
        signer: Address,
        block_hash: Option<B256>,
        block_number: Option<u64>,
        base_fee: Option<u64>,
        transaction_index: Option<usize>,
    ) -> Self;
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

    fn gas_price(tx: &impl alloy_consensus::Transaction, base_fee: Option<u64>) -> u128 {
        T::gas_price(tx, base_fee)
    }

    fn max_fee_per_gas(tx: impl alloy_consensus::Transaction) -> Option<u128> {
        T::max_fee_per_gas(tx)
    }

    fn fill(
        signed_tx: Signed<impl alloy_consensus::Transaction>,
        signer: Address,
        block_hash: Option<B256>,
        block_number: Option<u64>,
        base_fee: Option<u64>,
        transaction_index: Option<usize>,
    ) -> Self {
        Self::new(T::fill(signed_tx, signer, block_hash, block_number, base_fee, transaction_index))
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
