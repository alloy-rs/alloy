#![no_main]

use alloy_eips::eip4844::BlobTransactionSidecar;
use alloy_rlp::{Decodable, Encodable};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut buf = data;
    if let Ok(sidecar) = BlobTransactionSidecar::decode(&mut buf) {
        let mut encoded = Vec::with_capacity(sidecar.length());
        sidecar.encode(&mut encoded);
        let mut redecode = encoded.as_slice();
        let sidecar2 = BlobTransactionSidecar::decode(&mut redecode)
            .expect("re-decode of self-encoded blob sidecar must succeed");
        assert_eq!(sidecar, sidecar2, "blob sidecar roundtrip mismatch");
    }
});
