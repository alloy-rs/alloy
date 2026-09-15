//! Standalone EIP-8141 frame transaction encoding and validation.

use alloc::vec::Vec;
use alloy_eips::{
    eip2718::{Decodable2718, Eip2718Error, Eip2718Result, Encodable2718, IsTyped2718},
    eip4844::VERSIONED_HASH_VERSION_KZG,
    eip7594::MAX_BLOBS_PER_TX_FUSAKA,
    eip7623::tokens_in_calldata,
    eip7825::MAX_TX_GAS_LIMIT_OSAKA,
    eip8141::{
        constants::{
            FRAME_TX_DATA_TOKEN_STANDARD_COST, FRAME_TX_INTRINSIC_COST, FRAME_TX_PER_FRAME_COST,
            FRAME_TX_TOTAL_COST_FLOOR_PER_TOKEN, FRAME_TX_TYPE, MAX_FRAMES, TX_VALUE_COST,
        },
        ApprovalScope, Eip8141Error, Frame, FrameLimits, FrameMode, FrameSignature,
        TransactionFees,
    },
    Typed2718,
};
use alloy_primitives::{keccak256, Address, Bytes, ChainId, Sealable, TxKind, B256, U256};
use alloy_rlp::{BufMut, Decodable, Encodable, Header, RlpDecodable, RlpEncodable};

use super::Transaction;

/// A standalone [EIP-8141] frame transaction.
///
/// Signatures are carried in the transaction itself. RLP encodes the payload, while
/// [`Encodable2718`] adds the transaction type byte. Decoding does not validate the transaction;
/// call [`Self::validate`] to check its structural constraints.
///
/// The [`Transaction`] implementation represents the transaction as a call to [`Self::sender`],
/// with zero value and empty input. Individual call targets, values, and inputs belong to the
/// frames. Fee accessors saturate at [`u128::MAX`]. This type is not a variant of
/// [`super::TxEnvelope`] or [`super::TypedTransaction`].
///
/// ```
/// use alloy_consensus::TxEip8141;
/// use alloy_eips::{eip2718::Encodable2718, eip8141::Frame};
///
/// let tx = TxEip8141 { chain_id: 1, frames: vec![Frame::default()], ..Default::default() };
/// tx.validate()?;
/// let encoded = tx.encoded_2718();
/// assert_eq!(encoded[0], TxEip8141::tx_type());
/// # Ok::<(), alloy_consensus::TxEip8141ValidationError>(())
/// ```
///
/// [EIP-8141]: https://eips.ethereum.org/EIPS/eip-8141
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, RlpEncodable, RlpDecodable)]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "borsh", derive(borsh::BorshSerialize, borsh::BorshDeserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[doc(alias = "Eip8141Transaction", alias = "TransactionEip8141", alias = "Eip8141Tx")]
pub struct TxEip8141 {
    /// EIP-155 replay-protection chain ID.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub chain_id: ChainId,
    /// Sender nonce.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub nonce: u64,
    /// Intended transaction sender.
    pub sender: Address,
    /// Ordered frames to execute.
    pub frames: Vec<Frame>,
    /// Signature entries available to validation and execution.
    pub signatures: Vec<FrameSignature>,
    /// Fee parameters.
    pub fees: TransactionFees,
    /// Blob versioned hashes.
    pub blob_versioned_hashes: Vec<B256>,
}

impl TxEip8141 {
    /// EIP-2718 transaction type byte.
    pub const fn tx_type() -> u8 {
        FRAME_TX_TYPE
    }

    /// Calculates a heuristic for the in-memory size of the transaction.
    pub fn size(&self) -> usize {
        size_of::<Self>()
            + self.frames.capacity() * size_of::<Frame>()
            + self.frames.iter().map(|frame| frame.data.len()).sum::<usize>()
            + self.signatures.capacity() * size_of::<FrameSignature>()
            + self.signatures.iter().map(|signature| signature.signature.len()).sum::<usize>()
            + self.blob_versioned_hashes.capacity() * size_of::<B256>()
    }

    /// Validates the transaction's structural constraints.
    ///
    /// This checks frame and blob fields, signature metadata and canonical scalars, and the
    /// execution gas cap. It does not verify signatures cryptographically, execute frames, or
    /// check chain state, block capacity, base fees, or blob sidecars.
    pub fn validate(&self) -> Result<(), TxEip8141ValidationError> {
        use TxEip8141ValidationError as Error;

        if self.frames.is_empty() || self.frames.len() > MAX_FRAMES {
            return Err(Error::FrameCount(self.frames.len()));
        }
        if self.fees.max_priority_fee_per_gas > self.fees.max_fee_per_gas {
            return Err(Error::PriorityFeeAboveMaxFee);
        }
        if self.blob_versioned_hashes.is_empty() && !self.fees.max_fee_per_blob_gas.is_zero() {
            return Err(Error::BlobFeeWithoutBlobs);
        }
        if self.blob_versioned_hashes.len() as u64 > MAX_BLOBS_PER_TX_FUSAKA {
            return Err(Error::BlobCount(self.blob_versioned_hashes.len()));
        }
        for (index, hash) in self.blob_versioned_hashes.iter().enumerate() {
            if hash[0] != VERSIONED_HASH_VERSION_KZG {
                return Err(Error::BlobVersion { index, version: hash[0] });
            }
        }
        for (index, signature) in self.signatures.iter().enumerate() {
            if signature.explicit_message().is_some_and(|msg| msg.is_zero()) {
                return Err(Error::Signature { index, source: Eip8141Error::ZeroMessage });
            }
            signature
                .validate_structure_with_sender(self.sender)
                .map_err(|source| Error::Signature { index, source })?;
        }

        let mut frame_gas = 0u64;
        let mut expiry_verifier = false;
        let mut previous_atomic = false;
        for (index, frame) in self.frames.iter().enumerate() {
            if frame.has_reserved_flags() {
                return Err(Error::ReservedFlags { index, flags: frame.flags });
            }
            if !frame.value.is_zero() && frame.mode != FrameMode::Sender {
                return Err(Error::ValueOutsideSenderFrame { index });
            }
            if matches!(
                frame.allowed_scope(),
                ApprovalScope::Execution | ApprovalScope::ExecutionAndPayment
            ) && frame.resolved_target(self.sender) != self.sender
            {
                return Err(Error::ExecutionApprovalTarget { index });
            }
            let atomic = frame.is_atomic_batch();
            if atomic
                && (frame.mode == FrameMode::Verify
                    || self.frames.get(index + 1).is_none_or(|next| next.mode == FrameMode::Verify))
            {
                return Err(Error::AtomicBatch { index });
            }
            if (atomic || previous_atomic) && frame.allowed_scope() != ApprovalScope::None {
                return Err(Error::ApprovalInAtomicBatch { index });
            }
            previous_atomic = atomic;
            if frame.is_expiry_verifier() {
                if expiry_verifier || !frame.has_valid_expiry_verifier_fields() {
                    return Err(Error::ExpiryVerifier { index });
                }
                expiry_verifier = true;
            }
            frame_gas = frame_gas
                .checked_add(frame.limits.execution)
                .and_then(|gas| gas.checked_add(frame.limits.state))
                .ok_or(Error::FrameGasOverflow)?;
        }
        let execution = self.gas_limits().execution;
        if execution > MAX_TX_GAS_LIMIT_OSAKA {
            return Err(Error::ExecutionGasLimit(execution));
        }
        Ok(())
    }

    /// Returns the execution and state gas reservations of the transaction.
    ///
    /// Execution gas includes intrinsic costs and the calldata floor. State gas is the sum of
    /// the frames' state gas limits and is not subject to the execution gas cap. Arithmetic
    /// saturates at [`u64::MAX`]; use [`Self::validate`] to reject overflowing frame budgets.
    pub fn gas_limits(&self) -> FrameLimits {
        let mut limits = FrameLimits::default();
        let mut fixed = FRAME_TX_INTRINSIC_COST;
        for frame in &self.frames {
            fixed = fixed.saturating_add(FRAME_TX_PER_FRAME_COST);
            if !frame.value.is_zero() && frame.resolved_target(self.sender) != self.sender {
                fixed = fixed.saturating_add(TX_VALUE_COST);
            }
            limits.execution = limits.execution.saturating_add(frame.limits.execution);
            limits.state = limits.state.saturating_add(frame.limits.state);
        }
        for signature in &self.signatures {
            fixed = fixed.saturating_add(signature.verification_gas());
        }
        let data = self.frames.iter().map(|frame| frame.data.as_ref()).chain(
            self.signatures.iter().flat_map(|signature| {
                [
                    signature.signer.as_bytes(),
                    signature.msg.as_bytes(),
                    signature.signature.as_ref(),
                ]
            }),
        );
        let (tokens, bytes) = data.fold((0u64, 0u64), |(tokens, bytes), data| {
            (
                tokens.saturating_add(tokens_in_calldata(data)),
                bytes.saturating_add(data.len() as u64),
            )
        });
        let intrinsic =
            fixed.saturating_add(tokens.saturating_mul(FRAME_TX_DATA_TOKEN_STANDARD_COST));
        let floor =
            fixed.saturating_add(bytes.saturating_mul(4 * FRAME_TX_TOTAL_COST_FLOOR_PER_TOKEN));
        limits.execution = intrinsic.saturating_add(limits.execution).max(floor);
        limits
    }

    /// Returns the total gas reservation, including intrinsic gas and the calldata floor.
    ///
    /// The execution and state reservations are added with saturation at [`u64::MAX`].
    pub fn gas_limit(&self) -> u64 {
        let limits = self.gas_limits();
        limits.execution.saturating_add(limits.state)
    }

    /// Hash used by frame signatures.
    pub fn signature_hash(&self) -> B256 {
        let signatures = SigningSignatures(&self.signatures);
        let payload_length = self.chain_id.length()
            + self.nonce.length()
            + self.sender.length()
            + self.frames.length()
            + signatures.length()
            + self.fees.length()
            + self.blob_versioned_hashes.length();
        let header = Header { list: true, payload_length };
        let mut out = Vec::with_capacity(header.length_with_payload() + 1);
        out.put_u8(FRAME_TX_TYPE);
        header.encode(&mut out);
        self.chain_id.encode(&mut out);
        self.nonce.encode(&mut out);
        self.sender.encode(&mut out);
        self.frames.encode(&mut out);
        signatures.encode(&mut out);
        self.fees.encode(&mut out);
        self.blob_versioned_hashes.encode(&mut out);
        keccak256(out)
    }

    /// Canonical transaction hash.
    pub fn tx_hash(&self) -> B256 {
        let mut out = Vec::with_capacity(self.encode_2718_len());
        self.encode_2718(&mut out);
        keccak256(out)
    }
}

impl Typed2718 for TxEip8141 {
    fn ty(&self) -> u8 {
        FRAME_TX_TYPE
    }
}

impl IsTyped2718 for TxEip8141 {
    fn is_type(ty: u8) -> bool {
        ty == FRAME_TX_TYPE
    }
}

impl Sealable for TxEip8141 {
    fn hash_slow(&self) -> B256 {
        self.tx_hash()
    }
}

impl Encodable2718 for TxEip8141 {
    fn encode_2718_len(&self) -> usize {
        self.length() + 1
    }

    fn encode_2718(&self, out: &mut dyn BufMut) {
        out.put_u8(FRAME_TX_TYPE);
        self.encode(out);
    }
}

impl Decodable2718 for TxEip8141 {
    fn typed_decode(ty: u8, buf: &mut &[u8]) -> Eip2718Result<Self> {
        if ty == FRAME_TX_TYPE {
            Self::decode(buf).map_err(Into::into)
        } else {
            Err(Eip2718Error::UnexpectedType(ty))
        }
    }

    fn fallback_decode(_: &mut &[u8]) -> Eip2718Result<Self> {
        Err(Eip2718Error::UnexpectedType(FRAME_TX_TYPE))
    }
}

impl Transaction for TxEip8141 {
    fn chain_id(&self) -> Option<ChainId> {
        Some(self.chain_id)
    }

    fn nonce(&self) -> u64 {
        self.nonce
    }

    fn gas_limit(&self) -> u64 {
        self.gas_limit()
    }

    fn gas_price(&self) -> Option<u128> {
        None
    }

    fn max_fee_per_gas(&self) -> u128 {
        self.fees.max_fee_per_gas.saturating_to()
    }

    fn max_priority_fee_per_gas(&self) -> Option<u128> {
        Some(self.fees.max_priority_fee_per_gas.saturating_to())
    }

    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        (!self.blob_versioned_hashes.is_empty())
            .then(|| self.fees.max_fee_per_blob_gas.saturating_to())
    }

    fn priority_fee_or_price(&self) -> u128 {
        self.fees.max_priority_fee_per_gas.saturating_to()
    }

    fn effective_gas_price(&self, base_fee: Option<u64>) -> u128 {
        alloy_eips::eip1559::calc_effective_gas_price(
            self.max_fee_per_gas(),
            self.priority_fee_or_price(),
            base_fee,
        )
    }

    fn is_dynamic_fee(&self) -> bool {
        true
    }

    fn kind(&self) -> TxKind {
        TxKind::Call(self.sender)
    }

    fn is_create(&self) -> bool {
        false
    }

    fn value(&self) -> U256 {
        U256::ZERO
    }

    fn input(&self) -> &Bytes {
        &EMPTY_INPUT
    }

    fn access_list(&self) -> Option<&alloy_eips::eip2930::AccessList> {
        None
    }

    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        Some(&self.blob_versioned_hashes)
    }

    fn authorization_list(&self) -> Option<&[alloy_eips::eip7702::SignedAuthorization]> {
        None
    }
}

/// A structural validation error in an EIP-8141 transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TxEip8141ValidationError {
    /// The number of frames is outside the permitted range.
    #[error("expected between 1 and {MAX_FRAMES} frames, got {0}")]
    FrameCount(usize),
    /// The priority fee exceeds the maximum fee per gas.
    #[error("max priority fee exceeds max fee")]
    PriorityFeeAboveMaxFee,
    /// A transaction without blobs specifies a nonzero blob fee.
    #[error("blob fee is nonzero without blob hashes")]
    BlobFeeWithoutBlobs,
    /// The transaction exceeds the per-transaction blob limit.
    #[error("expected at most {MAX_BLOBS_PER_TX_FUSAKA} blobs, got {0}")]
    BlobCount(usize),
    /// A blob hash uses an unsupported version.
    #[error("blob {index} has unsupported version {version}")]
    BlobVersion {
        /// Index of the blob hash.
        index: usize,
        /// Version byte of the blob hash.
        version: u8,
    },
    /// A signature's metadata or canonical scalar checks failed.
    #[error("invalid signature {index}: {source}")]
    Signature {
        /// Index of the signature.
        index: usize,
        /// The underlying signature error.
        source: Eip8141Error,
    },
    /// A frame sets reserved flags.
    #[error("frame {index} has reserved flags: {flags}")]
    ReservedFlags {
        /// Index of the frame.
        index: usize,
        /// Frame flags.
        flags: u8,
    },
    /// Only sender frames may carry value.
    #[error("frame {index} carries value outside sender mode")]
    ValueOutsideSenderFrame {
        /// Index of the frame.
        index: usize,
    },
    /// Execution approval is only allowed at the transaction sender.
    #[error("frame {index} allows execution approval at a target other than the sender")]
    ExecutionApprovalTarget {
        /// Index of the frame.
        index: usize,
    },
    /// An atomic batch flag lacks a following non-verify frame or is set on a verify frame.
    #[error("invalid atomic batch flag on frame {index}")]
    AtomicBatch {
        /// Index of the frame.
        index: usize,
    },
    /// Frames in an atomic batch cannot approve execution or payment.
    #[error("frame {index} allows approval inside an atomic batch")]
    ApprovalInAtomicBatch {
        /// Index of the frame, including the terminating frame of a batch.
        index: usize,
    },
    /// An expiry verifier has invalid fields or follows another expiry verifier.
    #[error("invalid or duplicate expiry verifier at frame {index}")]
    ExpiryVerifier {
        /// Index of the frame.
        index: usize,
    },
    /// The sum of frame execution and state budgets overflows a `u64`.
    #[error("total frame gas overflows u64")]
    FrameGasOverflow,
    /// The execution reservation exceeds the transaction execution gas cap.
    #[error("execution gas {0} exceeds {MAX_TX_GAS_LIMIT_OSAKA}")]
    ExecutionGasLimit(u64),
}

static EMPTY_INPUT: Bytes = Bytes::new();

/// RLP signing-preimage view that blanks a transaction-hash signature to avoid self-reference.
struct SigningSignature<'a>(&'a FrameSignature);

impl SigningSignature<'_> {
    fn payload_length(&self) -> usize {
        self.0.scheme.length()
            + self.0.signer.length()
            + self.0.msg.length()
            + if self.0.signs_transaction_hash() {
                EMPTY_INPUT.length()
            } else {
                self.0.signature.length()
            }
    }
}

impl Encodable for SigningSignature<'_> {
    fn encode(&self, out: &mut dyn BufMut) {
        Header { list: true, payload_length: self.payload_length() }.encode(out);
        self.0.scheme.encode(out);
        self.0.signer.encode(out);
        self.0.msg.encode(out);
        if self.0.signs_transaction_hash() {
            EMPTY_INPUT.encode(out);
        } else {
            self.0.signature.encode(out);
        }
    }

    fn length(&self) -> usize {
        Header { list: true, payload_length: self.payload_length() }.length_with_payload()
    }
}

/// RLP list view of transformed signing entries, including the corresponding list-header length.
struct SigningSignatures<'a>(&'a [FrameSignature]);

impl Encodable for SigningSignatures<'_> {
    fn encode(&self, out: &mut dyn BufMut) {
        let payload_length = self.0.iter().map(|s| SigningSignature(s).length()).sum();
        Header { list: true, payload_length }.encode(out);
        for signature in self.0 {
            SigningSignature(signature).encode(out);
        }
    }

    fn length(&self) -> usize {
        let payload_length = self.0.iter().map(|s| SigningSignature(s).length()).sum();
        Header { list: true, payload_length }.length_with_payload()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_eips::{
        eip2718::{Decodable2718, Encodable2718},
        eip8141::{
            FrameAddress, SignatureMessage, SignatureScheme, EXPIRY_VERIFIER, SECP256K1N,
            SECP256R1N,
        },
    };

    use alloy_primitives::hex;

    fn valid_tx() -> TxEip8141 {
        TxEip8141 { chain_id: 1, frames: vec![Frame::default()], ..Default::default() }
    }

    #[test]
    fn roundtrips_canonical_encoding() {
        let tx = valid_tx();
        let encoded = tx.encoded_2718();
        assert_eq!(TxEip8141::decode_2718_exact(&encoded).unwrap(), tx);
        assert_ne!(tx.tx_hash(), B256::ZERO);
    }

    #[test]
    fn validates_arbitrary_signatures_without_signers() {
        let mut tx = valid_tx();
        tx.signatures.push(FrameSignature::default());
        assert_eq!(tx.validate(), Ok(()));
    }

    #[test]
    fn rejects_arbitrary_signature_with_signer() {
        let mut tx = valid_tx();
        tx.signatures.push(FrameSignature { signer: Address::ZERO.into(), ..Default::default() });
        assert_eq!(
            tx.validate(),
            Err(TxEip8141ValidationError::Signature {
                index: 0,
                source: Eip8141Error::UnexpectedSigner
            })
        );
    }

    #[test]
    fn rejects_noncanonical_secp256k1_signature() {
        let mut tx = valid_tx();
        tx.signatures.push(FrameSignature {
            scheme: SignatureScheme::Secp256k1,
            signature: Bytes::from(vec![0; 65]),
            ..Default::default()
        });
        assert_eq!(
            tx.validate(),
            Err(TxEip8141ValidationError::Signature {
                index: 0,
                source: Eip8141Error::InvalidSignatureScalar
            })
        );
    }

    #[test]
    fn signature_bytes_contribute_to_calldata_floor() {
        let mut tx = valid_tx();
        tx.signatures.push(FrameSignature { signature: vec![0; 100].into(), ..Default::default() });
        assert_eq!(tx.validate(), Ok(()));
        assert_eq!(tx.gas_limit(), 12_000 + 475 + 100 + 100 * 64);
    }

    #[test]
    fn signature_floor_can_exceed_execution_cap() {
        let mut tx = valid_tx();
        tx.signatures
            .push(FrameSignature { signature: vec![0; 262_144].into(), ..Default::default() });
        assert!(tx.validate().is_err(), "signature calldata floor alone exceeds 2**24");
    }

    #[test]
    fn state_gas_is_not_subject_to_execution_cap() {
        let mut tx = valid_tx();
        tx.frames[0].limits.execution = 100;
        tx.frames[0].limits.state = MAX_TX_GAS_LIMIT_OSAKA;
        assert_eq!(tx.validate(), Ok(()));
    }

    #[test]
    fn atomic_batch_cannot_approve() {
        for flags in [[5, 0], [4, 1], [6, 0], [4, 2]] {
            let mut tx = valid_tx();
            tx.frames = flags.map(|flags| Frame { flags, ..Default::default() }).to_vec();
            assert!(tx.validate().is_err(), "accepted batch flags {flags:?}");
        }
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_uses_alloy_field_names_and_quantities() {
        let json = serde_json::to_value(valid_tx()).unwrap();
        assert_eq!(json["chainId"], "0x1", "serialized {json}");
        assert_eq!(json["nonce"], "0x0");
        assert_eq!(json["blobVersionedHashes"], serde_json::json!([]));
    }

    #[test]
    #[cfg(feature = "serde")]
    fn serde_accepts_alloy_field_names_and_quantities() {
        let mut json = serde_json::to_value(valid_tx()).unwrap();
        let obj = json.as_object_mut().unwrap();
        obj.remove("chain_id");
        obj.remove("blob_versioned_hashes");
        obj.insert("chainId".into(), "0x1".into());
        obj.insert("nonce".into(), "0x0".into());
        obj.insert("blobVersionedHashes".into(), serde_json::json!([]));
        assert_eq!(serde_json::from_value::<TxEip8141>(json).unwrap(), valid_tx());
    }

    #[test]
    fn effective_gas_price_does_not_overflow() {
        let mut tx = valid_tx();
        tx.fees.max_fee_per_gas = U256::from(u128::MAX);
        tx.fees.max_priority_fee_per_gas = U256::from(u128::MAX);
        assert_eq!(tx.validate(), Ok(()));
        assert_eq!(tx.effective_gas_price(Some(1)), u128::MAX);
        tx.fees.max_fee_per_gas = U256::MAX;
        tx.fees.max_priority_fee_per_gas = U256::MAX;
        assert_eq!(tx.effective_gas_price(Some(u64::MAX)), u128::MAX);
        assert_eq!(tx.effective_gas_price(None), u128::MAX);
        tx.fees.max_fee_per_gas = U256::from(100);
        tx.fees.max_priority_fee_per_gas = U256::from(10);
        assert_eq!(tx.effective_gas_price(None), 100);
        assert_eq!(tx.effective_gas_price(Some(80)), 90);
        assert_eq!(tx.effective_gas_price(Some(95)), 100);
        tx.fees.max_priority_fee_per_gas = U256::from(101);
        assert_eq!(tx.validate(), Err(TxEip8141ValidationError::PriorityFeeAboveMaxFee));
    }

    #[test]
    fn kind_agrees_with_is_create() {
        let tx = valid_tx();
        assert_eq!(tx.kind().is_create(), tx.is_create());
        assert_eq!(tx.to(), Some(tx.sender));
        assert_eq!(tx.value(), U256::ZERO);
        assert!(tx.input().is_empty());
    }

    #[test]
    fn validation_rejects_explicit_zero_message() {
        let mut tx = valid_tx();
        tx.signatures.push(FrameSignature {
            msg: SignatureMessage::Explicit(B256::ZERO),
            ..Default::default()
        });
        assert!(TxEip8141::decode_2718_exact(&tx.encoded_2718()).is_err());
        assert!(tx.validate().is_err());
    }

    #[test]
    fn canonical_rlp_fixture() {
        let mut tx = valid_tx();
        tx.sender = Address::repeat_byte(0x11);
        // Independently assembled: type 6; seven-field list; one six-field frame.
        let expected = hex!(
            "06e70180941111111111111111111111111111111111111111c9c8808080c280808080c0c3808080c0"
        );
        assert_eq!(tx.encoded_2718(), expected);
        assert_eq!(tx.encode_2718_len(), expected.len());
        assert_eq!(tx.tx_hash(), keccak256(expected));
        assert_eq!(TxEip8141::decode_2718_exact(&expected).unwrap(), tx);
    }

    #[test]
    fn rlp_boundaries_and_signing_elision() {
        for len in [0, 1, 54, 55, 56, 255, 256, 65_535] {
            let mut tx = valid_tx();
            tx.frames[0].data = vec![0x81; len].into();
            for scheme in
                [SignatureScheme::Arbitrary, SignatureScheme::Secp256k1, SignatureScheme::P256]
            {
                tx.signatures.push(FrameSignature {
                    scheme,
                    signature: vec![0x82; len].into(),
                    ..Default::default()
                });
                tx.signatures.push(FrameSignature {
                    scheme,
                    msg: SignatureMessage::Explicit(B256::repeat_byte(0x33)),
                    signature: vec![0x84; len].into(),
                    ..Default::default()
                });
            }
            let encoded = tx.encoded_2718();
            assert_eq!(tx.encode_2718_len(), encoded.len());
            assert_eq!(TxEip8141::decode_2718_exact(&encoded).unwrap(), tx);
            let mut transformed = tx.clone();
            for signature in &mut transformed.signatures {
                if signature.msg == SignatureMessage::TransactionHash {
                    signature.signature = Bytes::new();
                }
            }
            assert_eq!(tx.signature_hash(), keccak256(transformed.encoded_2718()));
            let mut network = Vec::new();
            tx.network_encode(&mut network);
            assert_eq!(network.len(), tx.network_len());
            assert_eq!(TxEip8141::network_decode(&mut network.as_slice()).unwrap(), tx);
        }
    }

    #[test]
    fn malformed_rlp_is_rejected() {
        let encoded = valid_tx().encoded_2718();
        for len in 0..encoded.len() {
            assert!(TxEip8141::decode_2718_exact(&encoded[..len]).is_err());
        }
        let mut extra = encoded.clone();
        extra.push(0x80);
        assert!(TxEip8141::decode_2718_exact(&extra).is_err());
        let mut body = &encoded[1..];
        let header = Header::decode(&mut body).unwrap();
        for adjustment in [-1, 1] {
            let mut malformed = Vec::new();
            Header {
                list: true,
                payload_length: (header.payload_length as isize + adjustment) as usize,
            }
            .encode(&mut malformed);
            malformed.extend_from_slice(body);
            malformed.push(0x80);
            assert!(TxEip8141::decode(&mut malformed.as_slice()).is_err());
        }
    }

    #[test]
    fn validates_frame_counts() {
        let mut tx = valid_tx();
        for count in [0, MAX_FRAMES, MAX_FRAMES + 1] {
            tx.frames = vec![Frame::default(); count];
            let expected = if count == MAX_FRAMES {
                Ok(())
            } else {
                Err(TxEip8141ValidationError::FrameCount(count))
            };
            assert_eq!(tx.validate(), expected);
        }
    }

    #[test]
    fn validates_frame_flags_targets_and_value() {
        use TxEip8141ValidationError as Error;
        let mut tx = valid_tx();
        tx.frames[0].flags = 8;
        assert_eq!(tx.validate(), Err(Error::ReservedFlags { index: 0, flags: 8 }));
        tx.frames[0].flags = 0;
        tx.frames[0].value = U256::from(1);
        for mode in [FrameMode::Default, FrameMode::Verify] {
            tx.frames[0].mode = mode;
            assert_eq!(tx.validate(), Err(Error::ValueOutsideSenderFrame { index: 0 }));
        }
        tx.frames[0].mode = FrameMode::Sender;
        assert_eq!(tx.validate(), Ok(()));
        tx.frames[0].value = U256::ZERO;
        for scope in [ApprovalScope::Execution, ApprovalScope::ExecutionAndPayment] {
            tx.frames[0].flags = scope as u8;
            for target in [FrameAddress::default(), tx.sender.into()] {
                tx.frames[0].target = target;
                assert_eq!(tx.validate(), Ok(()));
            }
            tx.frames[0].target = Address::repeat_byte(1).into();
            assert_eq!(tx.validate(), Err(Error::ExecutionApprovalTarget { index: 0 }));
        }
        // Payment approval may target a paymaster, and VERIFY does not require an approval scope.
        tx.frames[0].flags = ApprovalScope::Payment as u8;
        assert_eq!(tx.validate(), Ok(()));
        tx.frames[0].mode = FrameMode::Verify;
        tx.frames[0].flags = 0;
        assert_eq!(tx.validate(), Ok(()));
    }

    #[test]
    fn validates_atomic_batch_boundaries() {
        use TxEip8141ValidationError as Error;
        let mut tx = valid_tx();
        tx.frames[0].flags = 4;
        assert_eq!(tx.validate(), Err(Error::AtomicBatch { index: 0 }));
        tx.frames.push(Frame::default());
        assert_eq!(tx.validate(), Ok(()));
        for index in 0..2 {
            tx.frames[index].mode = FrameMode::Verify;
            assert_eq!(tx.validate(), Err(Error::AtomicBatch { index: 0 }));
            tx.frames[index].mode = FrameMode::Sender;
        }
        // A terminating frame belongs to its batch, but the next frame does not.
        tx.frames[1].flags = 1;
        assert_eq!(tx.validate(), Err(Error::ApprovalInAtomicBatch { index: 1 }));
        tx.frames[1].flags = 0;
        tx.frames.push(Frame { flags: 1, ..Default::default() });
        assert_eq!(tx.validate(), Ok(()));
        // Consecutive batches are independently terminated.
        tx.frames[2].flags = 4;
        tx.frames.push(Frame::default());
        assert_eq!(tx.validate(), Ok(()));
    }

    #[test]
    fn validates_expiry_verifiers() {
        use TxEip8141ValidationError as Error;
        let mut tx = valid_tx();
        let expiry = Frame {
            mode: FrameMode::Verify,
            target: EXPIRY_VERIFIER.into(),
            data: Bytes::from(1_000u64.to_be_bytes().to_vec()),
            ..Default::default()
        };
        tx.frames[0] = expiry.clone();
        assert_eq!(tx.validate(), Ok(()));
        tx.frames[0].flags = 1;
        assert_eq!(tx.validate(), Err(Error::ExpiryVerifier { index: 0 }));
        tx.frames[0] = expiry.clone();
        tx.frames[0].limits.state = 1;
        assert_eq!(tx.validate(), Err(Error::ExpiryVerifier { index: 0 }));
        for len in [0, 7, 9] {
            tx.frames[0] = expiry.clone();
            tx.frames[0].data = vec![0; len].into();
            assert_eq!(tx.validate(), Err(Error::ExpiryVerifier { index: 0 }));
        }
        tx.frames = vec![expiry.clone(), expiry];
        assert_eq!(tx.validate(), Err(Error::ExpiryVerifier { index: 1 }));
    }

    #[test]
    fn validates_blob_fields() {
        use TxEip8141ValidationError as Error;
        let mut tx = valid_tx();
        tx.fees.max_fee_per_blob_gas = U256::from(1);
        assert_eq!(tx.validate(), Err(Error::BlobFeeWithoutBlobs));
        tx.blob_versioned_hashes = vec![B256::repeat_byte(VERSIONED_HASH_VERSION_KZG); 6];
        assert_eq!(tx.validate(), Ok(()));
        // The blob base fee is checked by the execution client at block inclusion.
        tx.fees.max_fee_per_blob_gas = U256::ZERO;
        assert_eq!(tx.validate(), Ok(()));
        tx.blob_versioned_hashes.push(B256::repeat_byte(VERSIONED_HASH_VERSION_KZG));
        assert_eq!(tx.validate(), Err(Error::BlobCount(7)));
        tx.blob_versioned_hashes.pop();
        tx.blob_versioned_hashes[1] = B256::ZERO;
        assert_eq!(tx.validate(), Err(Error::BlobVersion { index: 1, version: 0 }));
    }

    #[test]
    fn validates_gas_boundaries() {
        use TxEip8141ValidationError as Error;
        let mut tx = valid_tx();
        tx.frames[0].limits.execution = MAX_TX_GAS_LIMIT_OSAKA - 12_475;
        assert_eq!(tx.gas_limits().execution, MAX_TX_GAS_LIMIT_OSAKA);
        assert_eq!(tx.validate(), Ok(()));
        tx.frames[0].limits.execution += 1;
        assert_eq!(tx.validate(), Err(Error::ExecutionGasLimit(MAX_TX_GAS_LIMIT_OSAKA + 1)));
        tx.frames[0].limits = FrameLimits { execution: 1, state: u64::MAX };
        assert_eq!(tx.validate(), Err(Error::FrameGasOverflow));
        tx.frames[0].limits = FrameLimits { execution: 0, state: u64::MAX };
        tx.frames
            .push(Frame { limits: FrameLimits { execution: 0, state: 1 }, ..Default::default() });
        assert_eq!(tx.validate(), Err(Error::FrameGasOverflow));
        assert_eq!(tx.gas_limit(), u64::MAX);
    }

    #[test]
    fn gas_counts_metadata_and_keeps_state_outside_floor() {
        let mut tx = valid_tx();
        tx.frames[0].data = vec![0, 1].into();
        tx.frames[0].limits = FrameLimits { execution: 0, state: 1_000 };
        tx.signatures.push(FrameSignature {
            scheme: SignatureScheme::Secp256k1,
            signer: Address::repeat_byte(1).into(),
            msg: SignatureMessage::Explicit(B256::repeat_byte(2)),
            signature: vec![3; 65].into(),
        });
        let fixed = 12_000 + 475 + 2_800;
        let floor = fixed + (2 + 20 + 32 + 65) * 64;
        assert_eq!(tx.gas_limits(), FrameLimits { execution: floor, state: 1_000 });
        assert_eq!(tx.gas_limit(), floor + 1_000);
        // Once execution dominates, only the standard intrinsic byte costs apply.
        tx.frames[0].limits.execution = 100_000;
        let intrinsic = fixed + 4 + (1 + 20 + 32 + 65) * 16;
        assert_eq!(tx.gas_limit(), intrinsic + 100_000 + 1_000);
    }

    #[test]
    fn gas_charges_value_transfers_only_to_other_accounts() {
        let mut tx = valid_tx();
        tx.frames[0].mode = FrameMode::Sender;
        tx.frames[0].value = U256::from(1);
        for target in [FrameAddress::default(), tx.sender.into()] {
            tx.frames[0].target = target;
            assert_eq!(tx.gas_limit(), 12_475);
        }
        tx.frames[0].target = Address::repeat_byte(1).into();
        assert_eq!(tx.gas_limit(), 18_475);
    }

    #[test]
    fn validates_signature_scalars_and_resolved_signers() {
        use TxEip8141ValidationError as Error;
        let mut tx = valid_tx();
        for (scheme, order, len, offset) in [
            (SignatureScheme::Secp256k1, SECP256K1N, 65, 1),
            (SignatureScheme::P256, SECP256R1N, 128, 0),
        ] {
            let mut bytes = vec![0u8; len];
            bytes[offset + 31] = 1;
            bytes[offset + 63] = 1;
            let mut signature =
                FrameSignature { scheme, signature: bytes.clone().into(), ..Default::default() };
            tx.sender = signature.p256_signer_address().unwrap_or(Address::ZERO);
            tx.signatures = vec![signature.clone()];
            assert_eq!(tx.validate(), Ok(()));
            for (r, s) in [
                (U256::ZERO, U256::from(1)),
                (order, U256::from(1)),
                (U256::from(1), (order >> 1) + U256::from(1)),
            ] {
                bytes[offset..offset + 32].copy_from_slice(&r.to_be_bytes::<32>());
                bytes[offset + 32..offset + 64].copy_from_slice(&s.to_be_bytes::<32>());
                tx.signatures[0].signature = bytes.clone().into();
                assert_eq!(
                    tx.validate(),
                    Err(Error::Signature {
                        index: 0,
                        source: Eip8141Error::InvalidSignatureScalar
                    })
                );
            }
            if scheme == SignatureScheme::P256 {
                signature.signer = Address::repeat_byte(1).into();
                tx.signatures[0] = signature;
                assert!(matches!(
                    tx.validate(),
                    Err(Error::Signature { source: Eip8141Error::P256SignerMismatch { .. }, .. })
                ));
            } else {
                bytes[offset..offset + 32].copy_from_slice(&U256::from(1).to_be_bytes::<32>());
                bytes[offset + 32..offset + 64].copy_from_slice(&U256::from(1).to_be_bytes::<32>());
                bytes[0] = 27;
                tx.signatures[0].signature = bytes.clone().into();
                assert_eq!(
                    tx.validate(),
                    Err(Error::Signature { index: 0, source: Eip8141Error::InvalidParity(27) })
                );
            }
            tx.signatures[0].signature = Bytes::new();
            assert_eq!(
                tx.validate(),
                Err(Error::Signature {
                    index: 0,
                    source: Eip8141Error::InvalidSignatureLength { expected: len, actual: 0 }
                })
            );
        }
    }
}
