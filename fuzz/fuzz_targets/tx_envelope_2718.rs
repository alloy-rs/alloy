#![no_main]

use alloy_consensus::TxEnvelope;
use alloy_eips::eip2718::Decodable2718;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut buf = data;
    if let Ok(tx) = TxEnvelope::decode_2718(&mut buf) {
        // Roundtrip: any successfully decoded envelope must re-encode to a
        // byte sequence we can decode back to an equal value.
        let encoded = alloy_eips::eip2718::Encodable2718::encoded_2718(&tx);
        let mut redecode = encoded.as_slice();
        let tx2 = TxEnvelope::decode_2718(&mut redecode)
            .expect("re-decode of self-encoded envelope must succeed");
        assert_eq!(tx, tx2, "envelope roundtrip mismatch");
    }
});
