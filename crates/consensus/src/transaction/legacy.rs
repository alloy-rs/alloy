use crate::{SignableTransaction, Signed, Transaction};
use alloy_primitives::{keccak256, Bytes, ChainId, Signature, TxKind, U256};
use alloy_rlp::{length_of_length, BufMut, Decodable, Encodable, Header, Result};
use std::mem;

/// Legacy transaction.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TxLegacy {
    /// Added as EIP-155: Simple replay attack protection
    pub chain_id: Option<ChainId>,
    /// A scalar value equal to the number of transactions sent by the sender; formally Tn.
    pub nonce: u64,
    /// A scalar value equal to the number of
    /// Wei to be paid per unit of gas for all computation
    /// costs incurred as a result of the execution of this transaction; formally Tp.
    ///
    /// As ethereum circulation is around 120mil eth as of 2022 that is around
    /// 120000000000000000000000000 wei we are safe to use u128 as its max number is:
    /// 340282366920938463463374607431768211455
    pub gas_price: u128,
    /// A scalar value equal to the maximum
    /// amount of gas that should be used in executing
    /// this transaction. This is paid up-front, before any
    /// computation is done and may not be increased
    /// later; formally Tg.
    pub gas_limit: u64,
    /// The 160-bit address of the message call’s recipient or, for a contract creation
    /// transaction, ∅, used here to denote the only member of B0 ; formally Tt.
    pub to: TxKind,
    /// A scalar value equal to the number of Wei to
    /// be transferred to the message call’s recipient or,
    /// in the case of contract creation, as an endowment
    /// to the newly created account; formally Tv.
    pub value: U256,
    /// Input has two uses depending if transaction is Create or Call (if `to` field is None or
    /// Some). pub init: An unlimited size byte array specifying the
    /// EVM-code for the account initialisation procedure CREATE,
    /// data: An unlimited size byte array specifying the
    /// input data of the message call, formally Td.
    pub input: Bytes,
}

impl TxLegacy {
    /// The EIP-2718 transaction type.
    pub const TX_TYPE: isize = 0;

    /// Calculates a heuristic for the in-memory size of the [TxLegacy] transaction.
    #[inline]
    pub fn size(&self) -> usize {
        mem::size_of::<Option<ChainId>>() + // chain_id
        mem::size_of::<u64>() + // nonce
        mem::size_of::<u128>() + // gas_price
        mem::size_of::<u64>() + // gas_limit
        self.to.size() + // to
        mem::size_of::<U256>() + // value
        self.input.len() // input
    }

    /// Outputs the length of the transaction's fields, without a RLP header or length of the
    /// eip155 fields.
    pub(crate) fn fields_len(&self) -> usize {
        let mut len = 0;
        len += self.nonce.length();
        len += self.gas_price.length();
        len += self.gas_limit.length();
        len += self.to.length();
        len += self.value.length();
        len += self.input.0.length();
        len
    }

    /// Encodes only the transaction's fields into the desired buffer, without a RLP header or
    /// eip155 fields.
    pub(crate) fn encode_fields(&self, out: &mut dyn BufMut) {
        self.nonce.encode(out);
        self.gas_price.encode(out);
        self.gas_limit.encode(out);
        self.to.encode(out);
        self.value.encode(out);
        self.input.0.encode(out);
    }

    /// Inner encoding function that is used for both rlp [`Encodable`] trait and for calculating
    /// hash.
    pub fn encode_with_signature(&self, signature: &Signature, out: &mut dyn alloy_rlp::BufMut) {
        let payload_length = self.fields_len() + signature.rlp_vrs_len();
        let header = Header { list: true, payload_length };
        header.encode(out);
        self.encode_fields(out);
        signature.write_rlp_vrs(out);
    }

    /// Output the length of the RLP signed transaction encoding.
    pub fn payload_len_with_signature(&self, signature: &Signature) -> usize {
        let payload_length = self.fields_len() + signature.rlp_vrs_len();
        // 'header length' + 'payload length'
        length_of_length(payload_length) + payload_length
    }

    /// Encodes EIP-155 arguments into the desired buffer. Only encodes values
    /// for legacy transactions.
    pub(crate) fn encode_eip155_signing_fields(&self, out: &mut dyn BufMut) {
        // if this is a legacy transaction without a chain ID, it must be pre-EIP-155
        // and does not need to encode the chain ID for the signature hash encoding
        if let Some(id) = self.chain_id {
            // EIP-155 encodes the chain ID and two zeroes
            id.encode(out);
            0x00u8.encode(out);
            0x00u8.encode(out);
        }
    }

    /// Outputs the length of EIP-155 fields. Only outputs a non-zero value for EIP-155 legacy
    /// transactions.
    pub(crate) fn eip155_fields_len(&self) -> usize {
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

    /// Decode the RLP fields of the transaction, without decoding an RLP
    /// header.
    pub(crate) fn decode_fields(data: &mut &[u8]) -> Result<Self> {
        Ok(TxLegacy {
            nonce: Decodable::decode(data)?,
            gas_price: Decodable::decode(data)?,
            gas_limit: Decodable::decode(data)?,
            to: Decodable::decode(data)?,
            value: Decodable::decode(data)?,
            input: Decodable::decode(data)?,
            chain_id: None,
        })
    }
}

impl Transaction for TxLegacy {
    fn input(&self) -> &[u8] {
        &self.input
    }

    fn to(&self) -> TxKind {
        self.to
    }

    fn value(&self) -> U256 {
        self.value
    }

    fn chain_id(&self) -> Option<ChainId> {
        self.chain_id
    }

    fn nonce(&self) -> u64 {
        self.nonce
    }

    fn gas_limit(&self) -> u64 {
        self.gas_limit
    }

    fn gas_price(&self) -> Option<U256> {
        Some(U256::from(self.gas_price))
    }
}

impl SignableTransaction<Signature> for TxLegacy {
    fn set_chain_id(&mut self, chain_id: ChainId) {
        self.chain_id = Some(chain_id);
    }

    fn encode_for_signing(&self, out: &mut dyn BufMut) {
        Header { list: true, payload_length: self.fields_len() + self.eip155_fields_len() }
            .encode(out);
        self.encode_fields(out);
        self.encode_eip155_signing_fields(out);
    }

    fn payload_len_for_signature(&self) -> usize {
        let payload_length = self.fields_len() + self.eip155_fields_len();
        // 'header length' + 'payload length'
        length_of_length(payload_length) + payload_length
    }

    fn into_signed(self, signature: Signature) -> Signed<Self> {
        let payload_length = self.fields_len() + signature.rlp_vrs_len();
        let mut buf = Vec::with_capacity(payload_length);
        self.encode_with_signature(&signature, &mut buf);
        let hash = keccak256(&buf);
        Signed::new_unchecked(self, signature, hash)
    }

    fn encode_signed(&self, signature: &Signature, out: &mut dyn BufMut) {
        self.encode_with_signature(signature, out);
    }

    fn decode_signed(buf: &mut &[u8]) -> alloy_rlp::Result<Signed<Self>> {
        let header = Header::decode(buf)?;
        if !header.list {
            return Err(alloy_rlp::Error::UnexpectedString);
        }
        let mut tx = Self::decode_fields(buf)?;

        let signature = Signature::decode_rlp_vrs(buf)?;

        let v = signature.v();

        tx.chain_id = v.chain_id();

        Ok(tx.into_signed(signature))
    }
}

impl Encodable for TxLegacy {
    fn encode(&self, out: &mut dyn BufMut) {
        self.encode_for_signing(out)
    }

    fn length(&self) -> usize {
        let payload_length = self.fields_len() + self.eip155_fields_len();
        // 'header length' + 'payload length'
        length_of_length(payload_length) + payload_length
    }
}

impl Decodable for TxLegacy {
    fn decode(data: &mut &[u8]) -> Result<Self> {
        let header = Header::decode(data)?;
        let remaining_len = data.len();

        let transaction_payload_len = header.payload_length;

        if transaction_payload_len > remaining_len {
            return Err(alloy_rlp::Error::InputTooShort);
        }

        let mut transaction = Self::decode_fields(data)?;

        // If we still have data, it should be an eip-155 encoded chain_id
        if !data.is_empty() {
            transaction.chain_id = Some(Decodable::decode(data)?);
            let _: U256 = Decodable::decode(data)?; // r
            let _: U256 = Decodable::decode(data)?; // s
        }

        let decoded = remaining_len - data.len();
        if decoded != transaction_payload_len {
            return Err(alloy_rlp::Error::UnexpectedLength);
        }

        Ok(transaction)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "k256")]
    fn recover_signer_legacy() {
        use crate::{TxKind, TxLegacy};
        use alloy_network::SignableTransaction;
        use alloy_primitives::{address, b256, hex, Signature, U256};

        let signer = address!("398137383b3d25c92898c656696e41950e47316b");
        let hash = b256!("bb3a336e3f823ec18197f1e13ee875700f08f03e2cab75f0d0b118dabb44cba0");

        let tx = TxLegacy {
            chain_id: Some(1),
            nonce: 0x18,
            gas_price: 0xfa56ea00,
            gas_limit: 119902,
            to: TxKind::Call( hex!("06012c8cf97bead5deae237070f9587f8e7a266d").into()),
            value: U256::from(0x1c6bf526340000u64),
            input:  hex!("f7d8c88300000000000000000000000000000000000000000000000000000000000cee6100000000000000000000000000000000000000000000000000000000000ac3e1").into(),
        };

        let sig = Signature::from_scalars_and_parity(
            b256!("2a378831cf81d99a3f06a18ae1b6ca366817ab4d88a70053c41d7a8f0368e031"),
            b256!("450d831a05b6e418724436c05c155e0a1b7b921015d0fbc2f667aed709ac4fb5"),
            37,
        )
        .unwrap();

        let signed_tx = tx.into_signed(sig);

        assert_eq!(*signed_tx.hash(), hash, "Expected same hash");
        assert_eq!(signed_tx.recover_signer().unwrap(), signer, "Recovering signer should pass.");
    }

    #[test]
    #[cfg(feature = "k256")]
    // Test vector from https://github.com/alloy-rs/alloy/issues/125
    fn decode_legacy_and_recover_signer() {
        use crate::TxLegacy;
        use alloy_network::Signed;
        use alloy_primitives::address;
        use alloy_rlp::Decodable;

        let raw_tx = "f9015482078b8505d21dba0083022ef1947a250d5630b4cf539739df2c5dacb4c659f2488d880c46549a521b13d8b8e47ff36ab50000000000000000000000000000000000000000000066ab5a608bd00a23f2fe000000000000000000000000000000000000000000000000000000000000008000000000000000000000000048c04ed5691981c42154c6167398f95e8f38a7ff00000000000000000000000000000000000000000000000000000000632ceac70000000000000000000000000000000000000000000000000000000000000002000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc20000000000000000000000006c6ee5e31d828de241282b9606c8e98ea48526e225a0c9077369501641a92ef7399ff81c21639ed4fd8fc69cb793cfa1dbfab342e10aa0615facb2f1bcf3274a354cfe384a38d0cc008a11c2dd23a69111bc6930ba27a8";

        let tx = <Signed<TxLegacy> as Decodable>::decode(
            &mut alloy_primitives::hex::decode(raw_tx).unwrap().as_slice(),
        )
        .unwrap();

        let recovered = tx.recover_signer().unwrap();
        let expected = address!("a12e1462d0ceD572f396F58B6E2D03894cD7C8a4");

        assert_eq!(tx.chain_id, Some(1), "Expected same chain id");
        assert_eq!(expected, recovered, "Expected same signer");
    }
}
