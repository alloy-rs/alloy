use alloc::vec::Vec;
use alloy_primitives::Bytes;
use alloy_rlp::{Encodable, Header};
use serde::{Deserialize, Serialize};

/// Represents the execution witness of a block. Contains lists of required preimages and
/// headers used during execution and verification.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionWitness {
    /// List of all hashed trie nodes preimages that were required during the execution of
    /// the block, including during state root recomputation.
    pub state: Vec<Bytes>,
    /// List of all contract codes (created / accessed) preimages that were required during
    /// the execution of the block, including during state root recomputation.
    pub codes: Vec<Bytes>,
    /// List of all hashed account and storage keys (addresses and slots) preimages
    /// (unhashed account addresses and storage slots, respectively) that were required during
    /// the execution of the block.
    pub keys: Vec<Bytes>,
    /// RLP-encoded block headers required for proving correctness of stateless execution.
    ///
    /// This collection stores block headers needed to verify:
    /// - State reads are correct (i.e. the code and accounts are correct wrt the pre-state root)
    /// - BLOCKHASH opcode execution results are correct
    ///
    /// ## Why this field will be empty in the future
    ///
    /// This field is expected to be empty in the future because:
    /// - EIP-2935 (Prague) will include block hashes directly in the state
    /// - Verkle/Delayed execution will change the block structure to contain the pre-state root
    ///   instead of the post-state root.
    ///
    /// Once both of these upgrades have been implemented, this field will be empty
    /// moving forward because the data that this was proving will either be in the
    /// current block or in the state.
    ///
    /// ## State Read Verification
    ///
    /// To verify state reads are correct, we need the pre-state root of the current block,
    /// which is (currently) equal to the post-state root of the previous block. We therefore
    /// need the previous block's header in order to prove that the state reads are correct.
    ///
    /// Note: While the pre-state root is located in the previous block, this field
    /// will always have one or more items.
    ///
    /// ## BLOCKHASH Opcode Verification
    ///
    /// The BLOCKHASH opcode returns the block hash for a given block number, but it
    /// only works for the 256 most recent blocks, not including the current block.
    /// To verify that a block hash is indeed correct wrt the BLOCKHASH opcode
    /// and not an arbitrary set of block hashes, we need a contiguous set of
    /// block headers starting from the current block.
    ///
    /// ### Example
    ///
    /// Consider a blockchain at block 200, and inside of block 200, a transaction
    /// calls BLOCKHASH(100):
    /// - This is valid because block 100 is within the 256-block lookback window
    /// - To verify this, we need all of the headers from block 100 through block 200
    /// - These headers form a chain proving the correctness of block 100's hash.
    ///
    /// The naive way to construct the headers would be to unconditionally include the last
    /// 256 block headers. However note, we may not need all 256, like in the example above.
    pub headers: Vec<Bytes>,
}

impl ExecutionWitness {
    /// RLP-encodes the witness in Geth's external witness format: `[headers, codes, state, keys]`.
    ///
    /// Headers are embedded as already RLP-encoded lists, while codes and state nodes are encoded
    /// as byte strings. The debug API's unhashed account and storage keys are omitted by encoding
    /// an empty keys list. The order of headers, codes, and state nodes is preserved.
    ///
    /// The headers must contain valid RLP-encoded block headers; they are not validated here.
    ///
    /// See <https://github.com/ethereum/go-ethereum/blob/7538039f06792da46a91165e7eda98917edcfde2/core/stateless/encoding.go>.
    pub fn encode_witness(&self) -> Bytes {
        let headers = Header {
            list: true,
            payload_length: self.headers.iter().map(|header| header.len()).sum(),
        };
        let keys = Header { list: true, payload_length: 0 };
        let witness = Header {
            list: true,
            payload_length: headers.length_with_payload()
                + self.codes.length()
                + self.state.length()
                + keys.length(),
        };
        let mut encoded = Vec::with_capacity(witness.length_with_payload());
        witness.encode(&mut encoded);
        headers.encode(&mut encoded);
        for header in &self.headers {
            encoded.extend_from_slice(header);
        }
        self.codes.encode(&mut encoded);
        self.state.encode(&mut encoded);
        keys.encode(&mut encoded);
        encoded.into()
    }

    /// Sets the `headers` field from already RLP-encoded headers.
    pub fn with_rlp_headers(mut self, headers: Vec<Bytes>) -> Self {
        self.headers = headers;
        self
    }

    /// Sets the `headers` field by RLP-encoding each item.
    pub fn with_headers<H: Encodable>(mut self, headers: impl IntoIterator<Item = H>) -> Self {
        self.headers = headers
            .into_iter()
            .map(|header| {
                let mut buf = Vec::new();
                header.encode(&mut buf);
                buf.into()
            })
            .collect();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloy_primitives::hex;

    #[test]
    fn encode_witness_nested_headers_and_empty_keys() {
        let witness = ExecutionWitness {
            headers: vec![Bytes::from_static(&hex!("c101")), Bytes::from_static(&hex!("c102"))],
            codes: vec![Bytes::from_static(b"code")],
            state: vec![Bytes::from_static(b"node")],
            keys: vec![Bytes::from_static(b"debug-only")],
        };
        assert_eq!(
            witness.encode_witness().as_ref(),
            &hex!("d2c4c101c102c584636f6465c5846e6f6465c0")
        );
    }

    #[test]
    fn encode_witness_long_rlp_payload() {
        let witness =
            ExecutionWitness { codes: vec![Bytes::from(vec![0x7f; 56])], ..Default::default() };
        let expected = [&hex!("f83fc0f83ab838")[..], &[0x7f; 56], &hex!("c0c0")].concat();
        assert_eq!(witness.encode_witness().as_ref(), expected);
    }

    #[test]
    fn encode_empty_witness() {
        assert_eq!(ExecutionWitness::default().encode_witness().as_ref(), &hex!("c4c0c0c0c0"));
    }
}
