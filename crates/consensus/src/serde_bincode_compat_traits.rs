//! Bincode compatibility traits and helpers for consensus types.
//!
//! This module provides traits for converting to and from bincode-compatible representations
//! without panicking on decode failures, along with compat wrappers for block types.

use alloc::vec::Vec;
use alloy_primitives::Bytes;
use core::{convert::Infallible, error::Error as StdError, fmt::Debug};
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

    /// Error returned when converting from the bincode representation.
    type Error: StdError + 'static;

    /// Convert this type into its bincode representation
    fn as_repr(&self) -> Self::BincodeRepr<'_>;

    /// Convert from the bincode representation
    fn from_repr(repr: Self::BincodeRepr<'_>) -> Result<Self, Self::Error>;
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
    type Error = alloy_rlp::Error;

    fn as_repr(&self) -> Self::BincodeRepr<'_> {
        let mut buf = Vec::new();
        self.encode(&mut buf);
        buf.into()
    }

    fn from_repr(repr: Self::BincodeRepr<'_>) -> Result<Self, Self::Error> {
        Self::decode(&mut repr.as_ref())
    }
}

// --- Implementations for alloy-consensus types ---

impl SerdeBincodeCompat for crate::Header {
    type BincodeRepr<'a> = crate::serde_bincode_compat::Header<'a>;
    type Error = Infallible;

    fn as_repr(&self) -> Self::BincodeRepr<'_> {
        self.into()
    }

    fn from_repr(repr: Self::BincodeRepr<'_>) -> Result<Self, Self::Error> {
        Ok(repr.into())
    }
}

impl SerdeBincodeCompat for crate::EthereumTxEnvelope<crate::TxEip4844> {
    type BincodeRepr<'a> = crate::serde_bincode_compat::transaction::EthereumTxEnvelope<'a>;
    type Error = Infallible;

    fn as_repr(&self) -> Self::BincodeRepr<'_> {
        self.into()
    }

    fn from_repr(repr: Self::BincodeRepr<'_>) -> Result<Self, Self::Error> {
        Ok(repr.into())
    }
}

// --- Block/BlockBody implementations ---

mod block_bincode {
    use super::SerdeBincodeCompat;
    use alloc::{borrow::Cow, vec::Vec};
    use alloy_eips::eip4895::Withdrawals;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde_with::{DeserializeAs, SerializeAs};

    /// Error returned when decoding a [`crate::Block`] from its bincode representation.
    #[derive(Debug, thiserror::Error)]
    pub enum BlockReprError<T, H>
    where
        T: core::error::Error + 'static,
        H: core::error::Error + 'static,
    {
        /// Decoding the header failed.
        #[error("failed to decode block header from bincode representation")]
        Header(#[source] H),
        /// Decoding the body failed.
        #[error("failed to decode block body from bincode representation")]
        Body(#[source] BlockBodyReprError<T, H>),
    }

    /// Error returned when decoding a [`crate::BlockBody`] from its bincode representation.
    #[derive(Debug, thiserror::Error)]
    pub enum BlockBodyReprError<T, H>
    where
        T: core::error::Error + 'static,
        H: core::error::Error + 'static,
    {
        /// Decoding a transaction failed.
        #[error("failed to decode block transaction from bincode representation")]
        Transaction(#[source] T),
        /// Decoding an ommer failed.
        #[error("failed to decode block ommer from bincode representation")]
        Ommer(#[source] H),
    }

    /// Bincode-compatible [`crate::Block`] serde implementation.
    #[derive(Serialize, Deserialize)]
    pub struct Block<'a, T: SerdeBincodeCompat, H: SerdeBincodeCompat> {
        header: H::BincodeRepr<'a>,
        #[serde(bound = "BlockBody<'a, T, H>: Serialize + serde::de::DeserializeOwned")]
        body: BlockBody<'a, T, H>,
    }

    impl<T: SerdeBincodeCompat, H: SerdeBincodeCompat> core::fmt::Debug for Block<'_, T, H> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.debug_struct("Block").field("header", &self.header).field("body", &self.body).finish()
        }
    }

    impl<'a, T: SerdeBincodeCompat, H: SerdeBincodeCompat> From<&'a crate::Block<T, H>>
        for Block<'a, T, H>
    {
        fn from(value: &'a crate::Block<T, H>) -> Self {
            Self { header: value.header.as_repr(), body: (&value.body).into() }
        }
    }

    impl<'a, T: SerdeBincodeCompat, H: SerdeBincodeCompat> TryFrom<Block<'a, T, H>>
        for crate::Block<T, H>
    {
        type Error = BlockReprError<T::Error, H::Error>;

        fn try_from(value: Block<'a, T, H>) -> Result<Self, Self::Error> {
            Ok(Self {
                header: SerdeBincodeCompat::from_repr(value.header)
                    .map_err(BlockReprError::Header)?,
                body: value.body.try_into().map_err(BlockReprError::Body)?,
            })
        }
    }

    impl<T: SerdeBincodeCompat, H: SerdeBincodeCompat> SerializeAs<crate::Block<T, H>>
        for Block<'_, T, H>
    {
        fn serialize_as<S>(source: &crate::Block<T, H>, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            Block::from(source).serialize(serializer)
        }
    }

    impl<'de, T: SerdeBincodeCompat, H: SerdeBincodeCompat> DeserializeAs<'de, crate::Block<T, H>>
        for Block<'de, T, H>
    {
        fn deserialize_as<D>(deserializer: D) -> Result<crate::Block<T, H>, D::Error>
        where
            D: Deserializer<'de>,
        {
            Block::deserialize(deserializer)?
                .try_into()
                .map_err(<D::Error as serde::de::Error>::custom)
        }
    }

    impl<T: SerdeBincodeCompat, H: SerdeBincodeCompat> SerdeBincodeCompat for crate::Block<T, H> {
        type BincodeRepr<'a> = Block<'a, T, H>;
        type Error = BlockReprError<T::Error, H::Error>;

        fn as_repr(&self) -> Self::BincodeRepr<'_> {
            self.into()
        }

        fn from_repr(repr: Self::BincodeRepr<'_>) -> Result<Self, Self::Error> {
            repr.try_into()
        }
    }

    /// Bincode-compatible [`crate::BlockBody`] serde implementation.
    #[derive(Serialize, Deserialize)]
    pub struct BlockBody<'a, T: SerdeBincodeCompat, H: SerdeBincodeCompat> {
        transactions: Vec<T::BincodeRepr<'a>>,
        ommers: Vec<H::BincodeRepr<'a>>,
        withdrawals: Cow<'a, Option<Withdrawals>>,
    }

    impl<T: SerdeBincodeCompat, H: SerdeBincodeCompat> core::fmt::Debug for BlockBody<'_, T, H> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.debug_struct("BlockBody")
                .field("transactions", &self.transactions)
                .field("ommers", &self.ommers)
                .field("withdrawals", &self.withdrawals)
                .finish()
        }
    }

    impl<'a, T: SerdeBincodeCompat, H: SerdeBincodeCompat> From<&'a crate::BlockBody<T, H>>
        for BlockBody<'a, T, H>
    {
        fn from(value: &'a crate::BlockBody<T, H>) -> Self {
            Self {
                transactions: value.transactions.iter().map(|tx| tx.as_repr()).collect(),
                ommers: value.ommers.iter().map(|h| h.as_repr()).collect(),
                withdrawals: Cow::Borrowed(&value.withdrawals),
            }
        }
    }

    impl<'a, T: SerdeBincodeCompat, H: SerdeBincodeCompat> TryFrom<BlockBody<'a, T, H>>
        for crate::BlockBody<T, H>
    {
        type Error = BlockBodyReprError<T::Error, H::Error>;

        fn try_from(value: BlockBody<'a, T, H>) -> Result<Self, Self::Error> {
            Ok(Self {
                transactions: value
                    .transactions
                    .into_iter()
                    .map(|repr| {
                        SerdeBincodeCompat::from_repr(repr).map_err(BlockBodyReprError::Transaction)
                    })
                    .collect::<Result<_, _>>()?,
                ommers: value
                    .ommers
                    .into_iter()
                    .map(|repr| {
                        SerdeBincodeCompat::from_repr(repr).map_err(BlockBodyReprError::Ommer)
                    })
                    .collect::<Result<_, _>>()?,
                withdrawals: value.withdrawals.into_owned(),
            })
        }
    }

    impl<T: SerdeBincodeCompat, H: SerdeBincodeCompat> SerializeAs<crate::BlockBody<T, H>>
        for BlockBody<'_, T, H>
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
        fn deserialize_as<D>(deserializer: D) -> Result<crate::BlockBody<T, H>, D::Error>
        where
            D: Deserializer<'de>,
        {
            BlockBody::deserialize(deserializer)?
                .try_into()
                .map_err(<D::Error as serde::de::Error>::custom)
        }
    }

    impl<T: SerdeBincodeCompat, H: SerdeBincodeCompat> SerdeBincodeCompat for crate::BlockBody<T, H> {
        type BincodeRepr<'a> = BlockBody<'a, T, H>;
        type Error = BlockBodyReprError<T::Error, H::Error>;

        fn as_repr(&self) -> Self::BincodeRepr<'_> {
            self.into()
        }

        fn from_repr(repr: Self::BincodeRepr<'_>) -> Result<Self, Self::Error> {
            repr.try_into()
        }
    }
}

pub use block_bincode::{Block, BlockBody, BlockBodyReprError, BlockReprError};

#[cfg(test)]
mod tests {
    use super::{RlpBincode, SerdeBincodeCompat};
    use alloy_primitives::Bytes;
    use alloy_rlp::{RlpDecodable, RlpEncodable};

    #[derive(Debug, PartialEq, Eq, RlpEncodable, RlpDecodable)]
    struct TestRlp(u8);

    impl RlpBincode for TestRlp {}

    #[test]
    fn rlp_bincode_from_repr_returns_error() {
        assert!(<TestRlp as SerdeBincodeCompat>::from_repr(Bytes::from_static(&[0xff])).is_err());
    }

    #[test]
    fn block_from_repr_roundtrip() {
        let block = crate::Block::<crate::EthereumTxEnvelope<crate::TxEip4844>>::uncle(
            crate::Header::default(),
        );

        let repr = block.as_repr();
        let decoded = <crate::Block<crate::EthereumTxEnvelope<crate::TxEip4844>> as SerdeBincodeCompat>::from_repr(repr)
            .unwrap();

        assert_eq!(block, decoded);
    }
}
