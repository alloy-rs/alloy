//! Standalone EIP-8141 frame transaction encoding and validation.

use alloc::vec::Vec;
use alloy_eips::{
    eip2718::{Decodable2718, Eip2718Error, Eip2718Result, Encodable2718, IsTyped2718},
    eip7623::tokens_in_calldata,
    eip7825::MAX_TX_GAS_LIMIT_OSAKA,
    eip8141::{
        constants::{
            FRAME_TX_DATA_TOKEN_STANDARD_COST, FRAME_TX_INTRINSIC_COST, FRAME_TX_PER_FRAME_COST,
            FRAME_TX_TOTAL_COST_FLOOR_PER_TOKEN, FRAME_TX_TYPE, MAX_FRAMES, TX_VALUE_COST,
        },
        ApprovalScope, Eip8141Error, Frame, FrameMode, FrameSignature, TransactionFees,
    },
    Typed2718,
};
use alloy_primitives::{keccak256, Address, Bytes, ChainId, Sealable, TxKind, B256, U256};
use alloy_rlp::{BufMut, Decodable, Encodable, Header};

use super::Transaction;

static EMPTY_INPUT: Bytes = Bytes::new();

/// RLP signing-preimage view that blanks a transaction-hash signature to avoid self-reference.
struct SigningSignature<'a>(&'a FrameSignature);

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

/// An EIP-8141 frame transaction.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "borsh", derive(borsh::BorshSerialize, borsh::BorshDeserialize))]
pub struct TxEip8141 {
    /// EIP-155 replay-protection chain ID.
    pub chain_id: ChainId,
    /// Sender nonce.
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

    /// Validates structural constraints without executing a frame.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.frames.is_empty() || self.frames.len() > MAX_FRAMES {
            return Err("EIP-8141 transaction must contain between 1 and 64 frames");
        }
        if self.fees.max_priority_fee_per_gas > self.fees.max_fee_per_gas {
            return Err("max priority fee exceeds max fee");
        }
        if self.blob_versioned_hashes.is_empty() {
            if !self.fees.max_fee_per_blob_gas.is_zero() {
                return Err("blob fee is non-zero without blob hashes");
            }
        } else if self.fees.max_fee_per_blob_gas.is_zero()
            || self.blob_versioned_hashes.iter().any(|hash| hash[0] != 1)
        {
            return Err("invalid EIP-8141 blob fields");
        }

        let mut execution = 0u64;
        let mut state = 0u64;
        let mut expiry_verifiers = 0u8;
        self.signature_checks()?;
        for (index, frame) in self.frames.iter().enumerate() {
            if frame.has_reserved_flags() {
                return Err("reserved EIP-8141 frame flag is set");
            }
            if !frame.value.is_zero() && frame.mode != FrameMode::Sender {
                return Err("frame value is only valid in sender mode");
            }
            if matches!(
                frame.allowed_scope(),
                ApprovalScope::Execution | ApprovalScope::ExecutionAndPayment
            ) && frame.resolved_target(self.sender) != self.sender
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
            if frame.is_expiry_verifier() {
                expiry_verifiers = expiry_verifiers.saturating_add(1);
                if expiry_verifiers > 1 || !frame.has_valid_expiry_verifier_fields() {
                    return Err("invalid expiry verifier frame");
                }
            }
            execution = execution
                .checked_add(frame.limits.execution)
                .ok_or("frame execution gas overflow")?;
            state = state.checked_add(frame.limits.state).ok_or("frame state gas overflow")?;
        }
        if execution.checked_add(state).is_none() || self.gas_limit() > MAX_TX_GAS_LIMIT_OSAKA {
            return Err("EIP-8141 transaction exceeds the gas limit");
        }
        Ok(())
    }

    /// Sum of execution and state gas limits plus protocol intrinsic gas.
    pub fn gas_limit(&self) -> u64 {
        let data = self
            .frames
            .iter()
            .fold(0u64, |n, frame| n.saturating_add(tokens_in_calldata(&frame.data)));
        let data = self.signatures.iter().fold(data, |n, signature| {
            n.saturating_add(tokens_in_calldata(signature.signer.as_bytes()))
                .saturating_add(tokens_in_calldata(signature.msg.as_bytes()))
                .saturating_add(tokens_in_calldata(&signature.signature))
        });
        let signature_gas = self
            .signatures
            .iter()
            .fold(0u64, |n, signature| n.saturating_add(signature.verification_gas()));
        let value_gas = self.frames.iter().fold(0u64, |n, frame| {
            n.saturating_add(
                if !frame.value.is_zero() && frame.resolved_target(self.sender) != self.sender {
                    TX_VALUE_COST
                } else {
                    0
                },
            )
        });
        let execution = FRAME_TX_INTRINSIC_COST
            .saturating_add((self.frames.len() as u64).saturating_mul(FRAME_TX_PER_FRAME_COST))
            .saturating_add(data.saturating_mul(FRAME_TX_DATA_TOKEN_STANDARD_COST))
            .saturating_add(signature_gas)
            .saturating_add(value_gas)
            .saturating_add(
                self.frames.iter().fold(0u64, |n, frame| n.saturating_add(frame.limits.execution)),
            );
        let state = self.frames.iter().fold(0u64, |n, frame| n.saturating_add(frame.limits.state));
        let floor = FRAME_TX_INTRINSIC_COST
            .saturating_add((self.frames.len() as u64).saturating_mul(FRAME_TX_PER_FRAME_COST))
            .saturating_add(signature_gas)
            .saturating_add(value_gas)
            .saturating_add(
                self.frames
                    .iter()
                    .fold(0u64, |n, frame| n.saturating_add(frame.data.len() as u64))
                    .saturating_mul(4 * FRAME_TX_TOTAL_COST_FLOOR_PER_TOKEN),
            );
        execution.saturating_add(state).max(floor.saturating_add(state))
    }

    /// Hash used by frame signatures.
    pub fn signature_hash(&self) -> B256 {
        let mut out = Vec::with_capacity(self.rlp_encoded_length() + 1);
        out.put_u8(FRAME_TX_TYPE);
        let signatures = SigningSignatures(&self.signatures);
        let payload_length = self.chain_id.length()
            + self.nonce.length()
            + self.sender.length()
            + self.frames.length()
            + signatures.length()
            + self.fees.length()
            + self.blob_versioned_hashes.length();
        Header { list: true, payload_length }.encode(&mut out);
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
        let mut out = Vec::with_capacity(self.rlp_encoded_length() + 1);
        self.encode_2718(&mut out);
        keccak256(out)
    }

    fn payload_length(&self) -> usize {
        self.chain_id.length()
            + self.nonce.length()
            + self.sender.length()
            + self.frames.length()
            + self.signatures.length()
            + self.fees.length()
            + self.blob_versioned_hashes.length()
    }

    fn rlp_encoded_length(&self) -> usize {
        Header { list: true, payload_length: self.payload_length() }.length_with_payload()
    }

    fn signature_checks(&self) -> Result<(), &'static str> {
        for signature in &self.signatures {
            signature.validate_structure_with_sender(self.sender).map_err(|err| match err {
                Eip8141Error::UnexpectedSigner => {
                    "arbitrary signatures must not contain signer metadata"
                }
                Eip8141Error::InvalidSignatureLength { .. } => "invalid frame signature length",
                Eip8141Error::InvalidParity(_) | Eip8141Error::InvalidSignatureScalar => {
                    "frame signature is not canonical"
                }
                Eip8141Error::P256SignerMismatch { .. } => {
                    "P-256 public key does not match resolved signer"
                }
                _ => "invalid frame signature entry",
            })?;
        }
        Ok(())
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

impl Encodable for TxEip8141 {
    fn encode(&self, out: &mut dyn BufMut) {
        Header { list: true, payload_length: self.payload_length() }.encode(out);
        self.chain_id.encode(out);
        self.nonce.encode(out);
        self.sender.encode(out);
        self.frames.encode(out);
        self.signatures.encode(out);
        self.fees.encode(out);
        self.blob_versioned_hashes.encode(out);
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
        let tx = Self {
            chain_id: Decodable::decode(buf)?,
            nonce: Decodable::decode(buf)?,
            sender: Decodable::decode(buf)?,
            frames: Decodable::decode(buf)?,
            signatures: Decodable::decode(buf)?,
            fees: Decodable::decode(buf)?,
            blob_versioned_hashes: Decodable::decode(buf)?,
        };
        if buf.len() + header.payload_length != remaining {
            return Err(alloy_rlp::Error::UnexpectedLength);
        }
        Ok(tx)
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
        self.fees.max_fee_per_gas.saturating_to::<u128>().min(base_fee.map_or(u128::MAX, |base| {
            base as u128 + self.fees.max_priority_fee_per_gas.saturating_to::<u128>()
        }))
    }
    fn is_dynamic_fee(&self) -> bool {
        true
    }
    fn kind(&self) -> TxKind {
        TxKind::Create
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

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_eips::{
        eip2718::{Decodable2718, Encodable2718},
        eip8141::SignatureScheme,
    };

    fn valid_tx() -> TxEip8141 {
        TxEip8141 { frames: vec![Frame::default()], ..Default::default() }
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
        assert_eq!(tx.validate(), Err("arbitrary signatures must not contain signer metadata"));
    }

    #[test]
    fn rejects_noncanonical_secp256k1_signature() {
        let mut tx = valid_tx();
        tx.signatures.push(FrameSignature {
            scheme: SignatureScheme::Secp256k1,
            signature: Bytes::from(vec![0; 65]),
            ..Default::default()
        });
        assert_eq!(tx.validate(), Err("frame signature is not canonical"));
    }
}
