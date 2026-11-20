#![no_main]

use alloy_consensus::transaction::PooledTransaction;
use alloy_eips::eip2718::Decodable2718;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut buf = data;
    if let Ok(tx) = PooledTransaction::decode_2718(&mut buf) {
        let encoded = alloy_eips::eip2718::Encodable2718::encoded_2718(&tx);
        let mut redecode = encoded.as_slice();
        let tx2 = PooledTransaction::decode_2718(&mut redecode)
            .expect("re-decode of self-encoded pooled tx must succeed");
        assert_eq!(tx, tx2, "pooled transaction roundtrip mismatch");
    }
});
