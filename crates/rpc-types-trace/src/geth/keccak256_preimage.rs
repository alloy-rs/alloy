//! Geth keccak256 preimage tracer types.

use alloy_primitives::{Bytes, B256};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The `keccak256PreimageTracer` response object.
///
/// Maps the hash of every `KECCAK256` operation executed during the transaction to the memory
/// that was hashed.
///
/// <https://github.com/ethereum/go-ethereum/blob/6fe8b63d41be9218808adb9b25d6b21c55b37b54/eth/tracers/native/keccak256_preimage.go>
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Keccak256PreimageFrame(pub BTreeMap<B256, Bytes>);

#[cfg(test)]
mod tests {
    use crate::geth::*;
    use alloy_primitives::{b256, bytes};
    use similar_asserts::assert_eq;

    #[test]
    fn test_serialize_keccak256_preimage_trace() {
        let mut opts = GethDebugTracingCallOptions::default();
        opts.tracing_options.tracer = Some(GethDebugTracerType::BuiltInTracer(
            GethDebugBuiltInTracerType::Keccak256PreimageTracer,
        ));

        assert_eq!(
            serde_json::to_string(&opts).unwrap(),
            r#"{"tracer":"keccak256PreimageTracer"}"#
        );
    }

    #[test]
    fn test_deserialize_keccak256_preimage_tracer_name() {
        let tracer: GethDebugTracerType =
            serde_json::from_str(r#""keccak256PreimageTracer""#).unwrap();

        assert!(!tracer.is_js());
        assert_eq!(tracer.as_str(), "keccak256PreimageTracer");
    }

    #[test]
    fn test_deserialize_keccak256_preimage_frame() {
        // keccak256(0x01) and keccak256(32 zero bytes)
        let json = r#"{
            "0x5fe7f977e71dba2ea1a68e21057beebb9be2ac30c6410aa38d4f3fbe41dcffd2": "0x01",
            "0x290decd9548b62a8d60345a988386fc84ba6bc95484008f6362f93160ef3e563": "0x0000000000000000000000000000000000000000000000000000000000000000"
        }"#;

        let trace: GethTrace = serde_json::from_str(json).unwrap();
        let frame = trace.try_into_keccak256_preimage_frame().unwrap();

        assert_eq!(frame.0.len(), 2);
        assert_eq!(
            frame.0[&b256!("0x5fe7f977e71dba2ea1a68e21057beebb9be2ac30c6410aa38d4f3fbe41dcffd2")],
            bytes!("01")
        );
        for (hash, preimage) in &frame.0 {
            assert_eq!(*hash, alloy_primitives::keccak256(preimage));
        }
    }
}
