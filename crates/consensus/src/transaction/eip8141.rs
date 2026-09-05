use alloc::vec::Vec;
use core::{fmt, mem::size_of};

use alloy_eips::{
    eip2718::{Eip2718Error, Eip2718Result, IsTyped2718},
    eip7594::{Decodable7594, Encodable7594},
    eip7702::SignedAuthorization,
    eip7825::MAX_TX_GAS_LIMIT_OSAKA,
    eip8141::{
        constants::{
            FRAME_FLAGS_MASK, FRAME_TX_DATA_TOKEN_STANDARD_COST, FRAME_TX_INTRINSIC_COST,
            FRAME_TX_PER_FRAME_COST, FRAME_TX_TOTAL_COST_FLOOR_PER_TOKEN, FRAME_TX_TYPE,
            MAX_FRAMES, TX_VALUE_COST,
        },
        ApprovalScope, Frame, FrameMode, FrameSignature, SignatureScheme, TransactionFees,
    },
    Decodable2718, Encodable2718, Typed2718,
};
use alloy_primitives::{keccak256, Address, Bytes, ChainId, Sealable, TxKind, B256, U256};
use alloy_rlp::{BufMut, Decodable, Encodable, Header};

use crate::Transaction;

static EMPTY_INPUT: Bytes = Bytes::new();

struct SigningFrameSignature<'a>(&'a FrameSignature);

impl Encodable for SigningFrameSignature<'_> {
    fn encode(&self, out: &mut dyn BufMut) {
        let signature = self.0;
        Header { list: true, payload_length: self.payload_length() }.encode(out);
        signature.scheme.encode(out);
        signature.signer.encode(out);
        signature.msg.encode(out);
        if signature.msg.is_empty() {
            EMPTY_INPUT.encode(out);
        } else {
            signature.signature.encode(out);
        }
    }

    fn length(&self) -> usize {
        Header { list: true, payload_length: self.payload_length() }.length_with_payload()
    }
}

impl SigningFrameSignature<'_> {
    fn payload_length(&self) -> usize {
        let signature = self.0;
        signature.scheme.length()
            + signature.signer.length()
            + signature.msg.length()
            + if signature.msg.is_empty() {
                EMPTY_INPUT.length()
            } else {
                signature.signature.length()
            }
    }
}

struct SigningFrameSignatures<'a>(&'a [FrameSignature]);

impl Encodable for SigningFrameSignatures<'_> {
    fn encode(&self, out: &mut dyn BufMut) {
        let payload_length =
            self.0.iter().map(|signature| SigningFrameSignature(signature).length()).sum();
        Header { list: true, payload_length }.encode(out);
        for signature in self.0 {
            SigningFrameSignature(signature).encode(out);
        }
    }

    fn length(&self) -> usize {
        let payload_length =
            self.0.iter().map(|signature| SigningFrameSignature(signature).length()).sum();
        Header { list: true, payload_length }.length_with_payload()
    }
}

/// Counts frame transaction calldata tokens.
///
/// Zero bytes count as one token and non-zero bytes count as four tokens.
pub fn count_frame_data_tokens(data: &[u8]) -> u64 {
    data.iter().fold(0u64, |acc, byte| acc.saturating_add(if *byte == 0 { 1 } else { 4 }))
}

/// An EIP-8141 frame transaction.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "borsh", derive(borsh::BorshSerialize, borsh::BorshDeserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[doc(alias = "Eip8141Transaction", alias = "TransactionEip8141", alias = "Eip8141Tx")]
pub struct TxEip8141 {
    /// EIP-155 replay protection chain ID.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub chain_id: ChainId,
    /// Sender nonce.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub nonce: u64,
    /// Intended transaction sender.
    pub sender: Address,
    /// Ordered frames to execute.
    pub frames: Vec<Frame>,
    /// Signature entries available to validation and execution code.
    pub signatures: Vec<FrameSignature>,
    /// EIP-8141 fee parameters.
    pub fees: TransactionFees,
    /// Blob versioned hashes.
    pub blob_versioned_hashes: Vec<B256>,
}

/// An EIP-8141 frame transaction paired with its EIP-7594 blob sidecar.
///
/// The sidecar is network data and is excluded from the transaction hash. It is encoded after the
/// canonical frame transaction payload when the transaction is propagated over the network.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TxEip8141WithSidecar<T> {
    /// The canonical EIP-8141 transaction.
    pub tx: TxEip8141,
    /// The EIP-7594 blob sidecar.
    pub sidecar: T,
}

/// The EIP-8141 network representation, with an optional blob sidecar.
///
/// Transactions without blob hashes are propagated in their canonical form. Transactions with
/// blob hashes use the sidecar wrapper.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum TxEip8141Variant<T> {
    /// A canonical EIP-8141 transaction without a sidecar.
    TxEip8141(TxEip8141),
    /// An EIP-8141 transaction with its network sidecar.
    TxEip8141WithSidecar(TxEip8141WithSidecar<T>),
}

impl<T> TxEip8141Variant<T> {
    /// Returns the canonical transaction.
    pub const fn tx(&self) -> &TxEip8141 {
        match self {
            Self::TxEip8141(tx) => tx,
            Self::TxEip8141WithSidecar(tx) => tx.tx(),
        }
    }

    /// Returns the sidecar, if present.
    pub const fn sidecar(&self) -> Option<&T> {
        match self {
            Self::TxEip8141(_) => None,
            Self::TxEip8141WithSidecar(tx) => Some(tx.sidecar()),
        }
    }

    /// Returns the sidecar mutably, if present.
    pub const fn sidecar_mut(&mut self) -> Option<&mut T> {
        match self {
            Self::TxEip8141(_) => None,
            Self::TxEip8141WithSidecar(tx) => Some(&mut tx.sidecar),
        }
    }
}

impl<T> From<TxEip8141> for TxEip8141Variant<T> {
    fn from(value: TxEip8141) -> Self {
        Self::TxEip8141(value)
    }
}

impl<T> From<TxEip8141WithSidecar<T>> for TxEip8141Variant<T> {
    fn from(value: TxEip8141WithSidecar<T>) -> Self {
        Self::TxEip8141WithSidecar(value)
    }
}

impl<T> Sealable for TxEip8141Variant<T> {
    fn hash_slow(&self) -> B256 {
        self.tx().tx_hash()
    }
}

impl<T> Typed2718 for TxEip8141Variant<T> {
    fn ty(&self) -> u8 {
        FRAME_TX_TYPE
    }
}

impl<T> IsTyped2718 for TxEip8141Variant<T> {
    fn is_type(type_id: u8) -> bool {
        type_id == FRAME_TX_TYPE
    }
}

impl<T: Encodable7594> Encodable for TxEip8141Variant<T> {
    fn encode(&self, out: &mut dyn BufMut) {
        match self {
            Self::TxEip8141(tx) => tx.encode(out),
            Self::TxEip8141WithSidecar(tx) => tx.encode(out),
        }
    }

    fn length(&self) -> usize {
        match self {
            Self::TxEip8141(tx) => tx.length(),
            Self::TxEip8141WithSidecar(tx) => tx.length(),
        }
    }
}

impl<T: Encodable7594 + Send + Sync> Encodable2718 for TxEip8141Variant<T> {
    fn encode_2718_len(&self) -> usize {
        self.length() + 1
    }

    fn encode_2718(&self, out: &mut dyn BufMut) {
        out.put_u8(FRAME_TX_TYPE);
        self.encode(out);
    }
}

impl<T: Decodable7594 + Clone> Decodable for TxEip8141Variant<T> {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let needle = &mut &**buf;
        let trial = &mut &**buf;

        let outer = Header::decode(needle)?;
        if !outer.list {
            return Err(alloy_rlp::Error::UnexpectedString);
        }
        if Header::decode(needle).is_ok_and(|header| header.list) {
            if let Ok(tx) = TxEip8141WithSidecar::decode(trial) {
                *buf = *trial;
                return Ok(Self::TxEip8141WithSidecar(tx));
            }
        }
        let tx = TxEip8141::decode(buf)?;
        if !tx.blob_versioned_hashes.is_empty() {
            return Err(alloy_rlp::Error::Custom(
                "pooled frame transaction requires a blob sidecar",
            ));
        }
        Ok(Self::TxEip8141(tx))
    }
}

impl<T: Decodable7594 + Clone> Decodable2718 for TxEip8141Variant<T> {
    fn typed_decode(ty: u8, buf: &mut &[u8]) -> Eip2718Result<Self> {
        if ty != FRAME_TX_TYPE {
            return Err(Eip2718Error::UnexpectedType(ty));
        }
        Self::decode(buf).map_err(Into::into)
    }

    fn fallback_decode(_buf: &mut &[u8]) -> Eip2718Result<Self> {
        Err(Eip2718Error::UnexpectedType(FRAME_TX_TYPE))
    }
}

impl<T: fmt::Debug + Send + Sync + 'static> Transaction for TxEip8141Variant<T> {
    fn frame_transaction(&self) -> Option<&TxEip8141> {
        Some(self.tx())
    }

    fn chain_id(&self) -> Option<ChainId> {
        self.tx().chain_id()
    }

    fn nonce(&self) -> u64 {
        self.tx().nonce()
    }

    fn gas_limit(&self) -> u64 {
        self.tx().gas_limit()
    }

    fn gas_price(&self) -> Option<u128> {
        self.tx().gas_price()
    }

    fn max_fee_per_gas(&self) -> u128 {
        self.tx().max_fee_per_gas()
    }

    fn max_fee_per_gas_u256(&self) -> U256 {
        self.tx().max_fee_per_gas_u256()
    }

    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        self.tx().max_priority_fee_per_gas()
    }

    fn max_priority_fee_per_gas_u256(&self) -> Option<U256> {
        self.tx().max_priority_fee_per_gas_u256()
    }

    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        self.tx().max_fee_per_blob_gas()
    }

    fn max_fee_per_blob_gas_u256(&self) -> Option<U256> {
        self.tx().max_fee_per_blob_gas_u256()
    }

    fn priority_fee_or_price(&self) -> u128 {
        self.tx().priority_fee_or_price()
    }

    fn priority_fee_or_price_u256(&self) -> U256 {
        self.tx().priority_fee_or_price_u256()
    }

    fn effective_gas_price(&self, base_fee: Option<u64>) -> u128 {
        self.tx().effective_gas_price(base_fee)
    }

    fn effective_gas_price_u256(&self, base_fee: Option<u64>) -> U256 {
        self.tx().effective_gas_price_u256(base_fee)
    }

    fn is_dynamic_fee(&self) -> bool {
        true
    }

    fn kind(&self) -> TxKind {
        self.tx().kind()
    }

    fn is_create(&self) -> bool {
        false
    }

    fn value(&self) -> U256 {
        U256::ZERO
    }

    fn input(&self) -> &Bytes {
        self.tx().input()
    }

    fn access_list(&self) -> Option<&alloy_eips::eip2930::AccessList> {
        None
    }

    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        Some(&self.tx().blob_versioned_hashes)
    }

    fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
        None
    }
}

/// Immutable pooled frame transaction with its derived gas limit computed once.
/// Only sidecar mutation is exposed; canonical fields cannot invalidate the gas cache.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CachedFrameTransaction<T> {
    inner: TxEip8141Variant<T>,
    gas_limit: u64,
}

impl<T> CachedFrameTransaction<T> {
    /// Computes the derived gas limit and freezes the canonical transaction fields.
    pub fn new(inner: TxEip8141Variant<T>) -> Self {
        let gas_limit = inner.tx().calculate_gas_limit();
        Self { inner, gas_limit }
    }

    /// Returns the canonical transaction.
    pub const fn tx(&self) -> &TxEip8141 {
        self.inner.tx()
    }

    /// Returns the sidecar, if any.
    pub const fn sidecar(&self) -> Option<&T> {
        self.inner.sidecar()
    }

    /// Mutates only network data, leaving the canonical transaction and gas cache unchanged.
    pub const fn sidecar_mut(&mut self) -> Option<&mut T> {
        self.inner.sidecar_mut()
    }

    /// Unwraps the transaction, discarding derived metadata.
    pub fn into_inner(self) -> TxEip8141Variant<T> {
        self.inner
    }

    /// Returns the cached gas limit.
    pub const fn gas_limit(&self) -> u64 {
        self.gas_limit
    }
}

impl<T> From<TxEip8141Variant<T>> for CachedFrameTransaction<T> {
    fn from(tx: TxEip8141Variant<T>) -> Self {
        Self::new(tx)
    }
}

impl<T> From<TxEip8141> for CachedFrameTransaction<T> {
    fn from(tx: TxEip8141) -> Self {
        Self::new(tx.into())
    }
}

impl<T> From<TxEip8141WithSidecar<T>> for CachedFrameTransaction<T> {
    fn from(tx: TxEip8141WithSidecar<T>) -> Self {
        Self::new(tx.into())
    }
}

impl<T> Sealable for CachedFrameTransaction<T> {
    fn hash_slow(&self) -> B256 {
        self.inner.hash_slow()
    }
}

#[cfg(any(test, feature = "arbitrary"))]
impl<'a, T: arbitrary::Arbitrary<'a>> arbitrary::Arbitrary<'a> for CachedFrameTransaction<T> {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let mut tx = TxEip8141::arbitrary(u)?;
        if u.arbitrary()? {
            Ok(TxEip8141WithSidecar::new(tx, u.arbitrary()?).into())
        } else {
            // A bare pooled transaction cannot advertise blobs without their sidecar.
            tx.blob_versioned_hashes.clear();
            Ok(tx.into())
        }
    }
}

impl<T> Typed2718 for CachedFrameTransaction<T> {
    fn ty(&self) -> u8 {
        FRAME_TX_TYPE
    }
}

impl<T> IsTyped2718 for CachedFrameTransaction<T> {
    fn is_type(ty: u8) -> bool {
        ty == FRAME_TX_TYPE
    }
}

impl<T: Encodable7594> Encodable for CachedFrameTransaction<T> {
    fn encode(&self, out: &mut dyn BufMut) {
        self.inner.encode(out);
    }
    fn length(&self) -> usize {
        self.inner.length()
    }
}

impl<T: Decodable7594 + Clone> Decodable for CachedFrameTransaction<T> {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        TxEip8141Variant::decode(buf).map(Self::new)
    }
}

impl<T: Encodable7594 + Send + Sync> Encodable2718 for CachedFrameTransaction<T> {
    fn encode_2718_len(&self) -> usize {
        self.inner.encode_2718_len()
    }
    fn encode_2718(&self, out: &mut dyn BufMut) {
        self.inner.encode_2718(out);
    }
}

impl<T: Decodable7594 + Clone> Decodable2718 for CachedFrameTransaction<T> {
    fn typed_decode(ty: u8, buf: &mut &[u8]) -> Eip2718Result<Self> {
        TxEip8141Variant::typed_decode(ty, buf).map(Self::new)
    }
    fn fallback_decode(buf: &mut &[u8]) -> Eip2718Result<Self> {
        TxEip8141Variant::fallback_decode(buf).map(Self::new)
    }
}

#[cfg(feature = "serde")]
impl<T: serde::Serialize> serde::Serialize for CachedFrameTransaction<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.inner.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for CachedFrameTransaction<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let inner = TxEip8141Variant::deserialize(deserializer)?;
        if inner.sidecar().is_none() && !inner.tx().blob_versioned_hashes.is_empty() {
            return Err(serde::de::Error::custom(
                "pooled frame transaction requires a blob sidecar",
            ));
        }
        Ok(Self::new(inner))
    }
}

impl<T: fmt::Debug + Send + Sync + 'static> Transaction for CachedFrameTransaction<T> {
    fn frame_transaction(&self) -> Option<&TxEip8141> {
        Some(self.tx())
    }

    fn chain_id(&self) -> Option<ChainId> {
        self.tx().chain_id()
    }

    fn nonce(&self) -> u64 {
        self.tx().nonce()
    }

    fn gas_limit(&self) -> u64 {
        self.gas_limit
    }

    fn gas_price(&self) -> Option<u128> {
        self.tx().gas_price()
    }

    fn max_fee_per_gas(&self) -> u128 {
        self.tx().max_fee_per_gas()
    }

    fn max_fee_per_gas_u256(&self) -> U256 {
        self.tx().max_fee_per_gas_u256()
    }

    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        self.tx().max_priority_fee_per_gas()
    }

    fn max_priority_fee_per_gas_u256(&self) -> Option<U256> {
        self.tx().max_priority_fee_per_gas_u256()
    }

    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        self.tx().max_fee_per_blob_gas()
    }

    fn max_fee_per_blob_gas_u256(&self) -> Option<U256> {
        self.tx().max_fee_per_blob_gas_u256()
    }

    fn priority_fee_or_price(&self) -> u128 {
        self.tx().priority_fee_or_price()
    }

    fn priority_fee_or_price_u256(&self) -> U256 {
        self.tx().priority_fee_or_price_u256()
    }

    fn effective_gas_price(&self, base_fee: Option<u64>) -> u128 {
        self.tx().effective_gas_price(base_fee)
    }

    fn effective_gas_price_u256(&self, base_fee: Option<u64>) -> U256 {
        self.tx().effective_gas_price_u256(base_fee)
    }

    fn is_dynamic_fee(&self) -> bool {
        true
    }

    fn kind(&self) -> TxKind {
        self.tx().kind()
    }

    fn is_create(&self) -> bool {
        false
    }

    fn value(&self) -> U256 {
        U256::ZERO
    }

    fn input(&self) -> &Bytes {
        self.tx().input()
    }

    fn access_list(&self) -> Option<&alloy_eips::eip2930::AccessList> {
        None
    }

    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        Some(&self.tx().blob_versioned_hashes)
    }

    fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
        None
    }
}

impl<T> TxEip8141WithSidecar<T> {
    /// Creates a frame transaction with a network sidecar.
    pub const fn new(tx: TxEip8141, sidecar: T) -> Self {
        Self { tx, sidecar }
    }

    /// Returns the canonical transaction.
    pub const fn tx(&self) -> &TxEip8141 {
        &self.tx
    }

    /// Returns the sidecar.
    pub const fn sidecar(&self) -> &T {
        &self.sidecar
    }

    /// Splits the transaction and sidecar.
    pub fn into_parts(self) -> (TxEip8141, T) {
        (self.tx, self.sidecar)
    }
}

impl<T> Sealable for TxEip8141WithSidecar<T> {
    fn hash_slow(&self) -> B256 {
        self.tx.tx_hash()
    }
}

impl<T> Typed2718 for TxEip8141WithSidecar<T> {
    fn ty(&self) -> u8 {
        FRAME_TX_TYPE
    }
}

impl<T: Encodable7594> Encodable for TxEip8141WithSidecar<T> {
    fn encode(&self, out: &mut dyn BufMut) {
        let payload_length = self.tx.rlp_encoded_length() + self.sidecar.encode_7594_len();
        Header { list: true, payload_length }.encode(out);
        self.tx.rlp_encode(out);
        self.sidecar.encode_7594(out);
    }

    fn length(&self) -> usize {
        let payload_length = self.tx.rlp_encoded_length() + self.sidecar.encode_7594_len();
        Header { list: true, payload_length }.length_with_payload()
    }
}

impl<T: Encodable7594 + Send + Sync> Encodable2718 for TxEip8141WithSidecar<T> {
    fn encode_2718_len(&self) -> usize {
        self.length() + 1
    }

    fn encode_2718(&self, out: &mut dyn BufMut) {
        out.put_u8(FRAME_TX_TYPE);
        self.encode(out);
    }
}

impl<T: Decodable7594 + Clone> Decodable for TxEip8141WithSidecar<T> {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let header = Header::decode(buf)?;
        if !header.list {
            return Err(alloy_rlp::Error::UnexpectedString);
        }
        let remaining = buf.len();
        let tx = TxEip8141::decode(buf)?;
        let sidecar = T::decode_7594(buf)?;
        if buf.len() + header.payload_length != remaining {
            return Err(alloy_rlp::Error::UnexpectedLength);
        }
        Ok(Self { tx, sidecar })
    }
}

impl<T: Decodable7594 + Clone> Decodable2718 for TxEip8141WithSidecar<T> {
    fn typed_decode(ty: u8, buf: &mut &[u8]) -> Eip2718Result<Self> {
        if ty != FRAME_TX_TYPE {
            return Err(Eip2718Error::UnexpectedType(ty));
        }
        Self::decode(buf).map_err(Into::into)
    }

    fn fallback_decode(_buf: &mut &[u8]) -> Eip2718Result<Self> {
        Err(Eip2718Error::UnexpectedType(FRAME_TX_TYPE))
    }
}

impl<T: fmt::Debug + Send + Sync + 'static> Transaction for TxEip8141WithSidecar<T> {
    fn frame_transaction(&self) -> Option<&TxEip8141> {
        Some(&self.tx)
    }

    fn chain_id(&self) -> Option<ChainId> {
        self.tx.chain_id()
    }

    fn nonce(&self) -> u64 {
        self.tx.nonce()
    }

    fn gas_limit(&self) -> u64 {
        self.tx.gas_limit()
    }

    fn gas_price(&self) -> Option<u128> {
        self.tx.gas_price()
    }

    fn max_fee_per_gas(&self) -> u128 {
        self.tx.max_fee_per_gas()
    }

    fn max_fee_per_gas_u256(&self) -> U256 {
        self.tx.max_fee_per_gas_u256()
    }

    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        self.tx.max_priority_fee_per_gas()
    }

    fn max_priority_fee_per_gas_u256(&self) -> Option<U256> {
        self.tx.max_priority_fee_per_gas_u256()
    }

    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        self.tx.max_fee_per_blob_gas()
    }

    fn max_fee_per_blob_gas_u256(&self) -> Option<U256> {
        self.tx.max_fee_per_blob_gas_u256()
    }

    fn priority_fee_or_price(&self) -> u128 {
        self.tx.priority_fee_or_price()
    }

    fn priority_fee_or_price_u256(&self) -> U256 {
        self.tx.priority_fee_or_price_u256()
    }

    fn effective_gas_price(&self, base_fee: Option<u64>) -> u128 {
        self.tx.effective_gas_price(base_fee)
    }

    fn effective_gas_price_u256(&self, base_fee: Option<u64>) -> U256 {
        self.tx.effective_gas_price_u256(base_fee)
    }

    fn is_dynamic_fee(&self) -> bool {
        self.tx.is_dynamic_fee()
    }

    fn kind(&self) -> TxKind {
        self.tx.kind()
    }

    fn is_create(&self) -> bool {
        self.tx.is_create()
    }

    fn value(&self) -> U256 {
        self.tx.value()
    }

    fn input(&self) -> &Bytes {
        self.tx.input()
    }

    fn access_list(&self) -> Option<&alloy_eips::eip2930::AccessList> {
        self.tx.access_list()
    }

    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        self.tx.blob_versioned_hashes()
    }

    fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
        self.tx.authorization_list()
    }
}

impl TxEip8141 {
    /// Get the transaction type.
    #[doc(alias = "transaction_type")]
    pub const fn tx_type() -> u8 {
        FRAME_TX_TYPE
    }

    /// Returns a borrowed view for validation and gas accounting.
    pub fn as_frame_ref(&self) -> TxEip8141Ref<'_> {
        TxEip8141Ref {
            sender: self.sender,
            frames: &self.frames,
            signatures: &self.signatures,
            fees: &self.fees,
            blob_versioned_hashes: &self.blob_versioned_hashes,
        }
    }

    /// Validates the structural constraints without executing a frame.
    pub fn validate(&self) -> Result<(), &'static str> {
        self.as_frame_ref().validate()
    }

    /// Outputs the length of the transaction's fields, without an RLP header.
    #[doc(hidden)]
    pub fn rlp_encoded_fields_length(&self) -> usize {
        self.chain_id.length()
            + self.nonce.length()
            + self.sender.length()
            + self.frames.length()
            + self.signatures.length()
            + self.fees.length()
            + self.blob_versioned_hashes.length()
    }

    /// Encodes only the transaction fields into the desired buffer, without an RLP header.
    pub fn rlp_encode_fields(&self, out: &mut dyn BufMut) {
        self.chain_id.encode(out);
        self.nonce.encode(out);
        self.sender.encode(out);
        self.frames.encode(out);
        self.signatures.encode(out);
        self.fees.encode(out);
        self.blob_versioned_hashes.encode(out);
    }

    /// Decodes the fields of the transaction from RLP bytes.
    pub fn rlp_decode_fields(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Ok(Self {
            chain_id: Decodable::decode(buf)?,
            nonce: Decodable::decode(buf)?,
            sender: Decodable::decode(buf)?,
            frames: Decodable::decode(buf)?,
            signatures: Decodable::decode(buf)?,
            fees: Decodable::decode(buf)?,
            blob_versioned_hashes: Decodable::decode(buf)?,
        })
    }

    /// Creates the RLP list header for the transaction payload.
    pub fn rlp_header(&self) -> Header {
        Header { list: true, payload_length: self.rlp_encoded_fields_length() }
    }

    /// Returns the transaction length when RLP encoded.
    pub fn rlp_encoded_length(&self) -> usize {
        self.rlp_header().length_with_payload()
    }

    /// RLP encodes the transaction payload.
    pub fn rlp_encode(&self, out: &mut dyn BufMut) {
        self.rlp_header().encode(out);
        self.rlp_encode_fields(out);
    }

    /// Returns the EIP-2718 encoded transaction length.
    pub fn eip2718_encoded_length(&self) -> usize {
        self.rlp_encoded_length() + 1
    }

    /// EIP-2718 encodes the transaction.
    pub fn eip2718_encode(&self, out: &mut dyn BufMut) {
        out.put_u8(Self::tx_type());
        self.rlp_encode(out);
    }

    /// Encodes the transaction for EIP-8141 signature hashing.
    ///
    /// Raw signature bytes are elided for signatures whose `msg` field is empty.
    pub fn encode_for_signing(&self, out: &mut dyn BufMut) {
        out.put_u8(Self::tx_type());
        let signatures = SigningFrameSignatures(&self.signatures);
        let payload_length = self.chain_id.length()
            + self.nonce.length()
            + self.sender.length()
            + self.frames.length()
            + signatures.length()
            + self.fees.length()
            + self.blob_versioned_hashes.length();
        Header { list: true, payload_length }.encode(out);
        self.chain_id.encode(out);
        self.nonce.encode(out);
        self.sender.encode(out);
        self.frames.encode(out);
        signatures.encode(out);
        self.fees.encode(out);
        self.blob_versioned_hashes.encode(out);
    }

    /// Returns the length of the EIP-8141 signature payload.
    pub fn payload_len_for_signature(&self) -> usize {
        let signatures = SigningFrameSignatures(&self.signatures);
        let payload_length = self.chain_id.length()
            + self.nonce.length()
            + self.sender.length()
            + self.frames.length()
            + signatures.length()
            + self.fees.length()
            + self.blob_versioned_hashes.length();
        1 + Header { list: true, payload_length }.length_with_payload()
    }

    /// Calculates the canonical EIP-8141 signature hash.
    pub fn signature_hash(&self) -> B256 {
        let mut buf = Vec::with_capacity(self.payload_len_for_signature());
        self.encode_for_signing(&mut buf);
        keccak256(buf)
    }

    /// Calculates the transaction hash.
    pub fn tx_hash(&self) -> B256 {
        let mut buf = Vec::with_capacity(self.eip2718_encoded_length());
        self.eip2718_encode(&mut buf);
        keccak256(buf)
    }

    /// Returns the first sender frame, if present.
    pub fn first_sender_frame(&self) -> Option<&Frame> {
        self.frames.iter().find(|frame| frame.mode == FrameMode::Sender)
    }

    /// Resolves a frame target against this transaction.
    ///
    /// An empty frame target resolves to the transaction sender. A malformed non-empty target
    /// returns `None`.
    pub fn resolve_frame_target(&self, frame: &Frame) -> Option<Address> {
        frame.target_address().or_else(|| frame.target.is_empty().then_some(self.sender))
    }

    /// Resolves the target for the frame at `index`.
    pub fn resolve_frame_target_at(&self, index: usize) -> Option<Address> {
        self.frames.get(index).and_then(|frame| self.resolve_frame_target(frame))
    }

    /// Returns whether the frame at `index` is an expiry verifier frame.
    pub fn is_expiry_verifier_frame(&self, index: usize) -> bool {
        self.frames.get(index).is_some_and(Frame::is_expiry_verifier)
    }

    /// See [`TxEip8141Ref::total_frame_gas_limit`].
    pub fn total_frame_gas_limit(&self) -> u64 {
        self.as_frame_ref().total_frame_gas_limit()
    }

    /// See [`TxEip8141Ref::total_frame_execution_gas_limit`].
    pub fn total_frame_execution_gas_limit(&self) -> u64 {
        self.as_frame_ref().total_frame_execution_gas_limit()
    }

    /// See [`TxEip8141Ref::total_frame_state_gas_limit`].
    pub fn total_frame_state_gas_limit(&self) -> u64 {
        self.as_frame_ref().total_frame_state_gas_limit()
    }

    /// See [`TxEip8141Ref::signature_verification_gas`].
    pub fn signature_verification_gas(&self) -> u64 {
        self.as_frame_ref().signature_verification_gas()
    }

    /// See [`TxEip8141Ref::value_transfer_gas`].
    pub fn value_transfer_gas(&self) -> u64 {
        self.as_frame_ref().value_transfer_gas()
    }

    /// See [`TxEip8141Ref::frame_calldata_tokens`].
    pub fn frame_calldata_tokens(&self) -> u64 {
        self.as_frame_ref().frame_calldata_tokens()
    }

    /// See [`TxEip8141Ref::frame_calldata_len`].
    pub fn frame_calldata_len(&self) -> u64 {
        self.as_frame_ref().frame_calldata_len()
    }

    /// See [`TxEip8141Ref::calculate_execution_gas_limit_with_token_cost`].
    pub fn calculate_execution_gas_limit_with_token_cost(&self, data_token_cost: u64) -> u64 {
        self.as_frame_ref().calculate_execution_gas_limit_with_token_cost(data_token_cost)
    }

    /// See [`TxEip8141Ref::calculate_gas_limit_with_token_cost`].
    pub fn calculate_gas_limit_with_token_cost(&self, data_token_cost: u64) -> u64 {
        self.as_frame_ref().calculate_gas_limit_with_token_cost(data_token_cost)
    }

    /// See [`TxEip8141Ref::calculate_gas_limit`].
    pub fn calculate_gas_limit(&self) -> u64 {
        self.as_frame_ref().calculate_gas_limit()
    }

    /// See [`TxEip8141Ref::calculate_calldata_floor`].
    pub fn calculate_calldata_floor(&self) -> u64 {
        self.as_frame_ref().calculate_calldata_floor()
    }

    /// Calculates a heuristic for the in-memory size of the [TxEip8141] transaction.
    #[inline]
    pub fn size(&self) -> usize {
        size_of::<Self>()
            + self.frames.capacity() * size_of::<Frame>()
            + self.signatures.capacity() * size_of::<FrameSignature>()
            + self.blob_versioned_hashes.capacity() * size_of::<B256>()
            + self.frames.iter().map(|frame| frame.target.len() + frame.data.len()).sum::<usize>()
            + self
                .signatures
                .iter()
                .map(|signature| {
                    signature.signer.len() + signature.msg.len() + signature.signature.len()
                })
                .sum::<usize>()
    }
}

/// Borrowed frame fields for allocation-free structural validation and gas accounting.
#[derive(Clone, Copy, Debug)]
pub struct TxEip8141Ref<'a> {
    /// Transaction sender.
    pub sender: Address,
    /// Frame list.
    pub frames: &'a [Frame],
    /// Signature list.
    pub signatures: &'a [FrameSignature],
    /// Fee parameters.
    pub fees: &'a TransactionFees,
    /// Blob hashes.
    pub blob_versioned_hashes: &'a [B256],
}

impl TxEip8141Ref<'_> {
    /// Validates the structural constraints that can be checked without executing a frame.
    ///
    /// Signature validity itself is checked by the selected frame validation scheme. This method
    /// only rejects malformed transactions that must not reach signing, pooling, or execution.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.frames.is_empty() || self.frames.len() > MAX_FRAMES {
            return Err("EIP-8141 transaction must contain between 1 and 64 frames");
        }
        if self.fees.max_priority_fee_per_gas > self.fees.max_fee_per_gas {
            return Err("max priority fee exceeds max fee");
        }
        if !self.blob_versioned_hashes.is_empty() {
            if self.fees.max_fee_per_blob_gas.is_zero() {
                return Err("blob hashes require a non-zero blob fee");
            }
            if self.blob_versioned_hashes.iter().any(|hash| hash[0] != 0x01) {
                return Err("invalid EIP-4844 versioned hash");
            }
        } else if !self.fees.max_fee_per_blob_gas.is_zero() {
            return Err("blob fee is non-zero without blob hashes");
        }

        for signature in self.signatures {
            match signature.scheme {
                SignatureScheme::Arbitrary => {
                    if !signature.signer.is_empty() {
                        return Err("arbitrary signatures must not contain signer metadata");
                    }
                }
                SignatureScheme::Secp256k1 => {
                    if !matches!(signature.signer.len(), 0 | 20) || signature.signature.len() != 65
                    {
                        return Err("invalid secp256k1 frame signature dimensions");
                    }
                }
                SignatureScheme::P256 => {
                    if !matches!(signature.signer.len(), 0 | 20) || signature.signature.len() != 128
                    {
                        return Err("invalid P-256 frame signature dimensions");
                    }
                }
            }
            if !signature.msg.is_empty()
                && (signature.msg.len() != 32 || signature.msg.iter().all(|byte| *byte == 0))
            {
                return Err("frame signature message must be empty or a non-zero 32-byte digest");
            }
        }

        let mut execution_gas = 0u64;
        let mut state_gas = 0u64;
        let mut expiry_verifiers = 0u8;
        for (index, frame) in self.frames.iter().enumerate() {
            if !frame.has_valid_target_encoding() {
                return Err("invalid EIP-8141 frame target");
            }
            if frame.flags & !FRAME_FLAGS_MASK != 0 {
                return Err("reserved EIP-8141 frame flag is set");
            }
            if !frame.value.is_zero() && frame.mode != FrameMode::Sender {
                return Err("frame value is only valid in sender mode");
            }
            if frame.allowed_scope() & u8::from(ApprovalScope::Execution) != 0
                && !frame.target.is_empty()
                && frame.target_address() != Some(self.sender)
            {
                return Err("execution approval target must resolve to the transaction sender");
            }
            if frame.is_atomic_batch()
                && (frame.mode == FrameMode::Verify
                    || index + 1 == self.frames.len()
                    || self.frames[index + 1].mode == FrameMode::Verify)
            {
                return Err("invalid atomic EIP-8141 frame");
            }
            let is_atomic_batch_member = frame.is_atomic_batch()
                || index
                    .checked_sub(1)
                    .is_some_and(|previous| self.frames[previous].is_atomic_batch());
            if is_atomic_batch_member && frame.allowed_scope() != 0 {
                return Err("atomic batch frames must not approve payment or execution");
            }
            if frame.is_expiry_verifier() {
                expiry_verifiers = expiry_verifiers.saturating_add(1);
                if expiry_verifiers > 1 {
                    return Err("EIP-8141 transaction may contain at most one expiry verifier");
                }
                if !frame.has_valid_expiry_verifier_fields() {
                    return Err("invalid expiry verifier frame");
                }
            }
            execution_gas = execution_gas
                .checked_add(frame.limits.execution)
                .ok_or("frame execution gas limit overflows u64")?;
            state_gas = state_gas
                .checked_add(frame.limits.state)
                .ok_or("frame state gas limit overflows u64")?;
        }

        let _ = execution_gas.checked_add(state_gas).ok_or("frame gas limit overflows u64")?;
        let _ = self
            .signature_verification_gas()
            .checked_add(self.value_transfer_gas())
            .ok_or("frame intrinsic gas overflows u64")?;
        if self
            .calculate_execution_gas_limit_with_token_cost(FRAME_TX_DATA_TOKEN_STANDARD_COST)
            .max(self.calculate_calldata_floor())
            > MAX_TX_GAS_LIMIT_OSAKA
        {
            return Err("EIP-8141 transaction exceeds the EIP-7825 execution gas cap");
        }
        Ok(())
    }

    /// Returns the sum of all frame gas limits.
    pub fn total_frame_gas_limit(&self) -> u64 {
        self.frames.iter().fold(0u64, |acc, frame| {
            acc.saturating_add(frame.limits.execution.saturating_add(frame.limits.state))
        })
    }

    /// Returns the sum of all frame execution gas limits.
    pub fn total_frame_execution_gas_limit(&self) -> u64 {
        self.frames.iter().fold(0u64, |acc, frame| acc.saturating_add(frame.limits.execution))
    }

    /// Returns the sum of all frame state gas limits.
    pub fn total_frame_state_gas_limit(&self) -> u64 {
        self.frames.iter().fold(0u64, |acc, frame| acc.saturating_add(frame.limits.state))
    }

    /// Returns the gas charged for protocol validation of all signature entries.
    pub fn signature_verification_gas(&self) -> u64 {
        self.signatures
            .iter()
            .fold(0u64, |acc, signature| acc.saturating_add(signature.verification_gas()))
    }

    /// Returns the intrinsic value-transfer cost for frames with an explicit non-sender target.
    pub fn value_transfer_gas(&self) -> u64 {
        self.frames.iter().fold(0u64, |acc, frame| {
            let costs = if !frame.value.is_zero()
                && !frame.target.is_empty()
                && frame.target_address() != Some(self.sender)
            {
                TX_VALUE_COST
            } else {
                0
            };
            acc.saturating_add(costs)
        })
    }

    /// Returns the EIP-7623 token count of the frame transaction's charged byte fields.
    ///
    /// Only frame data and signature signer, message, and signature bytes are charged. RLP
    /// headers and fixed-size frame and signature fields are covered by the intrinsic and
    /// per-frame costs. Zero bytes count as one token and non-zero bytes count as four tokens.
    pub fn frame_calldata_tokens(&self) -> u64 {
        let frame_tokens = self
            .frames
            .iter()
            .fold(0u64, |acc, frame| acc.saturating_add(count_frame_data_tokens(&frame.data)));
        self.signatures.iter().fold(frame_tokens, |acc, signature| {
            acc.saturating_add(count_frame_data_tokens(&signature.signer))
                .saturating_add(count_frame_data_tokens(&signature.msg))
                .saturating_add(count_frame_data_tokens(&signature.signature))
        })
    }

    /// Returns the byte length of the frame transaction's charged byte fields.
    pub fn frame_calldata_len(&self) -> u64 {
        let frame_data_len =
            self.frames.iter().fold(0u64, |acc, frame| acc.saturating_add(frame.data.len() as u64));
        self.signatures.iter().fold(frame_data_len, |acc, signature| {
            acc.saturating_add(signature.signer.len() as u64)
                .saturating_add(signature.msg.len() as u64)
                .saturating_add(signature.signature.len() as u64)
        })
    }

    /// Calculates the execution gas portion with the provided calldata token gas cost.
    pub fn calculate_execution_gas_limit_with_token_cost(&self, data_token_cost: u64) -> u64 {
        FRAME_TX_INTRINSIC_COST
            .saturating_add((self.frames.len() as u64).saturating_mul(FRAME_TX_PER_FRAME_COST))
            .saturating_add(self.frame_calldata_tokens().saturating_mul(data_token_cost))
            .saturating_add(self.signature_verification_gas())
            .saturating_add(self.value_transfer_gas())
            .saturating_add(self.total_frame_execution_gas_limit())
    }

    /// Calculates the frame transaction gas limit with the provided calldata token gas cost.
    pub fn calculate_gas_limit_with_token_cost(&self, data_token_cost: u64) -> u64 {
        self.calculate_execution_gas_limit_with_token_cost(data_token_cost)
            .saturating_add(self.total_frame_state_gas_limit())
    }

    /// Calculates the derived total gas limit of this frame transaction.
    pub fn calculate_gas_limit(&self) -> u64 {
        let standard = self.calculate_gas_limit_with_token_cost(FRAME_TX_DATA_TOKEN_STANDARD_COST);
        standard
            .max(self.calculate_calldata_floor().saturating_add(self.total_frame_state_gas_limit()))
    }

    /// Calculates the calldata floor gas for this frame transaction.
    pub fn calculate_calldata_floor(&self) -> u64 {
        FRAME_TX_INTRINSIC_COST
            .saturating_add((self.frames.len() as u64).saturating_mul(FRAME_TX_PER_FRAME_COST))
            .saturating_add(self.signature_verification_gas())
            .saturating_add(self.value_transfer_gas())
            // EIP-7976 charges every calldata byte as four floor tokens.
            .saturating_add(
                self.frame_calldata_len()
                    .saturating_mul(4)
                    .saturating_mul(FRAME_TX_TOTAL_COST_FLOOR_PER_TOKEN),
            )
    }
}

impl Typed2718 for TxEip8141 {
    fn ty(&self) -> u8 {
        Self::tx_type()
    }
}

impl IsTyped2718 for TxEip8141 {
    fn is_type(type_id: u8) -> bool {
        matches!(type_id, FRAME_TX_TYPE)
    }
}

impl Sealable for TxEip8141 {
    fn hash_slow(&self) -> B256 {
        self.tx_hash()
    }
}

impl Encodable2718 for TxEip8141 {
    fn encode_2718_len(&self) -> usize {
        self.eip2718_encoded_length()
    }

    fn encode_2718(&self, out: &mut dyn BufMut) {
        self.eip2718_encode(out);
    }
}

impl Decodable2718 for TxEip8141 {
    fn typed_decode(ty: u8, buf: &mut &[u8]) -> Eip2718Result<Self> {
        if ty != Self::tx_type() {
            return Err(Eip2718Error::UnexpectedType(ty));
        }

        Self::decode(buf).map_err(Into::into)
    }

    fn fallback_decode(_buf: &mut &[u8]) -> Eip2718Result<Self> {
        Err(Eip2718Error::UnexpectedType(Self::tx_type()))
    }
}

impl Transaction for TxEip8141 {
    fn frame_transaction(&self) -> Option<&TxEip8141> {
        Some(self)
    }

    #[inline]
    fn chain_id(&self) -> Option<ChainId> {
        Some(self.chain_id)
    }

    #[inline]
    fn nonce(&self) -> u64 {
        self.nonce
    }

    #[inline]
    fn gas_limit(&self) -> u64 {
        self.calculate_gas_limit()
    }

    #[inline]
    fn gas_price(&self) -> Option<u128> {
        None
    }

    #[inline]
    fn max_fee_per_gas(&self) -> u128 {
        self.fees.max_fee_per_gas.saturating_to()
    }

    fn max_fee_per_gas_u256(&self) -> U256 {
        self.fees.max_fee_per_gas
    }

    #[inline]
    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        Some(self.fees.max_priority_fee_per_gas.saturating_to())
    }

    fn max_priority_fee_per_gas_u256(&self) -> Option<U256> {
        Some(self.fees.max_priority_fee_per_gas)
    }

    #[inline]
    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        Some(self.fees.max_fee_per_blob_gas.saturating_to())
    }

    fn max_fee_per_blob_gas_u256(&self) -> Option<U256> {
        Some(self.fees.max_fee_per_blob_gas)
    }

    #[inline]
    fn priority_fee_or_price(&self) -> u128 {
        self.fees.max_priority_fee_per_gas.saturating_to()
    }

    fn priority_fee_or_price_u256(&self) -> U256 {
        self.fees.max_priority_fee_per_gas
    }

    fn effective_gas_price(&self, base_fee: Option<u64>) -> u128 {
        alloy_eips::eip1559::calc_effective_gas_price(
            self.fees.max_fee_per_gas.saturating_to(),
            self.fees.max_priority_fee_per_gas.saturating_to(),
            base_fee,
        )
    }

    fn effective_gas_price_u256(&self, base_fee: Option<u64>) -> U256 {
        base_fee.map_or(self.fees.max_fee_per_gas, |base_fee| {
            self.fees
                .max_fee_per_gas
                .min(U256::from(base_fee).saturating_add(self.fees.max_priority_fee_per_gas))
        })
    }

    #[inline]
    fn is_dynamic_fee(&self) -> bool {
        true
    }

    #[inline]
    fn kind(&self) -> TxKind {
        self.sender.into()
    }

    #[inline]
    fn is_create(&self) -> bool {
        false
    }

    #[inline]
    fn value(&self) -> U256 {
        U256::ZERO
    }

    #[inline]
    fn input(&self) -> &Bytes {
        &EMPTY_INPUT
    }

    #[inline]
    fn access_list(&self) -> Option<&alloy_eips::eip2930::AccessList> {
        None
    }

    #[inline]
    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        Some(&self.blob_versioned_hashes)
    }

    #[inline]
    fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
        None
    }
}

impl Encodable for TxEip8141 {
    fn encode(&self, out: &mut dyn BufMut) {
        self.rlp_encode(out);
    }

    fn length(&self) -> usize {
        self.rlp_encoded_length()
    }
}

impl Decodable for TxEip8141 {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let header = Header::decode(buf)?;
        if !header.list {
            return Err(alloy_rlp::Error::UnexpectedString);
        }

        let remaining = buf.len();
        let this = Self::rlp_decode_fields(buf)?;

        if buf.len() + header.payload_length != remaining {
            return Err(alloy_rlp::Error::UnexpectedLength);
        }

        Ok(this)
    }
}

/// Bincode-compatible [`TxEip8141`] serde implementation.
#[cfg(all(feature = "serde", feature = "serde-bincode-compat"))]
pub(super) mod serde_bincode_compat {
    use alloc::borrow::Cow;
    use alloy_eips::eip8141::{Frame, FrameSignature, TransactionFees};
    use alloy_primitives::{Address, ChainId, B256};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde_with::{DeserializeAs, SerializeAs};

    /// Bincode-compatible [`super::TxEip8141`] serde implementation.
    ///
    /// Intended to use with the [`serde_with::serde_as`] macro in the following way:
    /// ```rust
    /// use alloy_consensus::{serde_bincode_compat, TxEip8141};
    /// use serde::{Deserialize, Serialize};
    /// use serde_with::serde_as;
    ///
    /// #[serde_as]
    /// #[derive(Serialize, Deserialize)]
    /// struct Data {
    ///     #[serde_as(as = "serde_bincode_compat::transaction::TxEip8141")]
    ///     transaction: TxEip8141,
    /// }
    /// ```
    #[derive(Debug, Serialize, Deserialize)]
    pub struct TxEip8141<'a> {
        chain_id: ChainId,
        nonce: u64,
        sender: Address,
        frames: Cow<'a, [Frame]>,
        signatures: Cow<'a, [FrameSignature]>,
        fees: TransactionFees,
        blob_versioned_hashes: Cow<'a, [B256]>,
    }

    impl<'a> From<&'a super::TxEip8141> for TxEip8141<'a> {
        fn from(value: &'a super::TxEip8141) -> Self {
            Self {
                chain_id: value.chain_id,
                nonce: value.nonce,
                sender: value.sender,
                frames: Cow::Borrowed(value.frames.as_slice()),
                signatures: Cow::Borrowed(value.signatures.as_slice()),
                fees: value.fees,
                blob_versioned_hashes: Cow::Borrowed(value.blob_versioned_hashes.as_slice()),
            }
        }
    }

    impl<'a> From<TxEip8141<'a>> for super::TxEip8141 {
        fn from(value: TxEip8141<'a>) -> Self {
            Self {
                chain_id: value.chain_id,
                nonce: value.nonce,
                sender: value.sender,
                frames: value.frames.into_owned(),
                signatures: value.signatures.into_owned(),
                fees: value.fees,
                blob_versioned_hashes: value.blob_versioned_hashes.into_owned(),
            }
        }
    }

    impl SerializeAs<super::TxEip8141> for TxEip8141<'_> {
        fn serialize_as<S>(source: &super::TxEip8141, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            TxEip8141::from(source).serialize(serializer)
        }
    }

    impl<'de> DeserializeAs<'de, super::TxEip8141> for TxEip8141<'de> {
        fn deserialize_as<D>(deserializer: D) -> Result<super::TxEip8141, D::Error>
        where
            D: Deserializer<'de>,
        {
            TxEip8141::deserialize(deserializer).map(Into::into)
        }
    }

    #[cfg(test)]
    mod tests {
        use arbitrary::Arbitrary;
        use bincode::config;
        use rand::Rng;
        use serde::{Deserialize, Serialize};
        use serde_with::serde_as;

        use super::super::{serde_bincode_compat, TxEip8141};

        #[test]
        fn test_tx_eip8141_bincode_roundtrip() {
            #[serde_as]
            #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
            struct Data {
                #[serde_as(as = "serde_bincode_compat::TxEip8141")]
                transaction: TxEip8141,
            }

            let mut bytes = [0u8; 1024];
            rand::thread_rng().fill(bytes.as_mut_slice());
            let data = Data {
                transaction: TxEip8141::arbitrary(&mut arbitrary::Unstructured::new(&bytes))
                    .unwrap(),
            };

            let encoded = bincode::serde::encode_to_vec(&data, config::legacy()).unwrap();
            let (decoded, _) =
                bincode::serde::decode_from_slice::<Data, _>(&encoded, config::legacy()).unwrap();
            assert_eq!(decoded, data);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EthereumTxEnvelope, TxEip4844};
    use alloy_eips::eip8141::{
        constants::{ATOMIC_BATCH_FLAG, EXPIRY_DATA_LENGTH, EXPIRY_VERIFIER},
        ApprovalScope, FrameLimits, FrameMode, SignatureScheme, TransactionFees,
    };
    use alloy_primitives::{Address, Bytes, U256};

    fn valid_tx() -> TxEip8141 {
        TxEip8141 { frames: vec![Frame::default()], ..Default::default() }
    }

    #[test]
    fn validates_protocol_signature_dimensions() {
        let mut tx = valid_tx();
        tx.signatures.push(FrameSignature {
            scheme: SignatureScheme::Secp256k1,
            signer: Bytes::new(),
            msg: Bytes::new(),
            signature: Bytes::from(vec![0x11; 65]),
        });
        assert!(tx.validate().is_ok());

        tx.signatures[0] = FrameSignature {
            scheme: SignatureScheme::P256,
            signer: Bytes::from(vec![0x22; 20]),
            msg: Bytes::new(),
            signature: Bytes::from(vec![0x33; 128]),
        };
        assert!(tx.validate().is_ok());

        tx.signatures[0].signature = Bytes::from(vec![0x33; 64]);
        assert!(tx.validate().is_err());
    }

    #[test]
    fn validates_execution_approval_and_atomic_batch_constraints() {
        let mut tx = valid_tx();
        tx.sender = Address::repeat_byte(0x11);
        tx.frames[0].flags = ApprovalScope::Execution.into();
        tx.frames[0].target = Bytes::copy_from_slice(Address::repeat_byte(0x22).as_slice());
        assert!(tx.validate().is_err());

        tx.frames[0].target = Bytes::copy_from_slice(tx.sender.as_slice());
        assert!(tx.validate().is_ok());

        tx.frames = vec![
            Frame { flags: ATOMIC_BATCH_FLAG, ..Default::default() },
            Frame { flags: ApprovalScope::Payment.into(), ..Default::default() },
        ];
        assert!(tx.validate().is_err());

        tx.frames[1].flags = 0;
        tx.frames[1].mode = FrameMode::Verify;
        assert!(tx.validate().is_err());
    }

    #[test]
    fn rejects_multiple_expiry_verifiers() {
        let expiry = Frame {
            mode: FrameMode::Verify,
            target: Bytes::copy_from_slice(EXPIRY_VERIFIER.as_slice()),
            data: Bytes::from(vec![0; EXPIRY_DATA_LENGTH]),
            ..Default::default()
        };
        let tx = TxEip8141 { frames: vec![expiry.clone(), expiry], ..Default::default() };

        assert!(tx.validate().is_err());
    }

    #[test]
    fn enforces_eip7825_execution_gas_cap() {
        let mut tx = valid_tx();
        let intrinsic =
            tx.calculate_execution_gas_limit_with_token_cost(FRAME_TX_DATA_TOKEN_STANDARD_COST);
        tx.frames[0].limits.execution = MAX_TX_GAS_LIMIT_OSAKA - intrinsic;
        assert!(tx.validate().is_ok());

        tx.frames[0].limits.execution += 1;
        assert!(tx.validate().is_err());
    }

    #[test]
    fn encode_decode_roundtrip() {
        let tx = TxEip8141 {
            chain_id: 1,
            nonce: 7,
            sender: Address::from([0x11; 20]),
            frames: vec![Frame {
                mode: FrameMode::Verify,
                flags: ApprovalScope::ExecutionAndPayment.into(),
                target: Bytes::new(),
                limits: FrameLimits { execution: 21_000, state: 0 },
                value: U256::ZERO,
                data: Bytes::new(),
            }],
            signatures: vec![FrameSignature {
                scheme: SignatureScheme::Secp256k1,
                signer: Bytes::copy_from_slice(&[0x11; 20]),
                msg: Bytes::new(),
                signature: Bytes::copy_from_slice(&[0x22; 65]),
            }],
            fees: TransactionFees {
                max_priority_fee_per_gas: U256::from(1),
                max_fee_per_gas: U256::from(10),
                max_fee_per_blob_gas: U256::ZERO,
            },
            blob_versioned_hashes: Vec::new(),
        };

        let mut buf = Vec::new();
        tx.encode(&mut buf);
        let decoded = TxEip8141::decode(&mut buf.as_ref()).unwrap();

        assert_eq!(buf.len(), tx.length());
        assert_eq!(decoded, tx);
    }

    #[test]
    fn signature_hash_elides_transaction_hash_signatures() {
        let mut tx = TxEip8141 {
            chain_id: 1,
            nonce: 0,
            sender: Address::from([0x11; 20]),
            frames: Vec::new(),
            signatures: vec![FrameSignature {
                scheme: SignatureScheme::Arbitrary,
                signer: Bytes::new(),
                msg: Bytes::new(),
                signature: Bytes::copy_from_slice(&[0x22; 32]),
            }],
            fees: TransactionFees {
                max_priority_fee_per_gas: U256::from(1),
                max_fee_per_gas: U256::from(10),
                max_fee_per_blob_gas: U256::ZERO,
            },
            blob_versioned_hashes: Vec::new(),
        };

        let first = tx.signature_hash();
        tx.signatures[0].signature = Bytes::copy_from_slice(&[0x33; 64]);
        let second = tx.signature_hash();

        assert_eq!(first, second);
    }

    #[test]
    fn signing_encoding_elides_only_transaction_hash_signatures() {
        let tx = TxEip8141 {
            signatures: vec![
                FrameSignature {
                    scheme: SignatureScheme::Arbitrary,
                    signer: Bytes::new(),
                    msg: Bytes::new(),
                    signature: Bytes::copy_from_slice(&[0x11; 64]),
                },
                FrameSignature {
                    scheme: SignatureScheme::Arbitrary,
                    signer: Bytes::new(),
                    msg: Bytes::copy_from_slice(&[0x22; 32]),
                    signature: Bytes::copy_from_slice(&[0x33; 64]),
                },
            ],
            ..Default::default()
        };
        let mut reference = tx.clone();
        reference.signatures[0].signature = Bytes::new();

        let mut expected = Vec::new();
        reference.eip2718_encode(&mut expected);
        let mut actual = Vec::new();
        tx.encode_for_signing(&mut actual);

        assert_eq!(actual, expected);
        assert_eq!(tx.payload_len_for_signature(), actual.len());
    }

    #[test]
    fn exposes_full_width_fee_values_through_wrappers() {
        let high = U256::from(u128::MAX) + U256::from(1);
        let tx = TxEip8141 {
            fees: TransactionFees {
                max_priority_fee_per_gas: high + U256::from(20),
                max_fee_per_gas: high + U256::from(100),
                max_fee_per_blob_gas: high + U256::from(200),
            },
            ..Default::default()
        };
        let expected_effective = high + U256::from(30);

        assert_eq!(tx.max_fee_per_gas(), u128::MAX);
        assert_eq!(tx.max_fee_per_gas_u256(), tx.fees.max_fee_per_gas);
        assert_eq!(tx.max_priority_fee_per_gas_u256(), Some(tx.fees.max_priority_fee_per_gas));
        assert_eq!(tx.max_fee_per_blob_gas_u256(), Some(tx.fees.max_fee_per_blob_gas));
        assert_eq!(tx.effective_gas_price_u256(Some(10)), expected_effective);
        assert_eq!(tx.effective_tip_per_gas_u256(10), Some(tx.fees.max_priority_fee_per_gas));

        let variant = TxEip8141Variant::<()>::from(tx.clone());
        assert_eq!(variant.effective_gas_price_u256(Some(10)), expected_effective);

        let with_sidecar = TxEip8141WithSidecar::new(tx.clone(), ());
        assert_eq!(with_sidecar.effective_gas_price_u256(Some(10)), expected_effective);

        let envelope: EthereumTxEnvelope<TxEip4844> = EthereumTxEnvelope::Eip8141(tx.seal_slow());
        assert_eq!(envelope.effective_gas_price_u256(Some(10)), expected_effective);
    }

    #[test]
    fn size_includes_nested_frame_and_signature_bytes() {
        let empty = TxEip8141 {
            frames: vec![Frame::default()],
            signatures: vec![FrameSignature::default()],
            ..Default::default()
        };
        let mut populated = empty.clone();
        populated.frames[0].target = Bytes::copy_from_slice(&[0x11; 20]);
        populated.frames[0].data = Bytes::copy_from_slice(&[0x22; 128]);
        populated.signatures[0].signer = Bytes::copy_from_slice(&[0x33; 20]);
        populated.signatures[0].msg = Bytes::copy_from_slice(&[0x44; 32]);
        populated.signatures[0].signature = Bytes::copy_from_slice(&[0x55; 65]);

        assert_eq!(populated.size() - empty.size(), 20 + 128 + 20 + 32 + 65);
    }

    #[test]
    fn helpers_resolve_targets_and_expose_sender_anchor() {
        let sender = Address::from([0x11; 20]);
        let target = Address::from([0x22; 20]);
        let tx = TxEip8141 {
            sender,
            frames: vec![
                Frame { mode: FrameMode::Default, target: Bytes::new(), ..Default::default() },
                Frame {
                    mode: FrameMode::Sender,
                    target: Bytes::copy_from_slice(target.as_slice()),
                    data: Bytes::copy_from_slice(&[0xaa, 0xbb]),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        assert_eq!(tx.resolve_frame_target_at(0), Some(sender));
        assert_eq!(tx.resolve_frame_target_at(1), Some(target));
        assert_eq!(tx.kind(), TxKind::Call(sender));
        assert!(tx.input().is_empty());
    }

    #[test]
    fn calculates_frame_transaction_gas_limit() {
        let tx = TxEip8141 {
            frames: vec![
                Frame {
                    limits: FrameLimits { execution: 10, state: 3 },
                    data: Bytes::copy_from_slice(&[0, 1]),
                    ..Default::default()
                },
                Frame {
                    limits: FrameLimits { execution: 20, state: 4 },
                    data: Bytes::copy_from_slice(&[2]),
                    ..Default::default()
                },
            ],
            signatures: vec![FrameSignature {
                scheme: SignatureScheme::Secp256k1,
                signer: Bytes::copy_from_slice(&[0x11; 20]),
                msg: Bytes::new(),
                signature: Bytes::copy_from_slice(&[0x22; 65]),
            }],
            ..Default::default()
        };

        let calldata_tokens = count_frame_data_tokens(&[0, 1, 2])
            + count_frame_data_tokens(&[0x11; 20])
            + count_frame_data_tokens(&[0x22; 65]);
        let calldata_len = 3 + 20 + 65;
        let expected = FRAME_TX_INTRINSIC_COST
            + 2 * FRAME_TX_PER_FRAME_COST
            + calldata_tokens * FRAME_TX_DATA_TOKEN_STANDARD_COST
            + 2_800
            + 37;

        assert_eq!(tx.total_frame_gas_limit(), 37);
        assert_eq!(tx.total_frame_execution_gas_limit(), 30);
        assert_eq!(tx.total_frame_state_gas_limit(), 7);
        assert_eq!(tx.signature_verification_gas(), 2_800);
        assert_eq!(tx.frame_calldata_tokens(), calldata_tokens);
        assert_eq!(tx.frame_calldata_len(), calldata_len);
        let floor = FRAME_TX_INTRINSIC_COST
            + 2 * FRAME_TX_PER_FRAME_COST
            + 2_800
            + calldata_len * 4 * FRAME_TX_TOTAL_COST_FLOOR_PER_TOKEN;
        let floor_with_state = floor + tx.total_frame_state_gas_limit();
        assert_eq!(tx.calculate_gas_limit(), expected.max(floor_with_state));
        assert_eq!(tx.gas_limit(), expected.max(floor_with_state));
        assert_eq!(tx.calculate_calldata_floor(), floor);
    }

    #[test]
    fn fixed_fields_and_rlp_headers_are_not_charged_as_calldata() {
        let tx = TxEip8141 {
            frames: vec![Frame {
                mode: FrameMode::Sender,
                flags: ApprovalScope::ExecutionAndPayment.into(),
                target: Bytes::copy_from_slice(Address::repeat_byte(0x44).as_slice()),
                limits: FrameLimits { execution: u64::MAX, state: u64::MAX },
                value: U256::MAX,
                data: Bytes::new(),
            }],
            signatures: Vec::new(),
            ..Default::default()
        };

        assert_eq!(tx.frame_calldata_tokens(), 0);
        assert_eq!(tx.frame_calldata_len(), 0);
        assert_eq!(
            tx.calculate_calldata_floor(),
            FRAME_TX_INTRINSIC_COST + FRAME_TX_PER_FRAME_COST + TX_VALUE_COST
        );
    }
}
