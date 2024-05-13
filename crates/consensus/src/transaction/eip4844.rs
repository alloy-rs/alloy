mod builder;
pub use builder::{SidecarBuilder, SidecarCoder, SimpleCoder};

pub mod utils;

use crate::{SignableTransaction, Signed, Transaction, TxType};

use alloy_eips::{
    eip2930::AccessList,
    eip4844::{BYTES_PER_BLOB, BYTES_PER_COMMITMENT, BYTES_PER_PROOF, DATA_GAS_PER_BLOB},
};
use alloy_primitives::{
    keccak256, Address, Bytes, ChainId, FixedBytes, Signature, TxKind, B256, U256,
};
use alloy_rlp::{length_of_length, BufMut, Decodable, Encodable, Header};
use core::mem;
use sha2::{Digest, Sha256};

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

#[cfg(not(feature = "kzg"))]
pub use alloy_eips::eip4844::{Blob, Bytes48};

#[cfg(feature = "kzg")]
use c_kzg::{KzgCommitment, KzgProof, KzgSettings};
#[cfg(feature = "kzg")]
use std::ops::Deref;

#[cfg(feature = "kzg")]
pub use c_kzg::{Blob, Bytes48};

/// An error that can occur when validating a [TxEip4844Variant].
#[derive(Debug, thiserror::Error)]
#[cfg(feature = "kzg")]
pub enum BlobTransactionValidationError {
    /// Proof validation failed.
    #[error("invalid KZG proof")]
    InvalidProof,
    /// An error returned by [`c_kzg`].
    #[error("KZG error: {0:?}")]
    KZGError(#[from] c_kzg::Error),
    /// The inner transaction is not a blob transaction.
    #[error("unable to verify proof for non blob transaction: {0}")]
    NotBlobTransaction(u8),
    /// Using a standalone [TxEip4844] instead of the [TxEip4844WithSidecar] variant, which
    /// includes the sidecar for validation.
    #[error("eip4844 tx variant without sidecar being used for verification. Please use the TxEip4844WithSidecar variant, which includes the sidecar")]
    MissingSidecar,
    /// The versioned hash is incorrect.
    #[error("wrong versioned hash: have {have}, expected {expected}")]
    WrongVersionedHash {
        /// The versioned hash we got
        have: B256,
        /// The versioned hash we expected
        expected: B256,
    },
}

/// [EIP-4844 Blob Transaction](https://eips.ethereum.org/EIPS/eip-4844#blob-transaction)
///
/// A transaction with blob hashes and max blob fee.
/// It can either be a standalone transaction, mainly seen when retrieving historical transactions,
/// or a transaction with a sidecar, which is used when submitting a transaction to the network and
/// when receiving and sending transactions during the gossip stage.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum TxEip4844Variant {
    /// A standalone transaction with blob hashes and max blob fee.
    TxEip4844(TxEip4844),
    /// A transaction with a sidecar, which contains the blob data, commitments, and proofs.
    TxEip4844WithSidecar(TxEip4844WithSidecar),
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for TxEip4844Variant {
    fn deserialize<D>(deserializer: D) -> Result<TxEip4844Variant, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct TxEip4844SerdeHelper {
            #[serde(flatten)]
            tx: TxEip4844,
            #[serde(flatten)]
            sidecar: Option<BlobTransactionSidecar>,
        }

        let tx = TxEip4844SerdeHelper::deserialize(deserializer)?;

        if let Some(sidecar) = tx.sidecar {
            Ok(TxEip4844Variant::TxEip4844WithSidecar(TxEip4844WithSidecar::from_tx_and_sidecar(
                tx.tx, sidecar,
            )))
        } else {
            Ok(TxEip4844Variant::TxEip4844(tx.tx))
        }
    }
}

impl From<TxEip4844WithSidecar> for TxEip4844Variant {
    fn from(tx: TxEip4844WithSidecar) -> Self {
        TxEip4844Variant::TxEip4844WithSidecar(tx)
    }
}

impl From<TxEip4844> for TxEip4844Variant {
    fn from(tx: TxEip4844) -> Self {
        TxEip4844Variant::TxEip4844(tx)
    }
}

impl From<(TxEip4844, BlobTransactionSidecar)> for TxEip4844Variant {
    fn from((tx, sidecar): (TxEip4844, BlobTransactionSidecar)) -> Self {
        TxEip4844Variant::TxEip4844WithSidecar(TxEip4844WithSidecar::from_tx_and_sidecar(
            tx, sidecar,
        ))
    }
}

impl TxEip4844Variant {
    /// Verifies that the transaction's blob data, commitments, and proofs are all valid.
    ///
    /// See also [TxEip4844::validate_blob]
    #[cfg(feature = "kzg")]
    pub fn validate(
        &self,
        proof_settings: &KzgSettings,
    ) -> Result<(), BlobTransactionValidationError> {
        match self {
            TxEip4844Variant::TxEip4844(_) => Err(BlobTransactionValidationError::MissingSidecar),
            TxEip4844Variant::TxEip4844WithSidecar(tx) => tx.validate_blob(proof_settings),
        }
    }

    /// Get the transaction type.
    pub const fn tx_type(&self) -> TxType {
        TxType::Eip4844
    }

    /// Get access to the inner tx [TxEip4844].
    pub const fn tx(&self) -> &TxEip4844 {
        match self {
            TxEip4844Variant::TxEip4844(tx) => tx,
            TxEip4844Variant::TxEip4844WithSidecar(tx) => tx.tx(),
        }
    }

    /// Outputs the length of the transaction's fields, without a RLP header.
    #[doc(hidden)]
    pub fn fields_len(&self) -> usize {
        match self {
            TxEip4844Variant::TxEip4844(tx) => tx.fields_len(),
            TxEip4844Variant::TxEip4844WithSidecar(tx) => tx.tx().fields_len(),
        }
    }

    /// Encodes the [TxEip4844Variant] fields as RLP, with a tx type. If `with_header` is `false`,
    /// the following will be encoded:
    /// `tx_type (0x03) || rlp([transaction_payload_body, blobs, commitments, proofs])`
    ///
    /// If `with_header` is `true`, the following will be encoded:
    /// `rlp(tx_type (0x03) || rlp([transaction_payload_body, blobs, commitments, proofs]))`
    #[doc(hidden)]
    pub fn encode_with_signature(
        &self,
        signature: &Signature,
        out: &mut dyn BufMut,
        with_header: bool,
    ) {
        let payload_length = match self {
            TxEip4844Variant::TxEip4844(tx) => tx.fields_len() + signature.rlp_vrs_len(),
            TxEip4844Variant::TxEip4844WithSidecar(tx) => {
                let payload_length = tx.tx().fields_len() + signature.rlp_vrs_len();
                let inner_header = Header { list: true, payload_length };
                inner_header.length() + payload_length + tx.sidecar().fields_len()
            }
        };

        if with_header {
            Header {
                list: false,
                payload_length: 1
                    + Header { list: false, payload_length }.length()
                    + payload_length,
            }
            .encode(out);
        }
        out.put_u8(self.tx_type() as u8);

        match self {
            TxEip4844Variant::TxEip4844(tx) => {
                tx.encode_with_signature_fields(signature, out);
            }
            TxEip4844Variant::TxEip4844WithSidecar(tx) => {
                tx.encode_with_signature_fields(signature, out);
            }
        }
    }

    /// Decodes the transaction from RLP bytes, including the signature.
    ///
    /// This __does not__ expect the bytes to start with a transaction type byte or string
    /// header.
    ///
    /// This __does__ expect the bytes to start with a list header and include a signature.
    #[doc(hidden)]
    pub fn decode_signed_fields(buf: &mut &[u8]) -> alloy_rlp::Result<Signed<Self>> {
        let mut current_buf = *buf;
        let _header = Header::decode(&mut current_buf)?;

        // There are two possibilities when decoding a signed EIP-4844 transaction:
        // If it's a historical transaction, it will only have the transaction fields, and no
        // sidecar. If it's a transaction received during the gossip stage or sent through
        // eth_sendRawTransaction, it will have the transaction fields and a sidecar.
        //
        // To disambiguate, we try to decode two list headers. If there is only one list header, we
        // assume it's a historical transaction. If there are two, we know the transaction contains
        // a sidecar.
        let header = Header::decode(&mut current_buf)?;
        if header.list {
            let tx = TxEip4844WithSidecar::decode_signed_fields(buf)?;
            let (tx, signature, hash) = tx.into_parts();
            return Ok(Signed::new_unchecked(
                TxEip4844Variant::TxEip4844WithSidecar(tx),
                signature,
                hash,
            ));
        }

        // Since there is not a second list header, this is a historical 4844 transaction without a
        // sidecar.
        let tx = TxEip4844::decode_signed_fields(buf)?;
        let (tx, signature, hash) = tx.into_parts();
        Ok(Signed::new_unchecked(TxEip4844Variant::TxEip4844(tx), signature, hash))
    }
}

impl Transaction for TxEip4844Variant {
    fn chain_id(&self) -> Option<ChainId> {
        match self {
            TxEip4844Variant::TxEip4844(tx) => Some(tx.chain_id),
            TxEip4844Variant::TxEip4844WithSidecar(tx) => Some(tx.tx().chain_id),
        }
    }

    fn gas_limit(&self) -> u128 {
        match self {
            TxEip4844Variant::TxEip4844(tx) => tx.gas_limit,
            TxEip4844Variant::TxEip4844WithSidecar(tx) => tx.tx().gas_limit,
        }
    }

    fn gas_price(&self) -> Option<u128> {
        None
    }

    fn input(&self) -> &[u8] {
        match self {
            TxEip4844Variant::TxEip4844(tx) => tx.input.as_ref(),
            TxEip4844Variant::TxEip4844WithSidecar(tx) => tx.tx().input.as_ref(),
        }
    }

    fn nonce(&self) -> u64 {
        match self {
            TxEip4844Variant::TxEip4844(tx) => tx.nonce,
            TxEip4844Variant::TxEip4844WithSidecar(tx) => tx.tx().nonce,
        }
    }

    fn to(&self) -> TxKind {
        let address = match self {
            TxEip4844Variant::TxEip4844(tx) => tx.to,
            TxEip4844Variant::TxEip4844WithSidecar(tx) => tx.tx.to,
        };
        TxKind::Call(address)
    }

    fn value(&self) -> U256 {
        match self {
            TxEip4844Variant::TxEip4844(tx) => tx.value,
            TxEip4844Variant::TxEip4844WithSidecar(tx) => tx.tx.value,
        }
    }
}

impl SignableTransaction<Signature> for TxEip4844Variant {
    fn set_chain_id(&mut self, chain_id: ChainId) {
        match self {
            TxEip4844Variant::TxEip4844(ref mut inner) => {
                inner.chain_id = chain_id;
            }
            TxEip4844Variant::TxEip4844WithSidecar(ref mut inner) => {
                inner.tx.chain_id = chain_id;
            }
        }
    }

    fn payload_len_for_signature(&self) -> usize {
        let payload_length = self.fields_len();
        // 'transaction type byte length' + 'header length' + 'payload length'
        1 + length_of_length(payload_length) + payload_length
    }

    fn into_signed(self, signature: Signature) -> Signed<Self> {
        let payload_length = 1 + self.fields_len() + signature.rlp_vrs_len();
        let mut buf = Vec::with_capacity(payload_length);
        // we use the inner tx to encode the fields
        self.tx().encode_with_signature(&signature, &mut buf, false);
        let hash = keccak256(&buf);

        // Drop any v chain id value to ensure the signature format is correct at the time of
        // combination for an EIP-4844 transaction. V should indicate the y-parity of the
        // signature.
        Signed::new_unchecked(self, signature.with_parity_bool(), hash)
    }

    fn encode_for_signing(&self, out: &mut dyn alloy_rlp::BufMut) {
        // A signature for a [TxEip4844WithSidecar] is a signature over the [TxEip4844Variant]
        // EIP-2718 payload fields:
        // (BLOB_TX_TYPE ||
        //   rlp([chain_id, nonce, max_priority_fee_per_gas, max_fee_per_gas, gas_limit, to, value,
        //     data, access_list, max_fee_per_blob_gas, blob_versioned_hashes]))
        self.tx().encode_for_signing(out);
    }
}

/// [EIP-4844 Blob Transaction](https://eips.ethereum.org/EIPS/eip-4844#blob-transaction)
///
/// A transaction with blob hashes and max blob fee. It does not have the Blob sidecar.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TxEip4844 {
    /// Added as EIP-pub 155: Simple replay attack protection
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::u64_via_ruint"))]
    pub chain_id: ChainId,
    /// A scalar value equal to the number of transactions sent by the sender; formally Tn.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::u64_via_ruint"))]
    pub nonce: u64,
    /// A scalar value equal to the maximum
    /// amount of gas that should be used in executing
    /// this transaction. This is paid up-front, before any
    /// computation is done and may not be increased
    /// later; formally Tg.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::u128_via_ruint"))]
    pub gas_limit: u128,
    /// A scalar value equal to the maximum
    /// amount of gas that should be used in executing
    /// this transaction. This is paid up-front, before any
    /// computation is done and may not be increased
    /// later; formally Tg.
    ///
    /// As ethereum circulation is around 120mil eth as of 2022 that is around
    /// 120000000000000000000000000 wei we are safe to use u128 as its max number is:
    /// 340282366920938463463374607431768211455
    ///
    /// This is also known as `GasFeeCap`
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::u128_via_ruint"))]
    pub max_fee_per_gas: u128,
    /// Max Priority fee that transaction is paying
    ///
    /// As ethereum circulation is around 120mil eth as of 2022 that is around
    /// 120000000000000000000000000 wei we are safe to use u128 as its max number is:
    /// 340282366920938463463374607431768211455
    ///
    /// This is also known as `GasTipCap`
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::u128_via_ruint"))]
    pub max_priority_fee_per_gas: u128,
    /// The 160-bit address of the message call’s recipient.
    pub to: Address,
    /// A scalar value equal to the number of Wei to
    /// be transferred to the message call’s recipient or,
    /// in the case of contract creation, as an endowment
    /// to the newly created account; formally Tv.
    pub value: U256,
    /// The accessList specifies a list of addresses and storage keys;
    /// these addresses and storage keys are added into the `accessed_addresses`
    /// and `accessed_storage_keys` global sets (introduced in EIP-2929).
    /// A gas cost is charged, though at a discount relative to the cost of
    /// accessing outside the list.
    pub access_list: AccessList,

    /// It contains a vector of fixed size hash(32 bytes)
    pub blob_versioned_hashes: Vec<B256>,

    /// Max fee per data gas
    ///
    /// aka BlobFeeCap or blobGasFeeCap
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::u128_via_ruint"))]
    pub max_fee_per_blob_gas: u128,

    /// Input has two uses depending if transaction is Create or Call (if `to` field is None or
    /// Some). pub init: An unlimited size byte array specifying the
    /// EVM-code for the account initialisation procedure CREATE,
    /// data: An unlimited size byte array specifying the
    /// input data of the message call, formally Td.
    pub input: Bytes,
}

impl TxEip4844 {
    /// Returns the effective gas price for the given `base_fee`.
    pub const fn effective_gas_price(&self, base_fee: Option<u64>) -> u128 {
        match base_fee {
            None => self.max_fee_per_gas,
            Some(base_fee) => {
                // if the tip is greater than the max priority fee per gas, set it to the max
                // priority fee per gas + base fee
                let tip = self.max_fee_per_gas.saturating_sub(base_fee as u128);
                if tip > self.max_priority_fee_per_gas {
                    self.max_priority_fee_per_gas + base_fee as u128
                } else {
                    // otherwise return the max fee per gas
                    self.max_fee_per_gas
                }
            }
        }
    }

    /// Returns the total gas for all blobs in this transaction.
    #[inline]
    pub fn blob_gas(&self) -> u64 {
        // SAFETY: we don't expect u64::MAX / DATA_GAS_PER_BLOB hashes in a single transaction
        self.blob_versioned_hashes.len() as u64 * DATA_GAS_PER_BLOB
    }

    /// Verifies that the given blob data, commitments, and proofs are all valid for this
    /// transaction.
    ///
    /// Takes as input the [KzgSettings], which should contain the parameters derived from the
    /// KZG trusted setup.
    ///
    /// This ensures that the blob transaction payload has the same number of blob data elements,
    /// commitments, and proofs. Each blob data element is verified against its commitment and
    /// proof.
    ///
    /// Returns [BlobTransactionValidationError::InvalidProof] if any blob KZG proof in the response
    /// fails to verify, or if the versioned hashes in the transaction do not match the actual
    /// commitment versioned hashes.
    #[cfg(feature = "kzg")]
    pub fn validate_blob(
        &self,
        sidecar: &BlobTransactionSidecar,
        proof_settings: &KzgSettings,
    ) -> Result<(), BlobTransactionValidationError> {
        // Ensure the versioned hashes and commitments have the same length.
        if self.blob_versioned_hashes.len() != sidecar.commitments.len() {
            return Err(c_kzg::Error::MismatchLength(format!(
                "There are {} versioned commitment hashes and {} commitments",
                self.blob_versioned_hashes.len(),
                sidecar.commitments.len()
            ))
            .into());
        }

        // calculate versioned hashes by zipping & iterating
        for (versioned_hash, commitment) in
            self.blob_versioned_hashes.iter().zip(sidecar.commitments.iter())
        {
            let commitment = KzgCommitment::from(*commitment.deref());

            // calculate & verify versioned hash
            let calculated_versioned_hash = kzg_to_versioned_hash(commitment.as_slice());
            if *versioned_hash != calculated_versioned_hash {
                return Err(BlobTransactionValidationError::WrongVersionedHash {
                    have: *versioned_hash,
                    expected: calculated_versioned_hash,
                });
            }
        }

        let res = KzgProof::verify_blob_kzg_proof_batch(
            sidecar.blobs.as_slice(),
            sidecar.commitments.as_slice(),
            sidecar.proofs.as_slice(),
            proof_settings,
        )
        .map_err(BlobTransactionValidationError::KZGError)?;

        if res {
            Ok(())
        } else {
            Err(BlobTransactionValidationError::InvalidProof)
        }
    }

    /// Decodes the inner [TxEip4844Variant] fields from RLP bytes.
    ///
    /// NOTE: This assumes a RLP header has already been decoded, and _just_ decodes the following
    /// RLP fields in the following order:
    ///
    /// - `chain_id`
    /// - `nonce`
    /// - `max_priority_fee_per_gas`
    /// - `max_fee_per_gas`
    /// - `gas_limit`
    /// - `to`
    /// - `value`
    /// - `data` (`input`)
    /// - `access_list`
    /// - `max_fee_per_blob_gas`
    /// - `blob_versioned_hashes`
    pub fn decode_fields(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Ok(Self {
            chain_id: Decodable::decode(buf)?,
            nonce: Decodable::decode(buf)?,
            max_priority_fee_per_gas: Decodable::decode(buf)?,
            max_fee_per_gas: Decodable::decode(buf)?,
            gas_limit: Decodable::decode(buf)?,
            to: Decodable::decode(buf)?,
            value: Decodable::decode(buf)?,
            input: Decodable::decode(buf)?,
            access_list: Decodable::decode(buf)?,
            max_fee_per_blob_gas: Decodable::decode(buf)?,
            blob_versioned_hashes: Decodable::decode(buf)?,
        })
    }

    /// Outputs the length of the transaction's fields, without a RLP header.
    #[doc(hidden)]
    pub fn fields_len(&self) -> usize {
        let mut len = 0;
        len += self.chain_id.length();
        len += self.nonce.length();
        len += self.gas_limit.length();
        len += self.max_fee_per_gas.length();
        len += self.max_priority_fee_per_gas.length();
        len += self.to.length();
        len += self.value.length();
        len += self.access_list.length();
        len += self.blob_versioned_hashes.length();
        len += self.max_fee_per_blob_gas.length();
        len += self.input.0.length();
        len
    }

    /// Encodes only the transaction's fields into the desired buffer, without a RLP header.
    pub(crate) fn encode_fields(&self, out: &mut dyn BufMut) {
        self.chain_id.encode(out);
        self.nonce.encode(out);
        self.max_priority_fee_per_gas.encode(out);
        self.max_fee_per_gas.encode(out);
        self.gas_limit.encode(out);
        self.to.encode(out);
        self.value.encode(out);
        self.input.0.encode(out);
        self.access_list.encode(out);
        self.max_fee_per_blob_gas.encode(out);
        self.blob_versioned_hashes.encode(out);
    }

    /// Calculates a heuristic for the in-memory size of the [TxEip4844Variant] transaction.
    #[inline]
    pub fn size(&self) -> usize {
        mem::size_of::<ChainId>() + // chain_id
        mem::size_of::<u64>() + // nonce
        mem::size_of::<u64>() + // gas_limit
        mem::size_of::<u128>() + // max_fee_per_gas
        mem::size_of::<u128>() + // max_priority_fee_per_gas
        mem::size_of::<Address>() + // to
        mem::size_of::<U256>() + // value
        self.access_list.size() + // access_list
        self.input.len() +  // input
        self.blob_versioned_hashes.capacity() * mem::size_of::<B256>() + // blob hashes size
        mem::size_of::<u128>() // max_fee_per_data_gas
    }

    /// Returns what the encoded length should be, if the transaction were RLP encoded with the
    /// given signature, depending on the value of `with_header`.
    ///
    /// If `with_header` is `true`, the payload length will include the RLP header length.
    /// If `with_header` is `false`, the payload length will not include the RLP header length.
    pub(crate) fn encoded_len_with_signature(
        &self,
        signature: &Signature,
        with_header: bool,
    ) -> usize {
        // this counts the tx fields and signature fields
        let payload_length = self.fields_len() + signature.rlp_vrs_len();

        // this counts:
        // * tx type byte
        // * inner header length
        // * inner payload length
        let inner_payload_length =
            1 + Header { list: true, payload_length }.length() + payload_length;

        if with_header {
            // header length plus length of the above, wrapped with a string header
            Header { list: false, payload_length: inner_payload_length }.length()
                + inner_payload_length
        } else {
            inner_payload_length
        }
    }

    /// Inner encoding function that is used for both rlp [`Encodable`] trait and for calculating
    /// hash that for eip2718 does not require a rlp header
    #[doc(hidden)]
    pub fn encode_with_signature(
        &self,
        signature: &Signature,
        out: &mut dyn BufMut,
        with_header: bool,
    ) {
        let payload_length = self.fields_len() + signature.rlp_vrs_len();
        if with_header {
            Header {
                list: false,
                payload_length: 1 + Header { list: true, payload_length }.length() + payload_length,
            }
            .encode(out);
        }
        out.put_u8(self.tx_type() as u8);
        self.encode_with_signature_fields(signature, out);
    }

    /// Encodes the transaction from RLP bytes, including the signature. This __does not__ encode a
    /// tx type byte or string header.
    ///
    /// This __does__ encode a list header and include a signature.
    pub(crate) fn encode_with_signature_fields(&self, signature: &Signature, out: &mut dyn BufMut) {
        let payload_length = self.fields_len() + signature.rlp_vrs_len();
        let header = Header { list: true, payload_length };
        header.encode(out);
        self.encode_fields(out);
        signature.write_rlp_vrs(out);
    }

    /// Decodes the transaction from RLP bytes, including the signature.
    ///
    /// This __does not__ expect the bytes to start with a transaction type byte or string
    /// header.
    ///
    /// This __does__ expect the bytes to start with a list header and include a signature.
    #[doc(hidden)]
    pub fn decode_signed_fields(buf: &mut &[u8]) -> alloy_rlp::Result<Signed<Self>> {
        let header = Header::decode(buf)?;
        if !header.list {
            return Err(alloy_rlp::Error::UnexpectedString);
        }

        // record original length so we can check encoding
        let original_len = buf.len();

        let tx = Self::decode_fields(buf)?;
        let signature = Signature::decode_rlp_vrs(buf)?;

        let signed = tx.into_signed(signature);
        if buf.len() + header.payload_length != original_len {
            return Err(alloy_rlp::Error::ListLengthMismatch {
                expected: header.payload_length,
                got: original_len - buf.len(),
            });
        }

        Ok(signed)
    }

    /// Get transaction type
    pub const fn tx_type(&self) -> TxType {
        TxType::Eip4844
    }

    /// Encodes the EIP-4844 transaction in RLP for signing.
    ///
    /// This encodes the transaction as:
    /// `tx_type || rlp(chain_id, nonce, max_priority_fee_per_gas, max_fee_per_gas, gas_limit, to,
    /// value, input, access_list, max_fee_per_blob_gas, blob_versioned_hashes)`
    ///
    /// Note that there is no rlp header before the transaction type byte.
    pub fn encode_for_signing(&self, out: &mut dyn BufMut) {
        out.put_u8(self.tx_type() as u8);
        Header { list: true, payload_length: self.fields_len() }.encode(out);
        self.encode_fields(out);
    }

    /// Outputs the length of the signature RLP encoding for the transaction.
    pub fn payload_len_for_signature(&self) -> usize {
        let payload_length = self.fields_len();
        // 'transaction type byte length' + 'header length' + 'payload length'
        1 + Header { list: true, payload_length }.length() + payload_length
    }
}

impl SignableTransaction<Signature> for TxEip4844 {
    fn set_chain_id(&mut self, chain_id: ChainId) {
        self.chain_id = chain_id;
    }

    fn payload_len_for_signature(&self) -> usize {
        self.payload_len_for_signature()
    }

    fn into_signed(self, signature: Signature) -> Signed<Self> {
        let mut buf = Vec::with_capacity(self.encoded_len_with_signature(&signature, false));
        self.encode_with_signature(&signature, &mut buf, false);
        let hash = keccak256(&buf);

        // Drop any v chain id value to ensure the signature format is correct at the time of
        // combination for an EIP-4844 transaction. V should indicate the y-parity of the
        // signature.
        Signed::new_unchecked(self, signature.with_parity_bool(), hash)
    }

    fn encode_for_signing(&self, out: &mut dyn alloy_rlp::BufMut) {
        self.encode_for_signing(out);
    }
}

impl Transaction for TxEip4844 {
    fn input(&self) -> &[u8] {
        &self.input
    }

    fn to(&self) -> TxKind {
        TxKind::Call(self.to)
    }

    fn value(&self) -> U256 {
        self.value
    }

    fn chain_id(&self) -> Option<ChainId> {
        Some(self.chain_id)
    }

    fn nonce(&self) -> u64 {
        self.nonce
    }

    fn gas_limit(&self) -> u128 {
        self.gas_limit
    }

    fn gas_price(&self) -> Option<u128> {
        None
    }
}

impl Encodable for TxEip4844 {
    fn encode(&self, out: &mut dyn BufMut) {
        Header { list: true, payload_length: self.fields_len() }.encode(out);
        self.encode_fields(out);
    }

    fn length(&self) -> usize {
        let payload_length = self.fields_len();
        length_of_length(payload_length) + payload_length
    }
}

impl Decodable for TxEip4844 {
    fn decode(data: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let header = Header::decode(data)?;
        let remaining_len = data.len();

        if header.payload_length > remaining_len {
            return Err(alloy_rlp::Error::InputTooShort);
        }

        Self::decode_fields(data)
    }
}

/// [EIP-4844 Blob Transaction](https://eips.ethereum.org/EIPS/eip-4844#blob-transaction)
///
/// A transaction with blob hashes and max blob fee, which also includes the
/// [BlobTransactionSidecar]. This is the full type sent over the network as a raw transaction. It
/// wraps a [TxEip4844] to include the sidecar and the ability to decode it properly.
///
/// This is defined in [EIP-4844](https://eips.ethereum.org/EIPS/eip-4844#networking) as an element
/// of a `PooledTransactions` response, and is also used as the format for sending raw transactions
/// through the network (eth_sendRawTransaction/eth_sendTransaction).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TxEip4844WithSidecar {
    /// The actual transaction.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub tx: TxEip4844,
    /// The sidecar.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub sidecar: BlobTransactionSidecar,
}

impl TxEip4844WithSidecar {
    /// Constructs a new [TxEip4844WithSidecar] from a [TxEip4844] and a [BlobTransactionSidecar].
    pub const fn from_tx_and_sidecar(tx: TxEip4844, sidecar: BlobTransactionSidecar) -> Self {
        Self { tx, sidecar }
    }

    /// Verifies that the transaction's blob data, commitments, and proofs are all valid.
    ///
    /// See also [TxEip4844::validate_blob]
    #[cfg(feature = "kzg")]
    pub fn validate_blob(
        &self,
        proof_settings: &KzgSettings,
    ) -> Result<(), BlobTransactionValidationError> {
        self.tx.validate_blob(&self.sidecar, proof_settings)
    }

    /// Get the transaction type.
    pub const fn tx_type(&self) -> TxType {
        self.tx.tx_type()
    }

    /// Get access to the inner tx [TxEip4844].
    pub const fn tx(&self) -> &TxEip4844 {
        &self.tx
    }

    /// Get access to the inner sidecar [BlobTransactionSidecar].
    pub const fn sidecar(&self) -> &BlobTransactionSidecar {
        &self.sidecar
    }

    /// Consumes the [TxEip4844WithSidecar] and returns the inner [TxEip4844].
    pub fn into_tx(self) -> TxEip4844 {
        self.tx
    }

    /// Consumes the [TxEip4844WithSidecar] and returns the inner sidecar [BlobTransactionSidecar].
    pub fn into_sidecar(self) -> BlobTransactionSidecar {
        self.sidecar
    }

    /// Consumes the [TxEip4844WithSidecar] and returns the inner [TxEip4844] and
    /// [BlobTransactionSidecar].
    pub fn into_parts(self) -> (TxEip4844, BlobTransactionSidecar) {
        (self.tx, self.sidecar)
    }

    /// Encodes the transaction from RLP bytes, including the signature. This __does not__ encode a
    /// tx type byte or string header.
    ///
    /// This __does__ encode a list header and include a signature.
    ///
    /// This encodes the following:
    /// `rlp([tx_payload, blobs, commitments, proofs])`
    ///
    /// where `tx_payload` is the RLP encoding of the [TxEip4844] transaction fields:
    /// `rlp([chain_id, nonce, max_priority_fee_per_gas, ..., v, r, s])`
    pub fn encode_with_signature_fields(&self, signature: &Signature, out: &mut dyn BufMut) {
        let inner_payload_length = self.tx.fields_len() + signature.rlp_vrs_len();
        let inner_header = Header { list: true, payload_length: inner_payload_length };

        let outer_payload_length =
            inner_header.length() + inner_payload_length + self.sidecar.fields_len();
        let outer_header = Header { list: true, payload_length: outer_payload_length };

        // write the two headers
        outer_header.encode(out);
        inner_header.encode(out);

        // now write the fields
        self.tx.encode_fields(out);
        signature.write_rlp_vrs(out);
        self.sidecar.encode_inner(out);
    }

    /// Decodes the transaction from RLP bytes, including the signature.
    ///
    /// This __does not__ expect the bytes to start with a transaction type byte or string
    /// header.
    ///
    /// This __does__ expect the bytes to start with a list header and include a signature.
    ///
    /// This is the inverse of [TxEip4844WithSidecar::encode_with_signature_fields].
    #[doc(hidden)]
    pub fn decode_signed_fields(buf: &mut &[u8]) -> alloy_rlp::Result<Signed<Self>> {
        let header = Header::decode(buf)?;
        if !header.list {
            return Err(alloy_rlp::Error::UnexpectedString);
        }

        // record original length so we can check encoding
        let original_len = buf.len();

        // decode the inner tx
        let inner_tx = TxEip4844::decode_signed_fields(buf)?;

        // decode the sidecar
        let sidecar = BlobTransactionSidecar::decode_inner(buf)?;

        if buf.len() + header.payload_length != original_len {
            return Err(alloy_rlp::Error::ListLengthMismatch {
                expected: header.payload_length,
                got: original_len - buf.len(),
            });
        }

        let (tx, signature, hash) = inner_tx.into_parts();

        // create unchecked signed tx because these checks should have happened during construction
        // of `Signed<TxEip4844>` in `TxEip4844::decode_signed_fields`
        Ok(Signed::new_unchecked(
            TxEip4844WithSidecar::from_tx_and_sidecar(tx, sidecar),
            signature,
            hash,
        ))
    }
}

impl SignableTransaction<Signature> for TxEip4844WithSidecar {
    fn set_chain_id(&mut self, chain_id: ChainId) {
        self.tx.chain_id = chain_id;
    }

    fn encode_for_signing(&self, out: &mut dyn alloy_rlp::BufMut) {
        // A signature for a [TxEip4844WithSidecar] is a signature over the [TxEip4844] EIP-2718
        // payload fields:
        // (BLOB_TX_TYPE ||
        //   rlp([chain_id, nonce, max_priority_fee_per_gas, max_fee_per_gas, gas_limit, to, value,
        //     data, access_list, max_fee_per_blob_gas, blob_versioned_hashes]))
        self.tx.encode_for_signing(out);
    }

    fn into_signed(self, signature: Signature) -> Signed<Self, Signature> {
        let mut buf = Vec::with_capacity(self.tx.encoded_len_with_signature(&signature, false));
        // The sidecar is NOT included in the signed payload, only the transaction fields and the
        // type byte. Include the type byte.
        //
        // Include the transaction fields, making sure to __not__ use the sidecar, and __not__
        // encode a header.
        self.tx.encode_with_signature(&signature, &mut buf, false);
        let hash = keccak256(&buf);

        // Drop any v chain id value to ensure the signature format is correct at the time of
        // combination for an EIP-4844 transaction. V should indicate the y-parity of the
        // signature.
        Signed::new_unchecked(self, signature.with_parity_bool(), hash)
    }

    fn payload_len_for_signature(&self) -> usize {
        // The payload length is the length of the `transaction_payload_body` list.
        // The sidecar is NOT included.
        self.tx.payload_len_for_signature()
    }
}

impl Transaction for TxEip4844WithSidecar {
    fn chain_id(&self) -> Option<ChainId> {
        self.tx.chain_id()
    }

    fn gas_limit(&self) -> u128 {
        self.tx.gas_limit()
    }

    fn gas_price(&self) -> Option<u128> {
        self.tx.gas_price()
    }

    fn nonce(&self) -> u64 {
        self.tx.nonce()
    }

    fn to(&self) -> TxKind {
        self.tx.to()
    }

    fn value(&self) -> U256 {
        self.tx.value()
    }

    fn input(&self) -> &[u8] {
        self.tx.input()
    }
}

/// This represents a set of blobs, and its corresponding commitments and proofs.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlobTransactionSidecar {
    /// The blob data.
    pub blobs: Vec<Blob>,
    /// The blob commitments.
    pub commitments: Vec<Bytes48>,
    /// The blob proofs.
    pub proofs: Vec<Bytes48>,
}

impl BlobTransactionSidecar {
    /// Constructs a new [BlobTransactionSidecar] from a set of blobs, commitments, and proofs.
    pub fn new(blobs: Vec<Blob>, commitments: Vec<Bytes48>, proofs: Vec<Bytes48>) -> Self {
        Self { blobs, commitments, proofs }
    }

    /// Returns an iterator over the versioned hashes of the commitments.
    pub fn versioned_hashes(&self) -> impl Iterator<Item = B256> + '_ {
        self.commitments.iter().map(|c| kzg_to_versioned_hash(c.as_slice()))
    }

    /// Returns the versioned hash for the blob at the given index, if it
    /// exists.
    pub fn versioned_hash_for_blob(&self, blob_index: usize) -> Option<B256> {
        self.commitments.get(blob_index).map(|c| kzg_to_versioned_hash(c.as_slice()))
    }

    /// Encodes the inner [BlobTransactionSidecar] fields as RLP bytes, without a RLP header.
    ///
    /// This encodes the fields in the following order:
    /// - `blobs`
    /// - `commitments`
    /// - `proofs`
    #[inline]
    pub(crate) fn encode_inner(&self, out: &mut dyn BufMut) {
        BlobTransactionSidecarRlp::wrap_ref(self).encode(out);
    }

    /// Decodes the inner [BlobTransactionSidecar] fields from RLP bytes, without a RLP header.
    ///
    /// This decodes the fields in the following order:
    /// - `blobs`
    /// - `commitments`
    /// - `proofs`
    pub(crate) fn decode_inner(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Ok(BlobTransactionSidecarRlp::decode(buf)?.unwrap())
    }

    /// Outputs the RLP length of the [BlobTransactionSidecar] fields, without a RLP header.
    pub fn fields_len(&self) -> usize {
        BlobTransactionSidecarRlp::wrap_ref(self).fields_len()
    }

    /// Calculates a size heuristic for the in-memory size of the [BlobTransactionSidecar].
    #[inline]
    pub fn size(&self) -> usize {
        self.blobs.len() * BYTES_PER_BLOB + // blobs
        self.commitments.len() * BYTES_PER_COMMITMENT + //   commitments
        self.proofs.len() * BYTES_PER_PROOF // proofs
    }
}

impl Encodable for BlobTransactionSidecar {
    /// Encodes the inner [BlobTransactionSidecar] fields as RLP bytes, without a RLP header.
    fn encode(&self, s: &mut dyn BufMut) {
        self.encode_inner(s);
    }

    fn length(&self) -> usize {
        self.fields_len()
    }
}

impl Decodable for BlobTransactionSidecar {
    /// Decodes the inner [BlobTransactionSidecar] fields from RLP bytes, without a RLP header.
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Self::decode_inner(buf)
    }
}

// Wrapper for c-kzg rlp
#[repr(C)]
struct BlobTransactionSidecarRlp {
    blobs: Vec<FixedBytes<BYTES_PER_BLOB>>,
    commitments: Vec<FixedBytes<BYTES_PER_COMMITMENT>>,
    proofs: Vec<FixedBytes<BYTES_PER_PROOF>>,
}

const _: [(); mem::size_of::<BlobTransactionSidecar>()] =
    [(); mem::size_of::<BlobTransactionSidecarRlp>()];

impl BlobTransactionSidecarRlp {
    const fn wrap_ref(other: &BlobTransactionSidecar) -> &Self {
        // SAFETY: Same repr and size
        unsafe { &*(other as *const BlobTransactionSidecar).cast::<Self>() }
    }

    fn unwrap(self) -> BlobTransactionSidecar {
        // SAFETY: Same repr and size
        unsafe { mem::transmute(self) }
    }

    fn encode(&self, out: &mut dyn BufMut) {
        // Encode the blobs, commitments, and proofs
        self.blobs.encode(out);
        self.commitments.encode(out);
        self.proofs.encode(out);
    }

    fn fields_len(&self) -> usize {
        self.blobs.length() + self.commitments.length() + self.proofs.length()
    }

    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Ok(Self {
            blobs: Decodable::decode(buf)?,
            commitments: Decodable::decode(buf)?,
            proofs: Decodable::decode(buf)?,
        })
    }
}

/// Calculates the versioned hash for a KzgCommitment
///
/// Specified in [EIP-4844](https://eips.ethereum.org/EIPS/eip-4844#header-extension)
pub(crate) fn kzg_to_versioned_hash(commitment: &[u8]) -> B256 {
    debug_assert_eq!(commitment.len(), 48, "commitment length is not 48");
    let mut res = Sha256::digest(commitment);
    res[0] = alloy_eips::eip4844::VERSIONED_HASH_VERSION_KZG;
    B256::new(res.into())
}

#[cfg(test)]
mod tests {
    use super::{BlobTransactionSidecar, TxEip4844, TxEip4844WithSidecar};
    use crate::{SignableTransaction, TxEip4844Variant, TxEnvelope};
    use alloy_eips::eip2930::AccessList;
    use alloy_primitives::{address, b256, bytes, Signature, U256};
    use alloy_rlp::{Decodable, Encodable};

    #[cfg(not(feature = "kzg"))]
    use alloy_eips::eip4844::{Blob, Bytes48};

    #[cfg(feature = "kzg")]
    use c_kzg::{Blob, Bytes48};

    #[test]
    fn different_sidecar_same_hash() {
        // this should make sure that the hash calculated for the `into_signed` conversion does not
        // change if the sidecar is different
        let tx = TxEip4844 {
            chain_id: 1,
            nonce: 1,
            max_priority_fee_per_gas: 1,
            max_fee_per_gas: 1,
            gas_limit: 1,
            to: Default::default(),
            value: U256::from(1),
            access_list: Default::default(),
            blob_versioned_hashes: vec![Default::default()],
            max_fee_per_blob_gas: 1,
            input: Default::default(),
        };
        let sidecar = BlobTransactionSidecar {
            blobs: vec![Blob::from([2; 131072])],
            commitments: vec![Bytes48::from([3; 48])],
            proofs: vec![Bytes48::from([4; 48])],
        };
        let mut tx = TxEip4844WithSidecar { tx, sidecar };
        let signature = Signature::test_signature();

        // turn this transaction into_signed
        let expected_signed = tx.clone().into_signed(signature);

        // change the sidecar, adding a single (blob, commitment, proof) pair
        tx.sidecar = BlobTransactionSidecar {
            blobs: vec![Blob::from([1; 131072])],
            commitments: vec![Bytes48::from([1; 48])],
            proofs: vec![Bytes48::from([1; 48])],
        };

        // turn this transaction into_signed
        let actual_signed = tx.into_signed(signature);

        // the hashes should be the same
        assert_eq!(expected_signed.hash(), actual_signed.hash());

        // convert to envelopes
        let expected_envelope: TxEnvelope = expected_signed.into();
        let actual_envelope: TxEnvelope = actual_signed.into();

        // now encode the transaction and check the length
        let mut buf = Vec::new();
        expected_envelope.encode(&mut buf);
        assert_eq!(buf.len(), expected_envelope.length());

        // ensure it's also the same size that `actual` claims to be, since we just changed the
        // sidecar values.
        assert_eq!(buf.len(), actual_envelope.length());

        // now decode the transaction and check the values
        let decoded = TxEnvelope::decode(&mut &buf[..]).unwrap();
        assert_eq!(decoded, expected_envelope);
    }

    #[test]
    fn test_4844_variant_into_signed_correct_hash() {
        // Taken from <https://etherscan.io/tx/0x93fc9daaa0726c3292a2e939df60f7e773c6a6a726a61ce43f4a217c64d85e87>
        let tx =
            TxEip4844 {
                chain_id: 1,
                nonce: 15435,
                gas_limit: 8000000,
                max_fee_per_gas: 10571233596,
                max_priority_fee_per_gas: 1000000000,
                to: address!("a8cb082a5a689e0d594d7da1e2d72a3d63adc1bd"),
                value: U256::ZERO,
                access_list: AccessList::default(),
                blob_versioned_hashes: vec![
                    b256!("01e5276d91ac1ddb3b1c2d61295211220036e9a04be24c00f76916cc2659d004"),
                    b256!("0128eb58aff09fd3a7957cd80aa86186d5849569997cdfcfa23772811b706cc2"),
                ],
                max_fee_per_blob_gas: 1,
                input: bytes!("701f58c50000000000000000000000000000000000000000000000000000000000073fb1ed12e288def5b439ea074b398dbb4c967f2852baac3238c5fe4b62b871a59a6d00000000000000000000000000000000000000000000000000000000123971da000000000000000000000000000000000000000000000000000000000000000ac39b2a24e1dbdd11a1e7bd7c0f4dfd7d9b9cfa0997d033ad05f961ba3b82c6c83312c967f10daf5ed2bffe309249416e03ee0b101f2b84d2102b9e38b0e4dfdf0000000000000000000000000000000000000000000000000000000066254c8b538dcc33ecf5334bbd294469f9d4fd084a3090693599a46d6c62567747cbc8660000000000000000000000000000000000000000000000000000000000000120000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000073fb20000000000000000000000000000000000000000000000000000000066254da10000000000000000000000000000000000000000000000000000000012397d5e20b09b263779fda4171c341e720af8fa469621ff548651f8dbbc06c2d320400c000000000000000000000000000000000000000000000000000000000000000b50a833bb11af92814e99c6ff7cf7ba7042827549d6f306a04270753702d897d8fc3c411b99159939ac1c16d21d3057ddc8b2333d1331ab34c938cff0eb29ce2e43241c170344db6819f76b1f1e0ab8206f3ec34120312d275c4f5bbea7f5c55700000000000000000000000000000000000000000000000000000000000001400000000000000000000000000000000000000000000000000000000000000480000000000000000000000000000000000000000000000000000000000000031800000000000000000000000000000000000000000000800b0000000000000000000000000000000000000000000000000000000000000004ed12e288def5b439ea074b398dbb4c967f2852baac3238c5fe4b62b871a59a6d00000ca8000000000000000000000000000000000000800b000000000000000000000000000000000000000000000000000000000000000300000000000000000000000066254da100000000000000000000000066254e9d00010ca80000000000000000000000000000000000008001000000000000000000000000000000000000000000000000000000000000000550a833bb11af92814e99c6ff7cf7ba7042827549d6f306a04270753702d897d800010ca800000000000000000000000000000000000080010000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000b00010ca8000000000000000000000000000000000000801100000000000000000000000000000000000000000000000000000000000000075c1cd5bd0fd333ce9d7c8edfc79f43b8f345b4a394f6aba12a2cc78ce4012ed700010ca80000000000000000000000000000000000008011000000000000000000000000000000000000000000000000000000000000000845392775318aa47beaafbdc827da38c9f1e88c3bdcabba2cb493062e17cbf21e00010ca800000000000000000000000000000000000080080000000000000000000000000000000000000000000000000000000000000000c094e20e7ac9b433f44a5885e3bdc07e51b309aeb993caa24ba84a661ac010c100010ca800000000000000000000000000000000000080080000000000000000000000000000000000000000000000000000000000000001ab42db8f4ed810bdb143368a2b641edf242af6e3d0de8b1486e2b0e7880d431100010ca8000000000000000000000000000000000000800800000000000000000000000000000000000000000000000000000000000000022d94e4cc4525e4e2d81e8227b6172e97076431a2cf98792d978035edd6e6f3100000000000000000000000000000000000000000000000000000000000000000000000000000012101c74dfb80a80fccb9a4022b2406f79f56305e6a7c931d30140f5d372fe793837e93f9ec6b8d89a9d0ab222eeb27547f66b90ec40fbbdd2a4936b0b0c19ca684ff78888fbf5840d7c8dc3c493b139471750938d7d2c443e2d283e6c5ee9fde3765a756542c42f002af45c362b4b5b1687a8fc24cbf16532b903f7bb289728170dcf597f5255508c623ba247735538376f494cdcdd5bd0c4cb067526eeda0f4745a28d8baf8893ecc1b8cee80690538d66455294a028da03ff2add9d8a88e6ee03ba9ffe3ad7d91d6ac9c69a1f28c468f00fe55eba5651a2b32dc2458e0d14b4dd6d0173df255cd56aa01e8e38edec17ea8933f68543cbdc713279d195551d4211bed5c91f77259a695e6768f6c4b110b2158fcc42423a96dcc4e7f6fddb3e2369d00000000000000000000000000000000000000000000000000000000000000") };
        let variant = TxEip4844Variant::TxEip4844(tx);

        let signature = Signature::from_rs_and_parity(
            b256!("6c173c3c8db3e3299f2f728d293b912c12e75243e3aa66911c2329b58434e2a4").into(),
            b256!("7dd4d1c228cedc5a414a668ab165d9e888e61e4c3b44cd7daf9cdcc4cec5d6b2").into(),
            false,
        )
        .unwrap();

        let signed = variant.into_signed(signature);
        assert_eq!(
            *signed.hash(),
            b256!("93fc9daaa0726c3292a2e939df60f7e773c6a6a726a61ce43f4a217c64d85e87")
        );
    }
}
