#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(feature = "arbitrary")]
use rand as _;

use once_cell as _;

pub use alloy_trie::TrieAccount;

/// Represents an TrieAccount in the account trie
#[deprecated(since = "0.7.3", note = "use TrieAccount instead")]
pub type Account = TrieAccount;

/// Generates a test that roundtrips arbitrary values of `$ty` through bincode using the
/// `serde_bincode_compat` type `$compat`.
#[cfg(all(test, feature = "serde", feature = "serde-bincode-compat"))]
macro_rules! bincode_compat_roundtrip_test {
    ($name:ident, $ty:ty, $compat:literal) => {
        #[test]
        fn $name() {
            use arbitrary::Arbitrary;
            use rand::{rngs::StdRng, Rng, SeedableRng};

            #[serde_with::serde_as]
            #[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
            struct Data {
                #[serde_as(as = $compat)]
                value: $ty,
            }

            let mut rng = StdRng::seed_from_u64(0);
            for _ in 0..32 {
                let mut bytes = [0u8; 1024];
                rng.fill(bytes.as_mut_slice());
                let data = Data {
                    value: <$ty>::arbitrary(&mut arbitrary::Unstructured::new(&bytes)).unwrap(),
                };

                let encoded =
                    bincode::serde::encode_to_vec(&data, bincode::config::legacy()).unwrap();
                let (decoded, _) = bincode::serde::decode_from_slice::<Data, _>(
                    &encoded,
                    bincode::config::legacy(),
                )
                .unwrap();
                assert_eq!(decoded, data);
            }
        }
    };
}

mod block;
pub use block::{
    decode_transactions, Block, BlockBody, BlockHeader, EthBlock, GasLimitMismatch, Header,
    HeaderInfo, HeaderRoots,
};

mod indexed;
pub use indexed::Indexed;

pub mod constants;
pub use constants::{EMPTY_OMMER_ROOT_HASH, EMPTY_ROOT_HASH};

mod receipt;
pub use receipt::{
    Eip2718DecodableReceipt, Eip2718EncodableReceipt, Eip658Value, EthereumReceipt,
    FrameReceiptEnvelope, Receipt, ReceiptEnvelope, ReceiptWithBloom, Receipts,
    RlpDecodableReceipt, RlpEncodableReceipt, TxReceipt, TxTy,
};

pub mod size;
pub use size::InMemorySize;

pub mod conditional;
pub mod proofs;

pub mod transaction;
#[cfg(feature = "kzg")]
pub use transaction::BlobTransactionValidationError;
pub use transaction::{
    EthereumTxEnvelope, EthereumTypedTransaction, SignableTransaction, Transaction,
    TransactionEnvelope, TxEip1559, TxEip2930, TxEip4844, TxEip4844Variant, TxEip4844WithSidecar,
    TxEip7702, TxEip8141, TxEip8141ValidationError, TxEnvelope, TxLegacy, TxType, TypedTransaction,
};

pub use alloy_eips::{
    eip4844::{
        builder::{SidecarBuilder, SidecarCoder, SimpleCoder},
        utils, Blob, BlobTransactionSidecar, Bytes48,
    },
    eip7594::{BlobTransactionSidecarEip7594, BlobTransactionSidecarVariant},
    Typed2718,
};

#[cfg(feature = "kzg")]
pub use alloy_eips::eip4844::env_settings::EnvKzgSettings;

pub use alloy_primitives::{Sealable, Sealed};

mod signed;
pub use signed::Signed;

pub use alloy_tx_macros::TransactionEnvelope;

pub mod crypto;
pub mod error;

pub mod extended;
pub use extended::Extended;

/// Bincode-compatible serde implementations for consensus types.
///
/// `bincode` crate doesn't work well with optionally serializable serde fields, but some of the
/// consensus types require optional serialization for RPC compatibility. This module makes so that
/// all fields are serialized.
///
/// Read more: <https://github.com/bincode-org/bincode/issues/326>
#[cfg(all(feature = "serde", feature = "serde-bincode-compat"))]
pub mod serde_bincode_compat {
    pub use super::{
        block::serde_bincode_compat::*,
        receipt::serde_bincode_compat::*,
        transaction::{serde_bincode_compat as transaction, serde_bincode_compat::*},
    };
}

#[doc(hidden)]
pub mod private {
    pub use alloy_eips;
    pub use alloy_primitives;
    pub use alloy_rlp;
    #[cfg(feature = "serde")]
    pub use alloy_serde;
    pub use alloy_trie;
    #[cfg(feature = "arbitrary")]
    pub use arbitrary;
    #[cfg(feature = "serde")]
    pub use serde;
    #[cfg(feature = "serde")]
    pub use serde_json;
}
