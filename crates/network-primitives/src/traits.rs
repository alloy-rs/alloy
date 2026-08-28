use crate::{BlockTransactions, InclusionInfo};
use alloy_consensus::{BlockHeader, Transaction};
use alloy_eips::BlockNumHash;
use alloy_primitives::{Address, BlockHash, TxHash, B256};
use alloy_serde::WithOtherFields;

/// Error returned when a transaction failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionFailedError {
    /// Hash of the failed transaction.
    pub transaction_hash: TxHash,
}

impl core::fmt::Display for TransactionFailedError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Transaction {} failed", self.transaction_hash)
    }
}

impl core::error::Error for TransactionFailedError {}

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
    ///
    /// [EIP-658]: https://eips.ethereum.org/EIPS/eip-658
    fn status(&self) -> bool;

    /// Hash of the block this transaction was included within.
    fn block_hash(&self) -> Option<BlockHash>;

    /// Number of the block this transaction was included within.
    fn block_number(&self) -> Option<u64>;

    /// Returns the [`BlockNumHash`] of the block this transaction was mined in.
    ///
    /// Returns `None` if either component is absent, as is normally the case for a pending
    /// transaction.
    fn block_hash_num(&self) -> Option<BlockNumHash> {
        Some(BlockNumHash::new(self.block_number()?, self.block_hash()?))
    }

    /// Transaction Hash.
    fn transaction_hash(&self) -> TxHash;

    /// Index within the block.
    fn transaction_index(&self) -> Option<u64>;

    /// Gas used by this transaction alone.
    fn gas_used(&self) -> u64;

    /// Effective gas price.
    fn effective_gas_price(&self) -> u128;

    /// Returns the execution-gas cost in wei: `gas_used * effective_gas_price`.
    ///
    /// This excludes blob-gas charges, transferred value, and network-specific fee components;
    /// it is not the sender's total balance change. The ordinary `u128` multiplication can
    /// overflow, panicking when overflow checks are enabled or wrapping otherwise.
    fn cost(&self) -> u128 {
        self.gas_used() as u128 * self.effective_gas_price()
    }

    /// Blob gas used by the eip-4844 transaction.
    fn blob_gas_used(&self) -> Option<u64>;

    /// Blob gas price paid by the eip-4844 transaction.
    fn blob_gas_price(&self) -> Option<u128>;

    /// Address of the sender.
    fn from(&self) -> Address;

    /// Address of the receiver, or `None` for contract creation.
    fn to(&self) -> Option<Address>;

    /// Returns the gas used in the block up to and including this transaction.
    fn cumulative_gas_used(&self) -> u64;

    /// Returns the post-transaction state root carried by pre-[EIP-658] receipts.
    ///
    /// [EIP-658] replaced this value with a status code, so post-Byzantium receipts normally
    /// return `None`.
    ///
    /// [EIP-658]: https://eips.ethereum.org/EIPS/eip-658
    fn state_root(&self) -> Option<B256>;

    /// Ensures the transaction was successful, returning its hash in the error if it failed.
    ///
    /// This does not recover revert data and has the same pre-EIP-658 limitation as
    /// [`Self::status`].
    fn ensure_success(&self) -> Result<(), TransactionFailedError> {
        if self.status() {
            Ok(())
        } else {
            Err(TransactionFailedError { transaction_hash: self.transaction_hash() })
        }
    }
}

/// Mutable access to the fields of a [`ReceiptResponse`].
///
/// This is a separate trait rather than additional methods on [`ReceiptResponse`] because that
/// trait is implemented outside of alloy for network-specific receipt types, and adding required
/// methods to it would be a breaking change for every external implementor. Implementing this
/// trait is opt-in and can be done independently.
///
/// The primary use case is patching a receipt whose reported `contractAddress` is absent because
/// the deployment happened through a factory, for example a CREATE2 deployment: the node reports
/// no contract address, but the caller knows the deployed address and needs to write it back.
pub trait ReceiptResponseMut: ReceiptResponse {
    /// Sets the address of the created contract, or `None` if the transaction was not a
    /// deployment.
    ///
    /// This takes an [`Option`] to mirror [`ReceiptResponse::contract_address`], so that the
    /// field can also be cleared.
    fn set_contract_address(&mut self, contract_address: Option<Address>);
}

/// Transaction JSON-RPC response. Aggregates transaction data with its block and signer context.
///
/// The optional fee accessors split fixed-price and dynamic-fee consensus caps by transaction type
/// and share names with differently typed methods on [`Transaction`]. Use trait-qualified calls
/// such as `TransactionResponse::max_fee_per_gas(tx)` when the distinction matters.
pub trait TransactionResponse: Transaction {
    /// Hash of the transaction
    #[doc(alias = "transaction_hash")]
    fn tx_hash(&self) -> TxHash;

    /// Returns the hash of the block this transaction was mined in.
    ///
    /// Returns `None` when absent from the response, normally for a pending transaction.
    fn block_hash(&self) -> Option<BlockHash>;

    /// Returns the number of the block this transaction was mined in.
    ///
    /// Returns `None` when absent from the response, normally for a pending transaction.
    fn block_number(&self) -> Option<u64>;

    /// Returns the [`BlockNumHash`] of the block this transaction was mined in.
    ///
    /// Returns `None` if either component is absent, as is normally the case for a pending
    /// transaction.
    fn block_hash_num(&self) -> Option<BlockNumHash> {
        Some(BlockNumHash::new(self.block_number()?, self.block_hash()?))
    }

    /// Transaction Index
    fn transaction_index(&self) -> Option<u64>;

    /// Sender of the transaction
    fn from(&self) -> Address;

    /// Returns the fixed gas price for standard Ethereum transaction type IDs 0 and 1.
    ///
    /// The default returns the consensus fee cap for those type IDs and `None` for IDs 2 and
    /// above. Networks with different type numbering or RPC `gasPrice` semantics must override
    /// this method.
    fn gas_price(&self) -> Option<u128> {
        if self.ty() < 2 {
            return Some(Transaction::max_fee_per_gas(self));
        }
        None
    }

    /// Returns the maximum fee per gas for standard Ethereum transaction type IDs 2 and above.
    ///
    /// The default returns `None` for type IDs 0 and 1 and the consensus fee cap for later IDs.
    /// Networks with different type numbering must override this method.
    fn max_fee_per_gas(&self) -> Option<u128> {
        if self.ty() < 2 {
            return None;
        }
        Some(Transaction::max_fee_per_gas(self))
    }

    /// Transaction type format for RPC. This field is included since eip-2930.
    fn transaction_type(&self) -> Option<u8> {
        match self.ty() {
            0 => None,
            ty => Some(ty),
        }
    }

    /// Returns the [`InclusionInfo`] if the transaction has been included.
    ///
    /// Returns `None` if this transaction is still pending (missing block number, hash, or index).
    fn inclusion_info(&self) -> Option<InclusionInfo> {
        Some(InclusionInfo {
            block_hash: self.block_hash()?,
            block_number: self.block_number()?,
            transaction_index: self.transaction_index()?,
        })
    }
}

/// Header JSON-RPC response.
pub trait HeaderResponse: BlockHeader {
    /// Block hash
    fn hash(&self) -> BlockHash;

    /// Returns the [`BlockNumHash`] of this header.
    fn num_hash(&self) -> BlockNumHash {
        BlockNumHash::new(self.number(), self.hash())
    }
}

/// Block JSON-RPC response.
pub trait BlockResponse {
    /// Concrete RPC header representation.
    type Header;
    /// Full-transaction representation used by [`BlockTransactions::Full`].
    type Transaction: TransactionResponse;

    /// Block header
    fn header(&self) -> &Self::Header;

    /// Block transactions
    fn transactions(&self) -> &BlockTransactions<Self::Transaction>;

    /// Returns a mutable reference to the block transactions.
    ///
    /// Mutating transactions does not recompute or validate header fields such as the transaction
    /// root.
    fn transactions_mut(&mut self) -> &mut BlockTransactions<Self::Transaction>;

    /// Returns flattened chain- or client-specific RPC fields when they were retained.
    ///
    /// The default is `None`. A [`WithOtherFields`] response returns `Some` even when its map is
    /// empty.
    fn other_fields(&self) -> Option<&alloy_serde::OtherFields> {
        None
    }
}

impl<T: TransactionResponse> TransactionResponse for WithOtherFields<T> {
    fn tx_hash(&self) -> TxHash {
        self.inner.tx_hash()
    }

    fn block_hash(&self) -> Option<BlockHash> {
        self.inner.block_hash()
    }

    fn block_number(&self) -> Option<u64> {
        self.inner.block_number()
    }

    fn transaction_index(&self) -> Option<u64> {
        self.inner.transaction_index()
    }

    fn from(&self) -> Address {
        self.inner.from()
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

    fn transaction_hash(&self) -> TxHash {
        self.inner.transaction_hash()
    }

    fn transaction_index(&self) -> Option<u64> {
        self.inner.transaction_index()
    }

    fn gas_used(&self) -> u64 {
        self.inner.gas_used()
    }

    fn effective_gas_price(&self) -> u128 {
        self.inner.effective_gas_price()
    }

    fn blob_gas_used(&self) -> Option<u64> {
        self.inner.blob_gas_used()
    }

    fn blob_gas_price(&self) -> Option<u128> {
        self.inner.blob_gas_price()
    }

    fn from(&self) -> Address {
        self.inner.from()
    }

    fn to(&self) -> Option<Address> {
        self.inner.to()
    }

    fn cumulative_gas_used(&self) -> u64 {
        self.inner.cumulative_gas_used()
    }

    fn state_root(&self) -> Option<B256> {
        self.inner.state_root()
    }
}

impl<T: ReceiptResponseMut> ReceiptResponseMut for WithOtherFields<T> {
    fn set_contract_address(&mut self, contract_address: Option<Address>) {
        self.inner.set_contract_address(contract_address);
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

    fn other_fields(&self) -> Option<&alloy_serde::OtherFields> {
        Some(&self.other)
    }
}

impl<T: HeaderResponse> HeaderResponse for WithOtherFields<T> {
    fn hash(&self) -> BlockHash {
        self.inner.hash()
    }
}
