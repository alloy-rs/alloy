#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

mod block;
pub use block::{AnyRpcBlock, AnyRpcHeader};

mod transaction;
pub use transaction::{AnyRpcTransaction, AnyTransactionReceipt, AnyTransactionRequest};

pub use alloy_consensus_any::{AnyReceiptEnvelope, AnyTxEnvelope};

#[cfg(test)]
mod tests {
    use alloy_consensus::Transaction;
    use alloy_consensus_any::AnyTxType;

    use super::*;

    #[test]
    fn test_serde_op_deposit() {
        let input = r#"{
            "blockHash": "0xef664d656f841b5ad6a2b527b963f1eb48b97d7889d742f6cbff6950388e24cd",
            "blockNumber": "0x73a78fd",
            "depositReceiptVersion": "0x1",
            "from": "0x36bde71c97b33cc4729cf772ae268934f7ab70b2",
            "gas": "0xc27a8",
            "gasPrice": "0x521",
            "hash": "0x0bf1845c5d7a82ec92365d5027f7310793d53004f3c86aa80965c67bf7e7dc80",
            "input": "0xd764ad0b000100000000000000000000000000000000000000000000000000000001cf5400000000000000000000000099c9fc46f92e8a1c0dec1b1747d010903e884be100000000000000000000000042000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000007a12000000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000000e40166a07a0000000000000000000000000994206dfe8de6ec6920ff4d779b0d950605fb53000000000000000000000000d533a949740bb3306d119cc777fa900ba034cd52000000000000000000000000ca74f404e0c7bfa35b13b511097df966d5a65597000000000000000000000000ca74f404e0c7bfa35b13b511097df966d5a65597000000000000000000000000000000000000000000000216614199391dbba2ba00000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "mint": "0x0",
            "nonce": "0x74060",
            "r": "0x0",
            "s": "0x0",
            "sourceHash": "0x074adb22f2e6ed9bdd31c52eefc1f050e5db56eb85056450bccd79a6649520b3",
            "to": "0x4200000000000000000000000000000000000007",
            "transactionIndex": "0x1",
            "type": "0x7e",
            "v": "0x0",
            "value": "0x0"
        }"#;

        let tx: AnyRpcTransaction = serde_json::from_str(input).unwrap();

        let AnyTxEnvelope::Unknown(inner) = tx.inner.inner.clone() else {
            panic!("expected unknown envelope");
        };

        assert_eq!(inner.inner.ty, AnyTxType(126));
        assert!(inner.inner.fields.contains_key("input"));
        assert!(inner.inner.fields.contains_key("mint"));
        assert!(inner.inner.fields.contains_key("sourceHash"));
        assert_eq!(inner.gas_limit(), 796584);
        assert_eq!(inner.gas_price(), Some(1313));
        assert_eq!(inner.nonce(), 475232);

        let roundrip_tx: AnyRpcTransaction =
            serde_json::from_str(&serde_json::to_string(&tx).unwrap()).unwrap();

        assert_eq!(tx, roundrip_tx);
    }
}
