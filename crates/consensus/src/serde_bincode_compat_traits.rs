//! Bincode compatibility support traits.
//!
//! This module provides traits and implementations to work around bincode's limitations
//! with optional serde fields. The bincode crate requires all fields to be present during
//! serialization, which conflicts with types that have `#[serde(skip_serializing_if)]`
//! attributes for RPC compatibility.
//!
//! # Overview
//!
//! The main trait is [`SerdeBincodeCompat`], which provides a conversion mechanism between
//! types and their bincode-compatible representations. There are two main ways to implement
//! this trait:
//!
//! 1. **Using RLP encoding** - Implement [`RlpBincode`] for types that already support RLP
//! 2. **Custom implementation** - Define a custom representation type

use alloc::vec::Vec;
use alloy_primitives::Bytes;
use core::fmt::Debug;
use serde::{de::DeserializeOwned, Serialize};

/// Trait for types that can be serialized and deserialized using bincode.
///
/// This trait provides a workaround for bincode's incompatibility with optional
/// serde fields. It ensures all fields are serialized, making the type bincode-compatible.
///
/// # Implementation
///
/// The easiest way to implement this trait is using [`RlpBincode`] for RLP-encodable types:
///
/// ```rust,ignore
/// impl RlpBincode for MyType {}
/// // SerdeBincodeCompat is automatically implemented
/// ```
///
/// The recommended way to add bincode compatible serialization is via the
/// [`serde_with`] crate and the `serde_as` macro.
pub trait SerdeBincodeCompat: Sized + 'static {
    /// Serde representation of the type for bincode serialization.
    ///
    /// This type defines the bincode compatible serde format for the type.
    type BincodeRepr<'a>: Debug + Serialize + DeserializeOwned;

    /// Convert this type into its bincode representation
    fn as_repr(&self) -> Self::BincodeRepr<'_>;

    /// Convert from the bincode representation
    fn from_repr(repr: Self::BincodeRepr<'_>) -> Self;
}

/// Type alias for the [`SerdeBincodeCompat::BincodeRepr`] associated type.
pub type BincodeReprFor<'a, T> = <T as SerdeBincodeCompat>::BincodeRepr<'a>;

/// A helper trait for using RLP-encoding for providing bincode-compatible serialization.
///
/// By implementing this trait, [`SerdeBincodeCompat`] will be automatically implemented for the
/// type and RLP encoding will be used for serialization and deserialization for bincode
/// compatibility.
pub trait RlpBincode: alloy_rlp::Encodable + alloy_rlp::Decodable {}

impl<T: RlpBincode + 'static> SerdeBincodeCompat for T {
    type BincodeRepr<'a> = Bytes;

    fn as_repr(&self) -> Self::BincodeRepr<'_> {
        let mut buf = Vec::new();
        self.encode(&mut buf);
        buf.into()
    }

    fn from_repr(repr: Self::BincodeRepr<'_>) -> Self {
        Self::decode(&mut repr.as_ref()).expect("Failed to decode bincode rlp representation")
    }
}

// --- Implementations for alloy-consensus types ---

impl SerdeBincodeCompat for crate::Header {
    type BincodeRepr<'a> = crate::serde_bincode_compat::Header<'a>;

    fn as_repr(&self) -> Self::BincodeRepr<'_> {
        self.into()
    }

    fn from_repr(repr: Self::BincodeRepr<'_>) -> Self {
        repr.into()
    }
}

impl SerdeBincodeCompat for crate::EthereumTxEnvelope<crate::TxEip4844> {
    type BincodeRepr<'a> =
        crate::serde_bincode_compat::transaction::EthereumTxEnvelope<'a>;

    fn as_repr(&self) -> Self::BincodeRepr<'_> {
        self.into()
    }

    fn from_repr(repr: Self::BincodeRepr<'_>) -> Self {
        repr.into()
    }
}

// --- Block/BlockBody implementations ---

mod block_bincode {
    use super::SerdeBincodeCompat;
    use alloc::{borrow::Cow, vec::Vec};
    use alloy_eips::eip4895::Withdrawals;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde_with::{DeserializeAs, SerializeAs};

    /// Bincode-compatible [`alloy_consensus::Block`] serde implementation.
    #[derive(Serialize, Deserialize)]
    pub struct Block<'a, T: SerdeBincodeCompat, H: SerdeBincodeCompat> {
        header: H::BincodeRepr<'a>,
        #[serde(bound = "BlockBody<'a, T, H>: Serialize + serde::de::DeserializeOwned")]
        body: BlockBody<'a, T, H>,
    }

    impl<T: SerdeBincodeCompat, H: SerdeBincodeCompat> core::fmt::Debug for Block<'_, T, H> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.debug_struct("Block")
                .field("header", &self.header)
                .field("body", &self.body)
                .finish()
        }
    }

    impl<'a, T: SerdeBincodeCompat, H: SerdeBincodeCompat>
        From<&'a crate::Block<T, H>> for Block<'a, T, H>
    {
        fn from(value: &'a crate::Block<T, H>) -> Self {
            Self { header: value.header.as_repr(), body: (&value.body).into() }
        }
    }

    impl<'a, T: SerdeBincodeCompat, H: SerdeBincodeCompat> From<Block<'a, T, H>>
        for crate::Block<T, H>
    {
        fn from(value: Block<'a, T, H>) -> Self {
            Self {
                header: SerdeBincodeCompat::from_repr(value.header),
                body: value.body.into(),
            }
        }
    }

    impl<T: SerdeBincodeCompat, H: SerdeBincodeCompat>
        SerializeAs<crate::Block<T, H>> for Block<'_, T, H>
    {
        fn serialize_as<S>(
            source: &crate::Block<T, H>,
            serializer: S,
        ) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            Block::from(source).serialize(serializer)
        }
    }

    impl<'de, T: SerdeBincodeCompat, H: SerdeBincodeCompat>
        DeserializeAs<'de, crate::Block<T, H>> for Block<'de, T, H>
    {
        fn deserialize_as<D>(deserializer: D) -> Result<crate::Block<T, H>, D::Error>
        where
            D: Deserializer<'de>,
        {
            Block::deserialize(deserializer).map(Into::into)
        }
    }

    impl<T: SerdeBincodeCompat, H: SerdeBincodeCompat> SerdeBincodeCompat
        for crate::Block<T, H>
    {
        type BincodeRepr<'a> = Block<'a, T, H>;

        fn as_repr(&self) -> Self::BincodeRepr<'_> {
            self.into()
        }

        fn from_repr(repr: Self::BincodeRepr<'_>) -> Self {
            repr.into()
        }
    }

    /// Bincode-compatible [`alloy_consensus::BlockBody`] serde implementation.
    #[derive(Serialize, Deserialize)]
    pub struct BlockBody<'a, T: SerdeBincodeCompat, H: SerdeBincodeCompat> {
        transactions: Vec<T::BincodeRepr<'a>>,
        ommers: Vec<H::BincodeRepr<'a>>,
        withdrawals: Cow<'a, Option<Withdrawals>>,
    }

    impl<T: SerdeBincodeCompat, H: SerdeBincodeCompat> core::fmt::Debug
        for BlockBody<'_, T, H>
    {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.debug_struct("BlockBody")
                .field("transactions", &self.transactions)
                .field("ommers", &self.ommers)
                .field("withdrawals", &self.withdrawals)
                .finish()
        }
    }

    impl<'a, T: SerdeBincodeCompat, H: SerdeBincodeCompat>
        From<&'a crate::BlockBody<T, H>> for BlockBody<'a, T, H>
    {
        fn from(value: &'a crate::BlockBody<T, H>) -> Self {
            Self {
                transactions: value.transactions.iter().map(|tx| tx.as_repr()).collect(),
                ommers: value.ommers.iter().map(|h| h.as_repr()).collect(),
                withdrawals: Cow::Borrowed(&value.withdrawals),
            }
        }
    }

    impl<'a, T: SerdeBincodeCompat, H: SerdeBincodeCompat> From<BlockBody<'a, T, H>>
        for crate::BlockBody<T, H>
    {
        fn from(value: BlockBody<'a, T, H>) -> Self {
            Self {
                transactions: value
                    .transactions
                    .into_iter()
                    .map(SerdeBincodeCompat::from_repr)
                    .collect(),
                ommers: value.ommers.into_iter().map(SerdeBincodeCompat::from_repr).collect(),
                withdrawals: value.withdrawals.into_owned(),
            }
        }
    }

    impl<T: SerdeBincodeCompat, H: SerdeBincodeCompat>
        SerializeAs<crate::BlockBody<T, H>> for BlockBody<'_, T, H>
    {
        fn serialize_as<S>(
            source: &crate::BlockBody<T, H>,
            serializer: S,
        ) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            BlockBody::from(source).serialize(serializer)
        }
    }

    impl<'de, T: SerdeBincodeCompat, H: SerdeBincodeCompat>
        DeserializeAs<'de, crate::BlockBody<T, H>> for BlockBody<'de, T, H>
    {
        fn deserialize_as<D>(
            deserializer: D,
        ) -> Result<crate::BlockBody<T, H>, D::Error>
        where
            D: Deserializer<'de>,
        {
            BlockBody::deserialize(deserializer).map(Into::into)
        }
    }

    impl<T: SerdeBincodeCompat, H: SerdeBincodeCompat> SerdeBincodeCompat
        for crate::BlockBody<T, H>
    {
        type BincodeRepr<'a> = BlockBody<'a, T, H>;

        fn as_repr(&self) -> Self::BincodeRepr<'_> {
            self.into()
        }

        fn from_repr(repr: Self::BincodeRepr<'_>) -> Self {
            repr.into()
        }
    }
}

pub use block_bincode::{Block, BlockBody};
