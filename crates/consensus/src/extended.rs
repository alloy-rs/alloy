//! Extended transaction types

use crate::{transaction::delegate_transaction, EthereumTxEnvelope, Transaction};
use alloy_eips::{
    eip2718::{Eip2718Error, Eip2718Result, IsTyped2718},
    Decodable2718, Encodable2718, Typed2718,
};
use alloy_rlp::{BufMut, Decodable, Encodable, Result as RlpResult};

/// An enum that combines two different transaction types.
///
/// This is intended to be used to extend existing presets, for example the ethereum or opstack
/// transaction types and receipts
///
/// Note: The [`Extended::Other`] variants must not overlap with the builtin one, transaction
/// types must be unique. For example if [`Extended::BuiltIn`] contains an `EIP-1559` type variant,
/// [`Extended::Other`] must not include that type.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Extended<BuiltIn, Other> {
    /// The builtin transaction type.
    BuiltIn(BuiltIn),
    /// The other transaction type.
    Other(Other),
}

impl<B, T> Extended<B, T> {
    /// Converts only the built-in transaction type using `From`, leaving the other type unchanged.
    pub fn convert_builtin<U>(self) -> Extended<U, T>
    where
        U: From<B>,
    {
        self.map_builtin(U::from)
    }

    /// Attempts to convert only the built-in transaction type using `TryFrom`, leaving the other
    /// type unchanged.
    pub fn try_convert_builtin<U>(self) -> Result<Extended<U, T>, U::Error>
    where
        U: TryFrom<B>,
    {
        self.try_map_builtin(U::try_from)
    }

    /// Converts only the other transaction type using `From`, leaving the built-in type unchanged.
    pub fn convert_other<U>(self) -> Extended<B, U>
    where
        U: From<T>,
    {
        self.map_other(U::from)
    }

    /// Attempts to convert only the other transaction type using `TryFrom`, leaving the built-in
    /// type unchanged.
    pub fn try_convert_other<U>(self) -> Result<Extended<B, U>, U::Error>
    where
        U: TryFrom<T>,
    {
        self.try_map_other(U::try_from)
    }

    /// Maps only the built-in transaction type using the provided function, leaving the other type
    /// unchanged.
    pub fn map_builtin<U>(self, f: impl FnOnce(B) -> U) -> Extended<U, T> {
        match self {
            Self::BuiltIn(tx) => Extended::BuiltIn(f(tx)),
            Self::Other(tx) => Extended::Other(tx),
        }
    }

    /// Attempts to map only the built-in transaction type using the provided fallible function,
    /// leaving the other type unchanged. Returns an error if the mapping of the built-in type
    /// fails.
    pub fn try_map_builtin<U, E>(
        self,
        f: impl FnOnce(B) -> Result<U, E>,
    ) -> Result<Extended<U, T>, E> {
        match self {
            Self::BuiltIn(tx) => f(tx).map(Extended::BuiltIn),
            Self::Other(tx) => Ok(Extended::Other(tx)),
        }
    }

    /// Maps only the other transaction type using the provided function, leaving the built-in type
    /// unchanged.
    pub fn map_other<U>(self, f: impl FnOnce(T) -> U) -> Extended<B, U> {
        match self {
            Self::BuiltIn(tx) => Extended::BuiltIn(tx),
            Self::Other(tx) => Extended::Other(f(tx)),
        }
    }

    /// Attempts to map only the other transaction type using the provided fallible function,
    /// leaving the built-in type unchanged. Returns an error if the mapping of the other type
    /// fails.
    pub fn try_map_other<U, F>(
        self,
        f: impl FnOnce(T) -> Result<U, F>,
    ) -> Result<Extended<B, U>, F> {
        match self {
            Self::BuiltIn(tx) => Ok(Extended::BuiltIn(tx)),
            Self::Other(tx) => f(tx).map(Extended::Other),
        }
    }
}

impl<B, T> Transaction for Extended<B, T>
where
    B: Transaction,
    T: Transaction,
{
    delegate_transaction!(self => match Self::BuiltIn, Self::Other);
}

impl<B, T> IsTyped2718 for Extended<B, T>
where
    B: IsTyped2718,
    T: IsTyped2718,
{
    fn is_type(type_id: u8) -> bool {
        B::is_type(type_id) || T::is_type(type_id)
    }
}

impl<B, T> Typed2718 for Extended<B, T>
where
    B: Typed2718,
    T: Typed2718,
{
    fn ty(&self) -> u8 {
        match self {
            Self::BuiltIn(tx) => tx.ty(),
            Self::Other(tx) => tx.ty(),
        }
    }
}

impl<B, T> Decodable2718 for Extended<B, T>
where
    B: Decodable2718 + IsTyped2718,
    T: Decodable2718,
{
    fn typed_decode(ty: u8, buf: &mut &[u8]) -> Eip2718Result<Self> {
        if B::is_type(ty) {
            let envelope = B::typed_decode(ty, buf)?;
            Ok(Self::BuiltIn(envelope))
        } else {
            let other = T::typed_decode(ty, buf)?;
            Ok(Self::Other(other))
        }
    }
    fn fallback_decode(buf: &mut &[u8]) -> Eip2718Result<Self> {
        if buf.is_empty() {
            return Err(Eip2718Error::RlpError(alloy_rlp::Error::InputTooShort));
        }
        B::fallback_decode(buf).map(Self::BuiltIn)
    }
}

impl<B, T> Encodable2718 for Extended<B, T>
where
    B: Encodable2718,
    T: Encodable2718,
{
    fn encode_2718_len(&self) -> usize {
        match self {
            Self::BuiltIn(envelope) => envelope.encode_2718_len(),
            Self::Other(tx) => tx.encode_2718_len(),
        }
    }

    fn encode_2718(&self, out: &mut dyn BufMut) {
        match self {
            Self::BuiltIn(envelope) => envelope.encode_2718(out),
            Self::Other(tx) => tx.encode_2718(out),
        }
    }
}

impl<B, T> Encodable for Extended<B, T>
where
    B: Encodable,
    T: Encodable,
{
    fn encode(&self, out: &mut dyn BufMut) {
        match self {
            Self::BuiltIn(envelope) => envelope.encode(out),
            Self::Other(tx) => tx.encode(out),
        }
    }

    fn length(&self) -> usize {
        match self {
            Self::BuiltIn(envelope) => envelope.length(),
            Self::Other(tx) => tx.length(),
        }
    }
}

impl<B, T> Decodable for Extended<B, T>
where
    B: Decodable,
    T: Decodable,
{
    fn decode(buf: &mut &[u8]) -> RlpResult<Self> {
        let original = *buf;

        B::decode(buf).map_or_else(
            |_| {
                *buf = original;
                T::decode(buf).map(Self::Other)
            },
            |tx| Ok(Self::BuiltIn(tx)),
        )
    }
}

impl<Eip4844, Tx> From<EthereumTxEnvelope<Eip4844>> for Extended<EthereumTxEnvelope<Eip4844>, Tx> {
    fn from(value: EthereumTxEnvelope<Eip4844>) -> Self {
        Self::BuiltIn(value)
    }
}
