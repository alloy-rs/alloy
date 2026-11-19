//! Geth 4byte tracer types.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The 4byte tracer response object.
///
/// <https://github.com/ethereum/go-ethereum/blob/91cb6f863a965481e51d5d9c0e5ccd54796fd967/eth/tracers/native/4byte.go#L48>
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FourByteFrame(pub BTreeMap<String, u64>);

#[cfg(test)]
mod tests {
    use crate::geth::*;
    use similar_asserts::assert_eq;

    #[test]
    fn test_serialize_four_byte_trace() {
        let mut opts = GethDebugTracingCallOptions::default();
        opts.tracing_options.tracer =
            Some(GethDebugTracerType::BuiltInTracer(GethDebugBuiltInTracerType::FourByteTracer));

        assert_eq!(serde_json::to_string(&opts).unwrap(), r#"{"tracer":"4byteTracer"}"#);
    }
}
