use super::{RlpEcdsaDecodableTx, RlpEcdsaEncodableTx};
use crate::{SignableTransaction, Transaction, TxType};
use alloc::vec::Vec;
use alloy_eips::{
    eip2718::IsTyped2718,
    eip2930::AccessList,
    eip7702::{constants::EIP7702_TX_TYPE_ID, SignedAuthorization},
    Typed2718,
};
use alloy_primitives::{Address, Bytes, ChainId, Signature, TxKind, B256, U256};
use alloy_rlp::{BufMut, Decodable, Encodable};

/// A transaction with a priority fee ([EIP-7702](https://eips.ethereum.org/EIPS/eip-7702)).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "borsh", derive(borsh::BorshSerialize, borsh::BorshDeserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[doc(alias = "Eip7702Transaction", alias = "TransactionEip7702", alias = "Eip7702Tx")]
pub struct TxEip7702 {
    /// EIP-155: Simple replay attack protection
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub chain_id: ChainId,
    /// A scalar value equal to the number of transactions sent by the sender; formally Tn.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub nonce: u64,
    /// A scalar value equal to the maximum
    /// amount of gas that should be used in executing
    /// this transaction. This is paid up-front, before any
    /// computation is done and may not be increased
    /// later; formally Tg.
    #[cfg_attr(
        feature = "serde",
        serde(with = "alloy_serde::quantity", rename = "gas", alias = "gasLimit")
    )]
    pub gas_limit: u64,
    /// A scalar value equal to the maximum total fee per unit of gas
    /// the sender is willing to pay. The actual fee paid per gas is
    /// the minimum of this and `base_fee + max_priority_fee_per_gas`.
    ///
    /// As ethereum circulation is around 120mil eth as of 2022 that is around
    /// 120000000000000000000000000 wei we are safe to use u128 as its max number is:
    /// 340282366920938463463374607431768211455
    ///
    /// This is also known as `GasFeeCap`
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
    pub max_fee_per_gas: u128,
    /// Max Priority fee that transaction is paying
    ///
    /// As ethereum circulation is around 120mil eth as of 2022 that is around
    /// 120000000000000000000000000 wei we are safe to use u128 as its max number is:
    /// 340282366920938463463374607431768211455
    ///
    /// This is also known as `GasTipCap`
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::quantity"))]
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
    /// Authorizations are used to temporarily set the code of its signer to
    /// the code referenced by `address`. These also include a `chain_id` (which
    /// can be set to zero and not evaluated) as well as an optional `nonce`.
    pub authorization_list: Vec<SignedAuthorization>,
    /// An unlimited size byte array specifying the
    /// input data of the message call, formally Td.
    pub input: Bytes,
}

impl TxEip7702 {
    /// Get the transaction type.
    #[doc(alias = "transaction_type")]
    pub const fn tx_type() -> TxType {
        TxType::Eip7702
    }

    /// Calculates a heuristic for the in-memory size of the [TxEip7702] transaction.
    #[inline]
    pub fn size(&self) -> usize {
        size_of::<Self>()
            + self.access_list.size()
            + self.input.len()
            + self.authorization_list.capacity() * size_of::<SignedAuthorization>()
    }
}

impl RlpEcdsaEncodableTx for TxEip7702 {
    /// Outputs the length of the transaction's fields, without a RLP header.
    #[doc(hidden)]
    fn rlp_encoded_fields_length(&self) -> usize {
        self.chain_id.length()
            + self.nonce.length()
            + self.max_priority_fee_per_gas.length()
            + self.max_fee_per_gas.length()
            + self.gas_limit.length()
            + self.to.length()
            + self.value.length()
            + self.input.0.length()
            + self.access_list.length()
            + self.authorization_list.length()
    }

    fn rlp_encode_fields(&self, out: &mut dyn alloy_rlp::BufMut) {
        self.chain_id.encode(out);
        self.nonce.encode(out);
        self.max_priority_fee_per_gas.encode(out);
        self.max_fee_per_gas.encode(out);
        self.gas_limit.encode(out);
        self.to.encode(out);
        self.value.encode(out);
        self.input.0.encode(out);
        self.access_list.encode(out);
        self.authorization_list.encode(out);
    }
}

impl RlpEcdsaDecodableTx for TxEip7702 {
    const DEFAULT_TX_TYPE: u8 = { Self::tx_type() as u8 };

    fn rlp_decode_fields(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
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
            authorization_list: Decodable::decode(buf)?,
        })
    }
}

impl Transaction for TxEip7702 {
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
        self.gas_limit
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
        None
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
        self.to.into()
    }

    #[inline]
    fn is_create(&self) -> bool {
        false
    }

    #[inline]
    fn value(&self) -> U256 {
        self.value
    }

    #[inline]
    fn input(&self) -> &Bytes {
        &self.input
    }

    #[inline]
    fn access_list(&self) -> Option<&AccessList> {
        Some(&self.access_list)
    }

    #[inline]
    fn blob_versioned_hashes(&self) -> Option<&[B256]> {
        None
    }

    #[inline]
    fn authorization_list(&self) -> Option<&[SignedAuthorization]> {
        Some(&self.authorization_list)
    }
}

impl SignableTransaction<Signature> for TxEip7702 {
    fn set_chain_id(&mut self, chain_id: ChainId) {
        self.chain_id = chain_id;
    }

    fn encode_for_signing(&self, out: &mut dyn alloy_rlp::BufMut) {
        out.put_u8(EIP7702_TX_TYPE_ID);
        self.encode(out)
    }

    fn payload_len_for_signature(&self) -> usize {
        self.length() + 1
    }
}

impl Typed2718 for TxEip7702 {
    fn ty(&self) -> u8 {
        TxType::Eip7702 as u8
    }
}

impl IsTyped2718 for TxEip7702 {
    fn is_type(type_id: u8) -> bool {
        matches!(type_id, 0x04)
    }
}

impl Encodable for TxEip7702 {
    fn encode(&self, out: &mut dyn BufMut) {
        self.rlp_encode(out);
    }

    fn length(&self) -> usize {
        self.rlp_encoded_length()
    }
}

impl Decodable for TxEip7702 {
    fn decode(data: &mut &[u8]) -> alloy_rlp::Result<Self> {
        Self::rlp_decode(data)
    }
}

/// Bincode-compatible [`TxEip7702`] serde implementation.
#[cfg(all(feature = "serde", feature = "serde-bincode-compat"))]
pub(super) mod serde_bincode_compat {
    use alloc::{borrow::Cow, vec::Vec};
    use alloy_eips::{eip2930::AccessList, eip7702::serde_bincode_compat::SignedAuthorization};
    use alloy_primitives::{Address, Bytes, ChainId, U256};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde_with::{DeserializeAs, SerializeAs};

    /// Bincode-compatible [`super::TxEip7702`] serde implementation.
    ///
    /// Intended to use with the [`serde_with::serde_as`] macro in the following way:
    /// ```rust
    /// use alloy_consensus::{serde_bincode_compat, TxEip7702};
    /// use serde::{Deserialize, Serialize};
    /// use serde_with::serde_as;
    ///
    /// #[serde_as]
    /// #[derive(Serialize, Deserialize)]
    /// struct Data {
    ///     #[serde_as(as = "serde_bincode_compat::transaction::TxEip7702")]
    ///     transaction: TxEip7702,
    /// }
    /// ```
    #[derive(Debug, Serialize, Deserialize)]
    pub struct TxEip7702<'a> {
        chain_id: ChainId,
        nonce: u64,
        gas_limit: u64,
        max_fee_per_gas: u128,
        max_priority_fee_per_gas: u128,
        to: Address,
        value: U256,
        access_list: Cow<'a, AccessList>,
        authorization_list: Vec<SignedAuthorization<'a>>,
        input: Cow<'a, Bytes>,
    }

    impl<'a> From<&'a super::TxEip7702> for TxEip7702<'a> {
        fn from(value: &'a super::TxEip7702) -> Self {
            Self {
                chain_id: value.chain_id,
                nonce: value.nonce,
                gas_limit: value.gas_limit,
                max_fee_per_gas: value.max_fee_per_gas,
                max_priority_fee_per_gas: value.max_priority_fee_per_gas,
                to: value.to,
                value: value.value,
                access_list: Cow::Borrowed(&value.access_list),
                authorization_list: value.authorization_list.iter().map(Into::into).collect(),
                input: Cow::Borrowed(&value.input),
            }
        }
    }

    impl<'a> From<TxEip7702<'a>> for super::TxEip7702 {
        fn from(value: TxEip7702<'a>) -> Self {
            Self {
                chain_id: value.chain_id,
                nonce: value.nonce,
                gas_limit: value.gas_limit,
                max_fee_per_gas: value.max_fee_per_gas,
                max_priority_fee_per_gas: value.max_priority_fee_per_gas,
                to: value.to,
                value: value.value,
                access_list: value.access_list.into_owned(),
                authorization_list: value.authorization_list.into_iter().map(Into::into).collect(),
                input: value.input.into_owned(),
            }
        }
    }

    impl SerializeAs<super::TxEip7702> for TxEip7702<'_> {
        fn serialize_as<S>(source: &super::TxEip7702, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            TxEip7702::from(source).serialize(serializer)
        }
    }

    impl<'de> DeserializeAs<'de, super::TxEip7702> for TxEip7702<'de> {
        fn deserialize_as<D>(deserializer: D) -> Result<super::TxEip7702, D::Error>
        where
            D: Deserializer<'de>,
        {
            TxEip7702::deserialize(deserializer).map(Into::into)
        }
    }

    #[cfg(test)]
    mod tests {
        use arbitrary::Arbitrary;
        use bincode::config;
        use rand::Rng;
        use serde::{Deserialize, Serialize};
        use serde_with::serde_as;

        use super::super::{serde_bincode_compat, TxEip7702};

        #[test]
        fn test_tx_eip7702_bincode_roundtrip() {
            #[serde_as]
            #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
            struct Data {
                #[serde_as(as = "serde_bincode_compat::TxEip7702")]
                transaction: TxEip7702,
            }

            let mut bytes = [0u8; 1024];
            rand::thread_rng().fill(bytes.as_mut_slice());
            let data = Data {
                transaction: TxEip7702::arbitrary(&mut arbitrary::Unstructured::new(&bytes))
                    .unwrap(),
            };

            let encoded = bincode::serde::encode_to_vec(&data, config::legacy()).unwrap();
            let (decoded, _) =
                bincode::serde::decode_from_slice::<Data, _>(&encoded, config::legacy()).unwrap();
            assert_eq!(decoded, data);
        }
    }
}

#[cfg(all(test, feature = "k256"))]
mod tests {
    use super::*;
    use crate::SignableTransaction;
    use alloy_eips::eip2930::AccessList;
    use alloy_primitives::{address, b256, hex, Address, Signature, U256};

    #[test]
    fn encode_decode_eip7702() {
        let tx =  TxEip7702 {
            chain_id: 1,
            nonce: 0x42,
            gas_limit: 44386,
            to: address!("6069a6c32cf691f5982febae4faf8a6f3ab2f0f6"),
            value: U256::from(0_u64),
            input:  hex!("a22cb4650000000000000000000000005eee75727d804a2b13038928d36f8b188945a57a0000000000000000000000000000000000000000000000000000000000000000").into(),
            max_fee_per_gas: 0x4a817c800,
            max_priority_fee_per_gas: 0x3b9aca00,
            access_list: AccessList::default(),
            authorization_list: vec![],
        };

        let sig = Signature::from_scalars_and_parity(
            b256!("840cfc572845f5786e702984c2a582528cad4b49b2a10b9db1be7fca90058565"),
            b256!("25e7109ceb98168d95b09b18bbf6b685130e0562f233877d492b94eee0c5b6d1"),
            false,
        );

        let mut buf = vec![];
        tx.rlp_encode_signed(&sig, &mut buf);
        let decoded = TxEip7702::rlp_decode_signed(&mut &buf[..]).unwrap();
        assert_eq!(decoded, tx.into_signed(sig));
    }

    #[test]
    fn test_decode_create() {
        // tests that a contract creation tx encodes and decodes properly
        let tx = TxEip7702 {
            chain_id: 1u64,
            nonce: 0,
            max_fee_per_gas: 0x4a817c800,
            max_priority_fee_per_gas: 0x3b9aca00,
            gas_limit: 2,
            to: Address::default(),
            value: U256::ZERO,
            input: vec![1, 2].into(),
            access_list: Default::default(),
            authorization_list: Default::default(),
        };
        let sig = Signature::from_scalars_and_parity(
            b256!("840cfc572845f5786e702984c2a582528cad4b49b2a10b9db1be7fca90058565"),
            b256!("25e7109ceb98168d95b09b18bbf6b685130e0562f233877d492b94eee0c5b6d1"),
            false,
        );
        let mut buf = vec![];
        tx.rlp_encode_signed(&sig, &mut buf);
        let decoded = TxEip7702::rlp_decode_signed(&mut &buf[..]).unwrap();
        assert_eq!(decoded, tx.into_signed(sig));
    }

    /// Demonstrates that a TxEip7702 with many zero-valued SignedAuthorization items
    /// can have an in-memory size ~4x its RLP-encoded size.
    ///
    /// Each SignedAuthorization is ~160 bytes in memory (due to U256 fields stored as
    /// 4×u64 limbs) but only ~27 bytes in RLP when all U256 fields are zero (each
    /// zero U256 encodes as a single 0x80 byte). The only non-compressible field is
    /// `address` (always 21 bytes on wire).
    #[test]
    fn test_rlp_vs_in_memory_size_amplification() {
        use alloy_eips::eip7702::Authorization;
        use alloy_rlp::Encodable;

        // Build a SignedAuthorization with all zeros (except address, which is fixed-size)
        let zero_auth = SignedAuthorization::new_unchecked(
            Authorization { chain_id: U256::ZERO, address: Address::ZERO, nonce: 0 },
            0,          // y_parity
            U256::ZERO, // r
            U256::ZERO, // s
        );

        // Measure a single item
        let single_rlp_len = zero_auth.length();
        let single_mem_size = core::mem::size_of::<SignedAuthorization>();
        eprintln!(
            "Single SignedAuthorization: RLP = {} bytes, mem = {} bytes, ratio = {:.1}x",
            single_rlp_len,
            single_mem_size,
            single_mem_size as f64 / single_rlp_len as f64,
        );

        // Target ~10 MiB RLP. We need to account for the RLP list header overhead
        // for the authorization_list vec + the outer TxEip7702 envelope.
        let target_rlp_bytes = 10 * 1024 * 1024; // 10 MiB
        let num_auths = target_rlp_bytes / single_rlp_len;

        let tx = TxEip7702 {
            chain_id: 0,
            nonce: 0,
            max_fee_per_gas: 0,
            max_priority_fee_per_gas: 0,
            gas_limit: 0,
            to: Address::ZERO,
            value: U256::ZERO,
            input: Default::default(),
            access_list: Default::default(),
            authorization_list: vec![zero_auth; num_auths],
        };

        let rlp_len = tx.length();
        let mem_size = tx.size();
        let ratio = mem_size as f64 / rlp_len as f64;

        eprintln!("num authorization items: {num_auths}");
        eprintln!("RLP encoded size:  {} bytes ({:.2} MiB)", rlp_len, rlp_len as f64 / (1024.0 * 1024.0));
        eprintln!("InMemorySize:      {} bytes ({:.2} MiB)", mem_size, mem_size as f64 / (1024.0 * 1024.0));
        eprintln!("Amplification:     {ratio:.2}x");

        // Assert we got close to 10 MiB RLP
        assert!(rlp_len >= 9 * 1024 * 1024, "RLP size should be ~10 MiB, got {rlp_len}");
        assert!(rlp_len <= 11 * 1024 * 1024, "RLP size should be ~10 MiB, got {rlp_len}");

        // Assert the memory amplification is >= 4x
        assert!(ratio >= 4.0, "Expected >=4x amplification, got {ratio:.2}x");
    }

    /// Explores the worst-case memory amplification for a *list* of signed
    /// transactions across all `EthereumTxEnvelope` variants, using both
    /// zero signatures (theoretical max) and valid/realistic signatures.
    ///
    /// A valid ECDSA signature has r,s as full 32-byte scalars (~33 bytes each
    /// in RLP with length prefix), whereas zero r,s encode as 1 byte each.
    /// This test quantifies the difference.
    #[test]
    fn test_list_of_signed_txs_amplification() {
        use crate::{InMemorySize, Signed, TxEip1559, TxEip2930, TxLegacy};
        use alloy_eips::eip7702::Authorization;
        use alloy_primitives::{Signature, TxKind};

        // A realistic signature from an existing test vector (32-byte r and s)
        let real_sig = Signature::from_scalars_and_parity(
            b256!("840cfc572845f5786e702984c2a582528cad4b49b2a10b9db1be7fca90058565"),
            b256!("25e7109ceb98168d95b09b18bbf6b685130e0562f233877d492b94eee0c5b6d1"),
            false,
        );
        // Realistic auth-list signature (32-byte r and s)
        let real_auth_sig_r =
            U256::from_be_bytes(b256!("840cfc572845f5786e702984c2a582528cad4b49b2a10b9db1be7fca90058565").0);
        let real_auth_sig_s =
            U256::from_be_bytes(b256!("25e7109ceb98168d95b09b18bbf6b685130e0562f233877d492b94eee0c5b6d1").0);

        let target_rlp = 10 * 1024 * 1024usize;

        fn measure<T: super::RlpEcdsaEncodableTx + InMemorySize>(
            name: &str,
            signed: &Signed<T>,
            target_rlp: usize,
        ) -> (String, f64) {
            let one_rlp = signed.eip2718_encoded_length();
            let one_mem = signed.size();
            let num_txs = core::cmp::max(1, target_rlp / one_rlp);
            let total_rlp = num_txs * one_rlp;
            let total_mem = num_txs * one_mem;
            let ratio = total_mem as f64 / total_rlp as f64;
            eprintln!(
                "{name:30}: per-tx RLP={one_rlp:4}, mem={one_mem:4} | \
                 {num_txs:>7} txs => RLP {:.2} MiB, mem {:.2} MiB, ratio {ratio:.2}x",
                total_rlp as f64 / (1024.0 * 1024.0),
                total_mem as f64 / (1024.0 * 1024.0),
            );
            (name.to_string(), ratio)
        }

        let mut results = Vec::new();

        // Legacy
        {
            let tx = TxLegacy {
                chain_id: None,
                nonce: 0,
                gas_price: 0,
                gas_limit: 0,
                to: TxKind::Call(Address::ZERO),
                value: U256::ZERO,
                input: Default::default(),
            };
            let signed = Signed::new_unhashed(tx, real_sig);
            results.push(measure("Legacy (real sig)", &signed, target_rlp));
        }

        // EIP-2930
        {
            let tx = TxEip2930 {
                chain_id: 0,
                nonce: 0,
                gas_price: 0,
                gas_limit: 0,
                to: TxKind::Call(Address::ZERO),
                value: U256::ZERO,
                access_list: Default::default(),
                input: Default::default(),
            };
            let signed = Signed::new_unhashed(tx, real_sig);
            results.push(measure("EIP-2930 (real sig)", &signed, target_rlp));
        }

        // EIP-1559
        {
            let tx = TxEip1559 {
                chain_id: 0,
                nonce: 0,
                max_fee_per_gas: 0,
                max_priority_fee_per_gas: 0,
                gas_limit: 0,
                to: TxKind::Call(Address::ZERO),
                value: U256::ZERO,
                access_list: Default::default(),
                input: Default::default(),
            };
            let signed = Signed::new_unhashed(tx, real_sig);
            results.push(measure("EIP-1559 (real sig)", &signed, target_rlp));
        }

        // EIP-7702 (0 auth)
        {
            let tx = TxEip7702 {
                chain_id: 0,
                nonce: 0,
                max_fee_per_gas: 0,
                max_priority_fee_per_gas: 0,
                gas_limit: 0,
                to: Address::ZERO,
                value: U256::ZERO,
                access_list: Default::default(),
                authorization_list: vec![],
                input: Default::default(),
            };
            let signed = Signed::new_unhashed(tx, real_sig);
            results.push(measure("EIP-7702 0auth (real sig)", &signed, target_rlp));
        }

        // EIP-7702 with 1 real-sig auth item
        {
            let auth = SignedAuthorization::new_unchecked(
                Authorization { chain_id: U256::ZERO, address: Address::ZERO, nonce: 0 },
                0,
                real_auth_sig_r,
                real_auth_sig_s,
            );
            let tx = TxEip7702 {
                chain_id: 0,
                nonce: 0,
                max_fee_per_gas: 0,
                max_priority_fee_per_gas: 0,
                gas_limit: 0,
                to: Address::ZERO,
                value: U256::ZERO,
                access_list: Default::default(),
                authorization_list: vec![auth],
                input: Default::default(),
            };
            let signed = Signed::new_unhashed(tx, real_sig);
            results.push(measure("EIP-7702 1auth (real sig)", &signed, target_rlp));
        }

        // EIP-7702 many zero-field auth items with real sigs
        {
            let auth = SignedAuthorization::new_unchecked(
                Authorization { chain_id: U256::ZERO, address: Address::ZERO, nonce: 0 },
                0,
                real_auth_sig_r,
                real_auth_sig_s,
            );
            let auth_rlp_len = alloy_rlp::Encodable::length(&auth);
            let tx = TxEip7702 {
                chain_id: 0,
                nonce: 0,
                max_fee_per_gas: 0,
                max_priority_fee_per_gas: 0,
                gas_limit: 0,
                to: Address::ZERO,
                value: U256::ZERO,
                access_list: Default::default(),
                authorization_list: vec![auth; target_rlp / auth_rlp_len],
                input: Default::default(),
            };
            let signed = Signed::new_unhashed(tx, real_sig);
            results.push(measure("EIP-7702 big (real sigs)", &signed, target_rlp));
        }

        eprintln!();
        let (best_name, best_ratio) = results
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();
        eprintln!("Worst case (valid sig): {best_name} at {best_ratio:.2}x");
    }

    #[test]
    fn test_decode_call() {
        let tx = TxEip7702 {
            chain_id: 1u64,
            nonce: 0,
            max_fee_per_gas: 0x4a817c800,
            max_priority_fee_per_gas: 0x3b9aca00,
            gas_limit: 2,
            to: Address::default(),
            value: U256::ZERO,
            input: vec![1, 2].into(),
            access_list: Default::default(),
            authorization_list: Default::default(),
        };

        let sig = Signature::from_scalars_and_parity(
            b256!("840cfc572845f5786e702984c2a582528cad4b49b2a10b9db1be7fca90058565"),
            b256!("25e7109ceb98168d95b09b18bbf6b685130e0562f233877d492b94eee0c5b6d1"),
            false,
        );

        let mut buf = vec![];
        tx.rlp_encode_signed(&sig, &mut buf);
        let decoded = TxEip7702::rlp_decode_signed(&mut &buf[..]).unwrap();
        assert_eq!(decoded, tx.into_signed(sig));
    }
}
