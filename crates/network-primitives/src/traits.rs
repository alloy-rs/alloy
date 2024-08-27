use alloy_primitives::{Address, BlockHash, Bytes, TxHash, U256};
use alloy_serde::WithOtherFields;

use crate::BlockTransactions;

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
}

/// Header JSON-RPC response.
pub trait HeaderResponse {
    /// Block hash
    fn hash(&self) -> BlockHash;

    /// Block number
    fn number(&self) -> u64;

    /// Block timestamp
    fn timestamp(&self) -> u64;

    /// Extra data
    fn extra_data(&self) -> &Bytes;

    /// Base fee per unit of gas (If EIP-1559 is supported)
    fn base_fee_per_gas(&self) -> Option<u128>;

    /// Blob fee for the next block (if EIP-4844 is supported)
    fn next_block_blob_fee(&self) -> Option<u128>;
}

/// Block JSON-RPC response.
pub trait BlockResponse {
    /// Header type
    type Header;
    /// Transaction type
    type Transaction;

    /// Block header
    fn header(&self) -> &Self::Header;

    /// Block transactions
    fn transactions(&self) -> &BlockTransactions<Self::Transaction>;

    /// Mutable reference to block transactions
    fn transactions_mut(&mut self) -> &mut BlockTransactions<Self::Transaction>;
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

impl<T: BlockResponse> BlockResponse for WithOtherFields<T> {
    type Header = T::Header;
    type Transaction = T::Transaction;

    fn header(&self) -> &Self::Header {
        self.inner.header()
    }

    fn transactions(&self) -> &BlockTransactions<Self::Transaction> {
        self.inner.transactions()
    }

    fn transactions_mut(&mut self) -> &mut BlockTransactions<Self::Transaction> {
        self.inner.transactions_mut()
    }
}

impl<T: HeaderResponse> HeaderResponse for WithOtherFields<T> {
    fn hash(&self) -> BlockHash {
        self.inner.hash()
    }

    fn number(&self) -> u64 {
        self.inner.number()
    }

    fn timestamp(&self) -> u64 {
        self.inner.timestamp()
    }

    fn extra_data(&self) -> &Bytes {
        self.inner.extra_data()
    }

    fn base_fee_per_gas(&self) -> Option<u128> {
        self.inner.base_fee_per_gas()
    }

    fn next_block_blob_fee(&self) -> Option<u128> {
        self.inner.next_block_blob_fee()
    }
}
