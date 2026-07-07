use alloc::vec::Vec;

use alloy_eips::{
    eip2718::{Eip2718Error, Eip2718Result, IsTyped2718},
    eip7702::SignedAuthorization,
    eip8141::{
        constants::{FRAME_TX_INTRINSIC_COST, FRAME_TX_PER_FRAME_COST, FRAME_TX_TYPE},
        Frame, FrameMode, FrameSignature,
    },
    Decodable2718, Encodable2718, Typed2718,
};
use alloy_primitives::{keccak256, Address, Bytes, ChainId, Sealable, TxKind, B256, U256};
use alloy_rlp::{BufMut, Decodable, Encodable, Header};

use crate::Transaction;

static EMPTY_INPUT: Bytes = Bytes::new();

/// Standard gas charged per frame transaction calldata token.
///
/// This matches `GasCosts.TX_DATA_TOKEN_STANDARD` in the execution-specs EIP-8141 draft.
pub const FRAME_TX_DATA_TOKEN_STANDARD_COST: u64 = 4;

/// Floor gas charged per frame transaction calldata token.
///
/// This matches `GasCosts.TX_DATA_TOKEN_FLOOR` in the execution-specs EIP-8141 draft.
pub const FRAME_TX_DATA_TOKEN_FLOOR_COST: u64 = 16;

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
    /// Max priority fee per gas.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub max_priority_fee_per_gas: u128,
    /// Max fee per gas.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub max_fee_per_gas: u128,
    /// Max fee per blob gas.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub max_fee_per_blob_gas: u128,
    /// Blob versioned hashes.
    pub blob_versioned_hashes: Vec<B256>,
}

impl TxEip8141 {
    /// Get the transaction type.
    #[doc(alias = "transaction_type")]
    pub const fn tx_type() -> u8 {
        FRAME_TX_TYPE
    }

    /// Outputs the length of the transaction's fields, without an RLP header.
    #[doc(hidden)]
    pub fn rlp_encoded_fields_length(&self) -> usize {
        self.chain_id.length()
            + self.nonce.length()
            + self.sender.length()
            + self.frames.length()
            + self.signatures.length()
            + self.max_priority_fee_per_gas.length()
            + self.max_fee_per_gas.length()
            + self.max_fee_per_blob_gas.length()
            + self.blob_versioned_hashes.length()
    }

    /// Encodes only the transaction fields into the desired buffer, without an RLP header.
    pub fn rlp_encode_fields(&self, out: &mut dyn BufMut) {
        self.chain_id.encode(out);
        self.nonce.encode(out);
        self.sender.encode(out);
        self.frames.encode(out);
        self.signatures.encode(out);
        self.max_priority_fee_per_gas.encode(out);
        self.max_fee_per_gas.encode(out);
        self.max_fee_per_blob_gas.encode(out);
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
            max_priority_fee_per_gas: Decodable::decode(buf)?,
            max_fee_per_gas: Decodable::decode(buf)?,
            max_fee_per_blob_gas: Decodable::decode(buf)?,
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

        let mut tx = self.clone();
        for signature in &mut tx.signatures {
            if signature.msg.is_empty() {
                signature.signature = Bytes::new();
            }
        }
        tx.rlp_encode(out);
    }

    /// Returns the length of the EIP-8141 signature payload.
    pub fn payload_len_for_signature(&self) -> usize {
        self.eip2718_encoded_length()
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

    /// Returns the sum of all frame gas limits.
    pub fn total_frame_gas_limit(&self) -> u64 {
        self.frames.iter().fold(0u64, |acc, frame| acc.saturating_add(frame.gas_limit))
    }

    /// Returns the gas charged for protocol validation of all signature entries.
    pub fn signature_verification_gas(&self) -> u64 {
        self.signatures
            .iter()
            .fold(0u64, |acc, signature| acc.saturating_add(signature.verification_gas()))
    }

    /// Returns the EIP-7623/EIP-7976-style token count of encoded frame transaction data.
    ///
    /// The encoded signature list and frame list are counted. Zero bytes count as one token and
    /// non-zero bytes count as four tokens.
    pub fn frame_calldata_tokens(&self) -> u64 {
        let mut encoded = Vec::new();
        self.signatures.encode(&mut encoded);
        self.frames.encode(&mut encoded);
        count_frame_data_tokens(&encoded)
    }

    /// Calculates the frame transaction gas limit with the provided calldata token gas cost.
    pub fn calculate_gas_limit_with_token_cost(&self, data_token_cost: u64) -> u64 {
        FRAME_TX_INTRINSIC_COST
            .saturating_add((self.frames.len() as u64).saturating_mul(FRAME_TX_PER_FRAME_COST))
            .saturating_add(self.frame_calldata_tokens().saturating_mul(data_token_cost))
            .saturating_add(self.signature_verification_gas())
            .saturating_add(self.total_frame_gas_limit())
    }

    /// Calculates the derived total gas limit of this frame transaction.
    pub fn calculate_gas_limit(&self) -> u64 {
        self.calculate_gas_limit_with_token_cost(FRAME_TX_DATA_TOKEN_STANDARD_COST)
    }

    /// Calculates the calldata floor gas for this frame transaction.
    pub fn calculate_calldata_floor(&self) -> u64 {
        FRAME_TX_INTRINSIC_COST.saturating_add(
            self.frame_calldata_tokens().saturating_mul(FRAME_TX_DATA_TOKEN_FLOOR_COST),
        )
    }

    /// Calculates a heuristic for the in-memory size of the [TxEip8141] transaction.
    #[inline]
    pub fn size(&self) -> usize {
        size_of::<Self>()
            + self.frames.capacity() * size_of::<Frame>()
            + self.signatures.capacity() * size_of::<FrameSignature>()
            + self.blob_versioned_hashes.capacity() * size_of::<B256>()
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
        self.max_fee_per_gas
    }

    #[inline]
    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        Some(self.max_priority_fee_per_gas)
    }

    #[inline]
    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        Some(self.max_fee_per_blob_gas)
    }

    #[inline]
    fn priority_fee_or_price(&self) -> u128 {
        self.max_priority_fee_per_gas
    }

    fn effective_gas_price(&self, base_fee: Option<u64>) -> u128 {
        alloy_eips::eip1559::calc_effective_gas_price(
            self.max_fee_per_gas,
            self.max_priority_fee_per_gas,
            base_fee,
        )
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

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_eips::eip8141::{ApprovalScope, FrameMode, SignatureScheme};
    use alloy_primitives::{Address, Bytes, U256};

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
                gas_limit: 21_000,
                value: U256::ZERO,
                data: Bytes::new(),
            }],
            signatures: vec![FrameSignature {
                scheme: SignatureScheme::Secp256k1,
                signer: Bytes::copy_from_slice(&[0x11; 20]),
                msg: Bytes::new(),
                signature: Bytes::copy_from_slice(&[0x22; 65]),
            }],
            max_priority_fee_per_gas: 1,
            max_fee_per_gas: 10,
            max_fee_per_blob_gas: 0,
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
            max_priority_fee_per_gas: 1,
            max_fee_per_gas: 10,
            max_fee_per_blob_gas: 0,
            blob_versioned_hashes: Vec::new(),
        };

        let first = tx.signature_hash();
        tx.signatures[0].signature = Bytes::copy_from_slice(&[0x33; 64]);
        let second = tx.signature_hash();

        assert_eq!(first, second);
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
                    gas_limit: 10,
                    data: Bytes::copy_from_slice(&[0, 1]),
                    ..Default::default()
                },
                Frame { gas_limit: 20, data: Bytes::copy_from_slice(&[2]), ..Default::default() },
            ],
            signatures: vec![FrameSignature {
                scheme: SignatureScheme::Secp256k1,
                signer: Bytes::copy_from_slice(&[0x11; 20]),
                msg: Bytes::new(),
                signature: Bytes::copy_from_slice(&[0x22; 65]),
            }],
            ..Default::default()
        };

        let mut encoded = Vec::new();
        tx.signatures.encode(&mut encoded);
        tx.frames.encode(&mut encoded);
        let calldata_tokens = count_frame_data_tokens(&encoded);
        let expected = FRAME_TX_INTRINSIC_COST
            + 2 * FRAME_TX_PER_FRAME_COST
            + calldata_tokens * FRAME_TX_DATA_TOKEN_STANDARD_COST
            + 2_800
            + 30;

        assert_eq!(tx.total_frame_gas_limit(), 30);
        assert_eq!(tx.signature_verification_gas(), 2_800);
        assert_eq!(tx.frame_calldata_tokens(), calldata_tokens);
        assert_eq!(tx.calculate_gas_limit(), expected);
        assert_eq!(tx.gas_limit(), expected);
    }
}
