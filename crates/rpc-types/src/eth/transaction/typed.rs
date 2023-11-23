#![allow(missing_docs)]
//! The [`TransactionRequest`][crate::TransactionRequest] is a universal representation for a
//! transaction deserialized from the json input of an RPC call. Depending on what fields are set,
//! it can be converted into the container type [`TypedTransactionRequest`].

use std::{mem, cmp::Ordering};

use crate::{eth::transaction::AccessList, Signature, TxType};
use alloy_primitives::{keccak256, Address, Bytes, B256, U128, U256, U64};
use alloy_rlp::{bytes, length_of_length, BufMut, Decodable, Encodable, Error as RlpError, Header, EMPTY_LIST_CODE, Buf};
use serde::{Deserialize, Serialize};

/// Container type for various Ethereum transaction requests
///
/// Its variants correspond to specific allowed transactions:
/// 1. Legacy (pre-EIP2718) [`LegacyTransactionRequest`]
/// 2. EIP2930 (state access lists) [`EIP2930TransactionRequest`]
/// 3. EIP1559 [`EIP1559TransactionRequest`]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TypedTransactionRequest {
    /// A Legacy Transaction request.
    Legacy(LegacyTransactionRequest),
    /// An EIP2930 transaction request.
    EIP2930(EIP2930TransactionRequest),
    /// An EIP1559 Transaction Request.
    EIP1559(EIP1559TransactionRequest),
}

impl Encodable for TypedTransactionRequest {
    fn encode(&self, out: &mut dyn BufMut) {
        match self {
            // Just encode as such
            TypedTransactionRequest::Legacy(tx) => tx.encode(out),
            // For EIP2930 and EIP1559 txs, we need to "envelop" the RLP encoding with the tx type.
            // For EIP2930, it's 1.
            TypedTransactionRequest::EIP2930(tx) => {
                let id = 1 as u8;
                id.encode(out);
                tx.encode(out)
            },
            // For EIP1559, it's 2.
            TypedTransactionRequest::EIP1559(tx) => {
                let id = 2 as u8;
                id.encode(out);
                tx.encode(out)
            },
        }
    }

    fn length(&self) -> usize {
        match self {
            TypedTransactionRequest::Legacy(tx) => tx.length(),
            TypedTransactionRequest::EIP2930(tx) => tx.length(),
            TypedTransactionRequest::EIP1559(tx) => tx.length(),
        }
    }
}

impl Decodable for TypedTransactionRequest {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        // First, decode the tx type.
        let tx_type = u8::decode(buf)?;
        // Then, decode the tx based on the type.
        match tx_type.cmp(&EMPTY_LIST_CODE) {
            Ordering::Less => {
                // strip out the string header
                // NOTE: typed transaction encodings either contain a "rlp header" which contains
                // the type of the payload and its length, or they do not contain a header and
                // start with the tx type byte.
                //
                // This line works for both types of encodings because byte slices starting with
                // 0x01 and 0x02 return a Header { list: false, payload_length: 1 } when input to
                // Header::decode.
                // If the encoding includes a header, the header will be properly decoded and
                // consumed.
                // Otherwise, header decoding will succeed but nothing is consumed.
                let _header = Header::decode(buf)?;
                let tx_type = *buf.first().ok_or(RlpError::Custom(
                    "typed tx cannot be decoded from an empty slice",
                ))?;
                if tx_type == 0x01 {
                    buf.advance(1);
                    EIP2930TransactionRequest::decode(buf)
                        .map(TypedTransactionRequest::EIP2930)
                } else if tx_type == 0x02 {
                    buf.advance(1);
                    EIP1559TransactionRequest::decode(buf)
                        .map(TypedTransactionRequest::EIP1559)
                } else {
                    Err(RlpError::Custom("invalid tx type"))
                }
            },
            Ordering::Equal => Err(RlpError::Custom("an empty list is not a valid transaction encoding")),
            Ordering::Greater => LegacyTransactionRequest::decode(buf).map(TypedTransactionRequest::Legacy),
        }
    }
}

/// Represents a legacy transaction request
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyTransactionRequest {
    pub nonce: U64,
    pub gas_price: U128,
    pub gas_limit: U256,
    pub kind: TransactionKind,
    pub value: U256,
    pub input: Bytes,
    pub chain_id: Option<u64>,
}

impl Encodable for LegacyTransactionRequest {
    fn encode(&self, out: &mut dyn BufMut) {
        self.nonce.encode(out);
        self.gas_price.encode(out);
        self.gas_limit.encode(out);
        self.kind.encode(out);
        self.value.encode(out);
        self.input.0.encode(out);
    }

    fn length(&self) -> usize {
        self.nonce.length() +
        self.gas_price.length() +
        self.gas_limit.length() +
        self.kind.length() +
        self.value.length() +
        self.input.0.length()
    }
}

impl Decodable for LegacyTransactionRequest {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Ok(Self {
            nonce: Decodable::decode(buf)?,
            gas_price: Decodable::decode(buf)?,
            gas_limit: Decodable::decode(buf)?,
            kind: Decodable::decode(buf)?,
            value: Decodable::decode(buf)?,
            input: Decodable::decode(buf)?,
            chain_id: None,
        })
    }
}

impl LegacyTransactionRequest {
    /// Calculates a heuristic for the in-memory size of the [LegacyTransactionRequest] transaction.
    #[inline]
    pub fn size(&self) -> usize {
        mem::size_of::<Option<u64>>() + // chain_id
        mem::size_of::<U64>() + // nonce
        mem::size_of::<U128>() + // gas_price
        mem::size_of::<U256>() + // gas_limit
        self.kind.size() + // to
        mem::size_of::<U256>() + // value
        self.input.len() // input
    }

    /// Outputs the length of the transaction's fields, without a RLP header or length of the
    /// eip155 fields.
    pub fn fields_len(&self) -> usize {
        let mut len = 0;
        len += self.nonce.length();
        len += self.gas_price.length();
        len += self.gas_limit.length();
        len += self.kind.length();
        len += self.value.length();
        len += self.input.0.length();
        len
    }

    /// Encodes only the transaction's fields into the desired buffer, without a RLP header or
    /// eip155 fields.
    pub fn encode_fields(&self, out: &mut dyn bytes::BufMut) {
        self.nonce.encode(out);
        self.gas_price.encode(out);
        self.gas_limit.encode(out);
        self.kind.encode(out);
        self.value.encode(out);
        self.input.0.encode(out);
    }

    /// Outputs the length of EIP-155 fields. Only outputs a non-zero value for EIP-155 legacy
    /// transactions.
    pub fn eip155_fields_len(&self) -> usize {
        if let Some(id) = self.chain_id {
            // EIP-155 encodes the chain ID and two zeroes, so we add 2 to the length of the chain
            // ID to get the length of all 3 fields
            // len(chain_id) + (0x00) + (0x00)
            id.length() + 2
        } else {
            // this is either a pre-EIP-155 legacy transaction or a typed transaction
            0
        }
    }

    /// Encodes EIP-155 arguments into the desired buffer. Only encodes values for legacy
    /// transactions.
    pub fn encode_eip155_fields(&self, out: &mut dyn bytes::BufMut) {
        // if this is a legacy transaction without a chain ID, it must be pre-EIP-155
        // and does not need to encode the chain ID for the signature hash encoding
        if let Some(id) = self.chain_id {
            // EIP-155 encodes the chain ID and two zeroes
            id.encode(out);
            0x00u8.encode(out);
            0x00u8.encode(out);
        }
    }

    /// Encodes the legacy transaction in RLP for signing, including the EIP-155 fields if possible.
    pub fn encode_for_signing(&self, out: &mut dyn bytes::BufMut) {
        Header { list: true, payload_length: self.fields_len() + self.eip155_fields_len() }
            .encode(out);
        self.encode_fields(out);
        self.encode_eip155_fields(out);
    }

    /// Outputs the length of the signature RLP encoding for the transaction, including the length
    /// of the EIP-155 fields if possible.
    pub fn payload_len_for_signature(&self) -> usize {
        let payload_length = self.fields_len() + self.eip155_fields_len();
        // 'header length' + 'payload length'
        length_of_length(payload_length) + payload_length
    }

    /// Outputs the signature hash of the transaction by first encoding without a signature, then
    /// hashing.
    ///
    /// See [Self::encode_for_signing] for more information on the encoding format.
    pub fn signature_hash(&self) -> B256 {
        let mut buf = bytes::BytesMut::with_capacity(self.payload_len_for_signature());
        self.encode_for_signing(&mut buf);
        keccak256(&buf)
    }
}

/// Represents an EIP-2930 transaction request
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EIP2930TransactionRequest {
    pub chain_id: u64,
    pub nonce: U64,
    pub gas_price: U128,
    pub gas_limit: U256,
    pub kind: TransactionKind,
    pub value: U256,
    pub input: Bytes,
    pub access_list: AccessList,
}

impl Encodable for EIP2930TransactionRequest {
    fn encode(&self, out: &mut dyn BufMut) {
        self.chain_id.encode(out);
        self.nonce.encode(out);
        self.gas_price.encode(out);
        self.gas_limit.encode(out);
        self.kind.encode(out);
        self.value.encode(out);
        self.input.0.encode(out);
        self.access_list.encode(out);
    }

    fn length(&self) -> usize {
        self.chain_id.length() +
        self.nonce.length() +
        self.gas_price.length() +
        self.gas_limit.length() +
        self.kind.length() +
        self.value.length() +
        self.input.0.length() +
        self.access_list.length()
    }
}

impl Decodable for EIP2930TransactionRequest {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Ok(Self {
            chain_id: Decodable::decode(buf)?,
            nonce: Decodable::decode(buf)?,
            gas_price: Decodable::decode(buf)?,
            gas_limit: Decodable::decode(buf)?,
            kind: Decodable::decode(buf)?,
            value: Decodable::decode(buf)?,
            input: Decodable::decode(buf)?,
            access_list: Decodable::decode(buf)?,
        })
    }
}

impl EIP2930TransactionRequest {
    /// Calculates a heuristic for the in-memory size of the transaction.
    #[inline]
    pub fn size(&self) -> usize {
        mem::size_of::<u64>() + // chain_id
        mem::size_of::<u64>() + // nonce
        mem::size_of::<u128>() + // gas_price
        mem::size_of::<u64>() + // gas_limit
        self.kind.size() + // to
        mem::size_of::<U256>() + // value
        self.access_list.size() + // access_list
        self.input.len() // input
    }

    /// Decodes the inner fields from RLP bytes.
    ///
    /// NOTE: This assumes a RLP header has already been decoded, and _just_ decodes the following
    /// RLP fields in the following order:
    ///
    /// - `chain_id`
    /// - `nonce`
    /// - `gas_price`
    /// - `gas_limit`
    /// - `to`
    /// - `value`
    /// - `data` (`input`)
    /// - `access_list`
    pub fn decode_inner(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Ok(Self {
            chain_id: Decodable::decode(buf)?,
            nonce: Decodable::decode(buf)?,
            gas_price: Decodable::decode(buf)?,
            gas_limit: Decodable::decode(buf)?,
            kind: Decodable::decode(buf)?,
            value: Decodable::decode(buf)?,
            input: Decodable::decode(buf)?,
            access_list: Decodable::decode(buf)?,
        })
    }

    /// Outputs the length of the transaction's fields, without a RLP header.
    pub fn fields_len(&self) -> usize {
        let mut len = 0;
        len += self.chain_id.length();
        len += self.nonce.length();
        len += self.gas_price.length();
        len += self.gas_limit.length();
        len += self.kind.length();
        len += self.value.length();
        len += self.input.0.length();
        len += self.access_list.length();
        len
    }

    /// Encodes only the transaction's fields into the desired buffer, without a RLP header.
    pub fn encode_fields(&self, out: &mut dyn bytes::BufMut) {
        self.chain_id.encode(out);
        self.nonce.encode(out);
        self.gas_price.encode(out);
        self.gas_limit.encode(out);
        self.kind.encode(out);
        self.value.encode(out);
        self.input.0.encode(out);
        self.access_list.encode(out);
    }

    /// Inner encoding function that is used for both rlp [`Encodable`] trait and for calculating
    /// hash that for eip2718 does not require rlp header
    pub fn encode_with_signature(
        &self,
        signature: &Signature,
        out: &mut dyn bytes::BufMut,
        with_header: bool,
    ) {
        let payload_length = self.fields_len() + signature.payload_len();
        if with_header {
            Header {
                list: false,
                payload_length: 1 + length_of_length(payload_length) + payload_length,
            }
            .encode(out);
        }
        out.put_u8(self.tx_type() as u8);
        let header = Header { list: true, payload_length };
        header.encode(out);
        self.encode_fields(out);
        signature.encode(out);
    }

    /// Output the length of the RLP signed transaction encoding, _without_ a RLP string header.
    pub fn payload_len_with_signature_without_header(&self, signature: &Signature) -> usize {
        let payload_length = self.fields_len() + signature.payload_len();
        // 'transaction type byte length' + 'header length' + 'payload length'
        1 + length_of_length(payload_length) + payload_length
    }

    /// Output the length of the RLP signed transaction encoding. This encodes with a RLP header.
    pub fn payload_len_with_signature(&self, signature: &Signature) -> usize {
        let len = self.payload_len_with_signature_without_header(signature);
        length_of_length(len) + len
    }

    /// Get transaction type
    pub fn tx_type(&self) -> TxType {
        TxType::EIP2930
    }

    /// Encodes the EIP-2930 transaction in RLP for signing.
    pub fn encode_for_signing(&self, out: &mut dyn bytes::BufMut) {
        out.put_u8(self.tx_type() as u8);
        Header { list: true, payload_length: self.fields_len() }.encode(out);
        self.encode_fields(out);
    }

    /// Outputs the length of the signature RLP encoding for the transaction.
    pub fn payload_len_for_signature(&self) -> usize {
        let payload_length = self.fields_len();
        // 'transaction type byte length' + 'header length' + 'payload length'
        1 + length_of_length(payload_length) + payload_length
    }

    /// Outputs the signature hash of the transaction by first encoding without a signature, then
    /// hashing.
    pub fn signature_hash(&self) -> B256 {
        let mut buf = bytes::BytesMut::with_capacity(self.payload_len_for_signature());
        self.encode_for_signing(&mut buf);
        keccak256(&buf)
    }
}

/// Represents an EIP-1559 transaction request
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EIP1559TransactionRequest {
    pub chain_id: u64,
    pub nonce: U64,
    pub max_priority_fee_per_gas: U128,
    pub max_fee_per_gas: U128,
    pub gas_limit: U256,
    pub kind: TransactionKind,
    pub value: U256,
    pub input: Bytes,
    pub access_list: AccessList,
}

impl Encodable for EIP1559TransactionRequest {
    fn encode(&self, out: &mut dyn BufMut) {
        self.chain_id.encode(out);
        self.nonce.encode(out);
        self.max_priority_fee_per_gas.encode(out);
        self.max_fee_per_gas.encode(out);
        self.gas_limit.encode(out);
        self.kind.encode(out);
        self.value.encode(out);
        self.input.0.encode(out);
        self.access_list.encode(out);
    }

    fn length(&self) -> usize {
        self.chain_id.length() +
        self.nonce.length() +
        self.max_priority_fee_per_gas.length() +
        self.max_fee_per_gas.length() +
        self.gas_limit.length() +
        self.kind.length() +
        self.value.length() +
        self.input.0.length() +
        self.access_list.length()
    }
}

impl Decodable for EIP1559TransactionRequest {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Ok(Self {
            chain_id: Decodable::decode(buf)?,
            nonce: Decodable::decode(buf)?,
            max_priority_fee_per_gas: Decodable::decode(buf)?,
            max_fee_per_gas: Decodable::decode(buf)?,
            gas_limit: Decodable::decode(buf)?,
            kind: Decodable::decode(buf)?,
            value: Decodable::decode(buf)?,
            input: Decodable::decode(buf)?,
            access_list: Decodable::decode(buf)?,
        })
    }
}

impl EIP1559TransactionRequest {
    /// Decodes the inner fields from RLP bytes.
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
    pub fn decode_inner(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Ok(Self {
            chain_id: Decodable::decode(buf)?,
            nonce: Decodable::decode(buf)?,
            max_priority_fee_per_gas: Decodable::decode(buf)?,
            max_fee_per_gas: Decodable::decode(buf)?,
            gas_limit: Decodable::decode(buf)?,
            kind: Decodable::decode(buf)?,
            value: Decodable::decode(buf)?,
            input: Decodable::decode(buf)?,
            access_list: Decodable::decode(buf)?,
        })
    }

    /// Encodes only the transaction's fields into the desired buffer, without a RLP header.
    pub fn fields_len(&self) -> usize {
        let mut len = 0;
        len += self.chain_id.length();
        len += self.nonce.length();
        len += self.max_priority_fee_per_gas.length();
        len += self.max_fee_per_gas.length();
        len += self.gas_limit.length();
        len += self.kind.length();
        len += self.value.length();
        len += self.input.0.length();
        len += self.access_list.length();
        len
    }

    /// Encodes only the transaction's fields into the desired buffer, without a RLP header.
    pub fn encode_fields(&self, out: &mut dyn bytes::BufMut) {
        self.chain_id.encode(out);
        self.nonce.encode(out);
        self.max_priority_fee_per_gas.encode(out);
        self.max_fee_per_gas.encode(out);
        self.gas_limit.encode(out);
        self.kind.encode(out);
        self.value.encode(out);
        self.input.0.encode(out);
        self.access_list.encode(out);
    }

    /// Inner encoding function that is used for both rlp [`Encodable`] trait and for calculating
    /// hash that for eip2718 does not require rlp header
    pub fn encode_with_signature(
        &self,
        signature: &Signature,
        out: &mut dyn bytes::BufMut,
        with_header: bool,
    ) {
        let payload_length = self.fields_len() + signature.payload_len();
        if with_header {
            Header {
                list: false,
                payload_length: 1 + length_of_length(payload_length) + payload_length,
            }
            .encode(out);
        }
        out.put_u8(self.tx_type() as u8);
        let header = Header { list: true, payload_length };
        header.encode(out);
        self.encode_fields(out);
        signature.encode(out);
    }

    /// Output the length of the RLP signed transaction encoding, _without_ a RLP string header.
    pub fn payload_len_with_signature_without_header(&self, signature: &Signature) -> usize {
        let payload_length = self.fields_len() + signature.payload_len();
        // 'transaction type byte length' + 'header length' + 'payload length'
        1 + length_of_length(payload_length) + payload_length
    }

    /// Output the length of the RLP signed transaction encoding. This encodes with a RLP header.
    pub fn payload_len_with_signature(&self, signature: &Signature) -> usize {
        let len = self.payload_len_with_signature_without_header(signature);
        length_of_length(len) + len
    }

    /// Get transaction type
    pub fn tx_type(&self) -> TxType {
        TxType::EIP1559
    }

    /// Calculates a heuristic for the in-memory size of the transaction.
    #[inline]
    pub fn size(&self) -> usize {
        mem::size_of::<u64>() + // chain_id
        mem::size_of::<u64>() + // nonce
        mem::size_of::<u64>() + // gas_limit
        mem::size_of::<u128>() + // max_fee_per_gas
        mem::size_of::<u128>() + // max_priority_fee_per_gas
        self.kind.size() + // to
        mem::size_of::<U256>() + // value
        self.access_list.size() + // access_list
        self.input.len() // input
    }

    /// Encodes the legacy transaction in RLP for signing.
    pub fn encode_for_signing(&self, out: &mut dyn bytes::BufMut) {
        out.put_u8(self.tx_type() as u8);
        Header { list: true, payload_length: self.fields_len() }.encode(out);
        self.encode_fields(out);
    }

    /// Outputs the length of the signature RLP encoding for the transaction.
    pub fn payload_len_for_signature(&self) -> usize {
        let payload_length = self.fields_len();
        // 'transaction type byte length' + 'header length' + 'payload length'
        1 + length_of_length(payload_length) + payload_length
    }

    /// Outputs the signature hash of the transaction by first encoding without a signature, then
    /// hashing.
    pub fn signature_hash(&self) -> B256 {
        let mut buf = bytes::BytesMut::with_capacity(self.payload_len_for_signature());
        self.encode_for_signing(&mut buf);
        keccak256(&buf)
    }
}

/// Represents the `to` field of a transaction request
///
/// This determines what kind of transaction this is
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransactionKind {
    /// Transaction will call this address or transfer funds to this address
    Call(Address),
    /// No `to` field set, this transaction will create a contract
    Create,
}

// == impl TransactionKind ==

impl TransactionKind {
    /// If this transaction is a call this returns the address of the callee
    pub fn as_call(&self) -> Option<&Address> {
        match self {
            TransactionKind::Call(to) => Some(to),
            TransactionKind::Create => None,
        }
    }

    /// Calculates a heuristic for the in-memory size of the [TransactionKind].
    #[inline]
    pub fn size(&self) -> usize {
        mem::size_of::<Self>()
    }
}

impl Encodable for TransactionKind {
    fn encode(&self, out: &mut dyn BufMut) {
        match self {
            TransactionKind::Call(to) => to.encode(out),
            TransactionKind::Create => ([]).encode(out),
        }
    }
    fn length(&self) -> usize {
        match self {
            TransactionKind::Call(to) => to.length(),
            TransactionKind::Create => ([]).length(),
        }
    }
}

impl Decodable for TransactionKind {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        if let Some(&first) = buf.first() {
            if first == 0x80 {
                *buf = &buf[1..];
                Ok(TransactionKind::Create)
            } else {
                let addr = <Address as Decodable>::decode(buf)?;
                Ok(TransactionKind::Call(addr))
            }
        } else {
            Err(RlpError::InputTooShort)
        }
    }
}
