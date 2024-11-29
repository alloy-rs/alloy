//! Helper function for calculating Merkle proofs and hashes.

use crate::{Header, ReceiptWithBloom, RlpReceipt, EMPTY_OMMER_ROOT_HASH};
use alloc::vec::Vec;
use alloy_eips::{eip2718::Encodable2718, eip4895::Withdrawal};
use alloy_primitives::{keccak256, B256};
use alloy_trie::root::{ordered_trie_root, ordered_trie_root_with_encoder};

/// Calculate a transaction root.
///
/// `(rlp(index), encoded(tx))` pairs.
pub fn calculate_transaction_root<T, E>(transactions: &[T]) -> B256
where
    T: Encodable2718,
{
    ordered_trie_root_with_encoder(transactions, |tx: &T, buf| tx.encode_2718(buf))
}

/// Calculates the root hash of the withdrawals.
pub fn calculate_withdrawals_root(withdrawals: &[Withdrawal]) -> B256 {
    ordered_trie_root(withdrawals)
}

/// Calculates the root hash for ommer/uncle headers.
pub fn calculate_ommers_root(ommers: &[Header]) -> B256 {
    // Check if `ommers` list is empty
    if ommers.is_empty() {
        return EMPTY_OMMER_ROOT_HASH;
    }
    // RLP Encode
    let mut ommers_rlp = Vec::new();
    alloy_rlp::encode_list(ommers, &mut ommers_rlp);
    keccak256(ommers_rlp)
}


/// Calculates the receipt root for a header.
pub fn calculate_receipt_root<T>(receipts: &[ReceiptWithBloom<T>]) -> B256
where T: Encodable2718
{
    // TODO - Implement this function according to https://github.com/paradigmxyz/reth/blob/b09c345257cda4a88e8e347654e946a20f9e5cb7/crates/primitives/src/proofs.rs#L27-L27
    // ordered_trie_root_with_encoder(receipts, |r, buf| r.encode_inner(buf, false))
    todo!()
}


#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{bloom, Address, Log, LogData};
    use crate::{Eip658Value, Receipt};

    fn check_receipt_root_optimism() {
        let logs = vec![Log {
            address: Address::ZERO,
            data: LogData::new_unchecked(vec![], Default::default()),
        }];
        let logs_bloom = bloom!("00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001");
        let receipt = ReceiptWithBloom {
            receipt: Receipt {
                status: Eip658Value::success(),
                cumulative_gas_used: 102068,
                logs,
            },
            logs_bloom,
        };
        let receipt = vec![receipt];
        // let root = calculate_receipt_root(&receipt);
        // assert_eq!(root, b256!("fe70ae4a136d98944951b2123859698d59ad251a381abc9960fa81cae3d0d4a0"));
    }
}