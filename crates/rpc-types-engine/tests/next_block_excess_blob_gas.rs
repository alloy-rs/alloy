//! `ExecutionPayload::next_block_excess_blob_gas` must not panic on an untrusted base fee that does
//! not fit in a `u64`.

use alloy_eips::eip7840::BlobParams;
use alloy_primitives::{Address, Bloom, Bytes, B256, U256};
use alloy_rpc_types_engine::{
    ExecutionPayload, ExecutionPayloadV1, ExecutionPayloadV2, ExecutionPayloadV3,
};

#[test]
fn next_block_excess_blob_gas_saturates_base_fee() {
    let blob_params = BlobParams::osaka();
    let blob_gas_used = blob_params.max_blob_gas_per_block();
    let payload = ExecutionPayload::V3(ExecutionPayloadV3 {
        payload_inner: ExecutionPayloadV2 {
            payload_inner: ExecutionPayloadV1 {
                parent_hash: B256::ZERO,
                fee_recipient: Address::ZERO,
                state_root: B256::ZERO,
                receipts_root: B256::ZERO,
                logs_bloom: Bloom::ZERO,
                prev_randao: B256::ZERO,
                block_number: 1,
                gas_limit: 30_000_000,
                gas_used: 0,
                timestamp: 1,
                extra_data: Bytes::new(),
                base_fee_per_gas: U256::MAX,
                block_hash: B256::ZERO,
                transactions: Vec::new(),
            },
            withdrawals: Vec::new(),
        },
        blob_gas_used,
        excess_blob_gas: 0,
    });

    assert_eq!(
        payload.next_block_excess_blob_gas(blob_params),
        Some(blob_params.next_block_excess_blob_gas_osaka(0, blob_gas_used, u64::MAX))
    );
}
