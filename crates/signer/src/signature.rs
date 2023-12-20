use crate::utils::{public_key_to_address, to_eip155_v};
use alloy_primitives::{eip191_hash_message, hex, Address, B256, U256};
use alloy_rlp::{self, Decodable, Encodable};
use elliptic_curve::NonZeroScalar;
use k256::{
    ecdsa::{self, RecoveryId, VerifyingKey},
    Secp256k1,
};
use std::str::FromStr;

/// An Ethereum ECDSA signature.
///
/// This is a wrapper around [`ecdsa::Signature`] and a [`RecoveryId`] to provide public key
/// recovery functionality.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Signature {
    /// The inner ECDSA signature.
    inner: ecdsa::Signature,
    /// The recovery ID.
    recid: RecoveryId,
}

impl<'a> TryFrom<&'a [u8]> for Signature {
    type Error = ecdsa::Error;

    /// Parses a raw signature which is expected to be 65 bytes long where
    /// the first 32 bytes is the `r` value, the second 32 bytes the `s` value
    /// and the final byte is the `v` value in 'Electrum' notation.
    fn try_from(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        if bytes.len() != 65 {
            return Err(ecdsa::Error::new());
        }
        Self::from_bytes(&bytes[..64], bytes[64] as u64)
    }
}

impl FromStr for Signature {
    type Err = ecdsa::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match hex::decode(s) {
            Ok(bytes) => Self::try_from(&bytes[..]),
            Err(e) => Err(ecdsa::Error::from_source(e)),
        }
    }
}

impl Decodable for Signature {
    /// Decodes the provided byte buffer into a Signature instance
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let v = u64::decode(buf)?;
        Self::from_scalars(U256::decode(buf)?.into(), U256::decode(buf)?.into(), v)
            .map_err(|_| alloy_rlp::Error::Custom("Signature decoding error"))
    }
}

impl Encodable for Signature {
    /// Encodes the Signature components (`v`, `r`, and `s`) into the provided `out` buffer
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        self.v().encode(out);
        B256::from_slice(&self.r().to_bytes()).encode(out);
        B256::from_slice(&self.s().to_bytes()).encode(out);
    }

    /// Computes the total length of the encoded Signature components (`v`, `r`, and `s`)
    fn length(&self) -> usize {
        self.v().length() + self.r().to_bytes().length() + self.s().to_bytes().length()
    }
}

impl From<&Signature> for [u8; 65] {
    #[inline]
    fn from(value: &Signature) -> [u8; 65] {
        value.as_bytes()
    }
}

impl From<Signature> for [u8; 65] {
    #[inline]
    fn from(value: Signature) -> [u8; 65] {
        value.as_bytes()
    }
}

impl From<&Signature> for Vec<u8> {
    #[inline]
    fn from(value: &Signature) -> Vec<u8> {
        value.as_bytes().to_vec()
    }
}

impl From<Signature> for Vec<u8> {
    #[inline]
    fn from(value: Signature) -> Vec<u8> {
        value.as_bytes().to_vec()
    }
}

impl Signature {
    /// Creates a new [`Signature`] from the given ECDSA signature and recovery ID.
    ///
    /// Normalizes the signature into "low S" form as described in
    /// [BIP 0062: Dealing with Malleability][1].
    ///
    /// [1]: https://github.com/bitcoin/bips/blob/master/bip-0062.mediawiki
    #[inline]
    pub fn new(inner: ecdsa::Signature, recid: RecoveryId) -> Self {
        let mut sig = Self::new_not_normalized(inner, recid);
        sig.normalize_s();
        sig
    }

    /// Creates a new signature from the given inner signature and recovery ID, without normalizing
    /// it into "low S" form.
    #[inline]
    pub const fn new_not_normalized(inner: ecdsa::Signature, recid: RecoveryId) -> Self {
        Self { inner, recid }
    }

    /// Normalizes the signature into "low S" form as described in
    /// [BIP 0062: Dealing with Malleability][1].
    ///
    /// [1]: https://github.com/bitcoin/bips/blob/master/bip-0062.mediawiki
    #[inline]
    pub fn normalize_s(&mut self) {
        // Normalize into "low S" form. See:
        // - https://github.com/RustCrypto/elliptic-curves/issues/988
        // - https://github.com/bluealloy/revm/pull/870
        if let Some(normalized) = self.inner.normalize_s() {
            self.inner = normalized;
            self.recid = RecoveryId::from_byte(self.recid.to_byte() ^ 1).unwrap();
        }
    }

    /// Parses a signature from a byte slice.
    #[inline]
    pub fn from_bytes(bytes: &[u8], v: u64) -> Result<Self, ecdsa::Error> {
        let inner = ecdsa::Signature::from_slice(bytes)?;
        let recid = normalize_v(v);
        Ok(Self::new(inner, recid))
    }

    /// Creates a [`Signature`] from the serialized `r` and `s` scalar values, which comprise the
    /// ECDSA signature, alongside a `v` value, used to determine the recovery ID.
    ///
    /// See [`ecdsa::Signature::from_scalars`] for more details.
    #[inline]
    pub fn from_scalars(r: B256, s: B256, v: u64) -> Result<Self, ecdsa::Error> {
        let inner = ecdsa::Signature::from_scalars(r.0, s.0)?;
        let recid = normalize_v(v);
        Ok(Self::new(inner, recid))
    }

    /// Returns the inner ECDSA signature.
    #[inline]
    pub const fn inner(&self) -> &ecdsa::Signature {
        &self.inner
    }

    /// Returns the inner ECDSA signature.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut ecdsa::Signature {
        &mut self.inner
    }

    /// Returns the inner ECDSA signature.
    #[inline]
    pub const fn into_inner(self) -> ecdsa::Signature {
        self.inner
    }

    /// Returns the recovery ID.
    #[inline]
    pub const fn recid(&self) -> RecoveryId {
        self.recid
    }

    #[doc(hidden)]
    #[deprecated(note = "use `Signature::recid` instead")]
    pub const fn recovery_id(&self) -> RecoveryId {
        self.recid
    }

    /// Returns the `r` component of this signature.
    #[inline]
    pub fn r(&self) -> NonZeroScalar<Secp256k1> {
        self.inner.r()
    }

    /// Returns the `s` component of this signature.
    #[inline]
    pub fn s(&self) -> NonZeroScalar<Secp256k1> {
        self.inner.s()
    }

    /// Returns the recovery ID as a `u8`.
    #[inline]
    pub const fn v(&self) -> u8 {
        self.recid.to_byte()
    }

    /// Returns the byte-array representation of this signature.
    ///
    /// The first 32 bytes are the `r` value, the second 32 bytes the `s` value
    /// and the final byte is the `v` value in 'Electrum' notation.
    #[inline]
    pub fn as_bytes(&self) -> [u8; 65] {
        let mut sig = [0u8; 65];
        sig[..32].copy_from_slice(self.r().to_bytes().as_ref());
        sig[32..64].copy_from_slice(self.s().to_bytes().as_ref());
        sig[64] = self.recid.to_byte();
        sig
    }

    /// Sets the recovery ID.
    #[inline]
    pub fn set_recid(&mut self, recid: RecoveryId) {
        self.recid = recid;
    }

    /// Sets the recovery ID by normalizing a `v` value.
    #[inline]
    pub fn set_v(&mut self, v: u64) {
        self.set_recid(normalize_v(v));
    }

    /// Modifies the recovery ID by applying [EIP-155] to a `v` value.
    ///
    /// [EIP-155]: https://eips.ethereum.org/EIPS/eip-155
    #[inline]
    pub fn apply_eip155(&mut self, chain_id: u64) {
        self.set_v(to_eip155_v(self.recid.to_byte(), chain_id));
    }

    /// Recovers an [`Address`] from this signature and the given message by first prefixing and
    /// hashing the message according to [EIP-191](eip191_hash_message).
    #[inline]
    pub fn recover_address_from_msg<T: AsRef<[u8]>>(
        &self,
        msg: T,
    ) -> Result<Address, ecdsa::Error> {
        self.recover_from_msg(msg).map(|pubkey| public_key_to_address(&pubkey))
    }

    /// Recovers an [`Address`] from this signature and the given prehashed message.
    #[inline]
    pub fn recover_address_from_prehash(&self, prehash: &B256) -> Result<Address, ecdsa::Error> {
        self.recover_from_prehash(prehash).map(|pubkey| public_key_to_address(&pubkey))
    }

    /// Recovers a [`VerifyingKey`] from this signature and the given message by first prefixing and
    /// hashing the message according to [EIP-191](eip191_hash_message).
    #[inline]
    pub fn recover_from_msg<T: AsRef<[u8]>>(&self, msg: T) -> Result<VerifyingKey, ecdsa::Error> {
        self.recover_from_prehash(&eip191_hash_message(msg))
    }

    /// Recovers a [`VerifyingKey`] from this signature and the given prehashed message.
    #[inline]
    pub fn recover_from_prehash(&self, prehash: &B256) -> Result<VerifyingKey, ecdsa::Error> {
        VerifyingKey::recover_from_prehash(prehash.as_slice(), &self.inner, self.recid)
    }
}

/// Normalizes a `v` value, respecting raw, legacy, and EIP-155 values.
///
/// This function covers the entire u64 range, producing v-values as follows:
/// - 0-26 - raw/bare. 0-3 are legal. In order to ensure that all values are covered, we also handle
///   4-26 here by returning v % 4.
/// - 27-34 - legacy. 27-30 are legal. By legacy bitcoin convention range 27-30 signals uncompressed
///   pubkeys, while 31-34 signals compressed pubkeys. We do not respect the compression convention.
///   All Ethereum keys are uncompressed.
/// - 35+ - EIP-155. By EIP-155 convention, `v = 35 + CHAIN_ID * 2 + 0/1` We return (v-1 % 2) here.
///
/// NB: raw and legacy support values 2, and 3, while EIP-155 does not.
/// Recovery values of 2 and 3 are unlikely to occur in practice. In the vanishingly unlikely event
/// that you encounter an EIP-155 signature with a recovery value of 2 or 3, you should normalize
/// out of band.
#[inline]
const fn normalize_v(v: u64) -> RecoveryId {
    let byte = match v {
        // Case 1: raw/bare
        0..=26 => (v % 4) as u8,
        // Case 2: non-EIP-155 v value
        27..=34 => ((v - 27) % 4) as u8,
        // Case 3: EIP-155 V value
        35.. => ((v - 1) % 2) as u8,
    };
    debug_assert!(byte <= RecoveryId::MAX);
    match RecoveryId::from_byte(byte) {
        Some(recid) => recid,
        None => unsafe { core::hint::unreachable_unchecked() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, b256};
    use std::str::FromStr;

    #[test]
    #[cfg(TODO)] // TODO: Transaction
    fn can_recover_tx_sender() {
        // random mainnet tx: https://etherscan.io/tx/0x86718885c4b4218c6af87d3d0b0d83e3cc465df2a05c048aa4db9f1a6f9de91f
        let tx_rlp = hex::decode("02f872018307910d808507204d2cb1827d0094388c818ca8b9251b393131c08a736a67ccb19297880320d04823e2701c80c001a0cf024f4815304df2867a1a74e9d2707b6abda0337d2d54a4438d453f4160f190a07ac0e6b3bc9395b5b9c8b9e6d77204a236577a5b18467b9175c01de4faa208d9").unwrap();
        let tx: Transaction = rlp::decode(&tx_rlp).unwrap();
        assert_eq!(tx.rlp(), tx_rlp);
        assert_eq!(
            tx.hash,
            "0x86718885c4b4218c6af87d3d0b0d83e3cc465df2a05c048aa4db9f1a6f9de91f".parse().unwrap()
        );
        assert_eq!(tx.transaction_type, Some(2.into()));
        let expected = Address::from_str("0x95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5").unwrap();
        assert_eq!(tx.recover_from().unwrap(), expected);
    }

    #[test]
    fn can_recover_tx_sender_not_normalized() {
        let sig = Signature::from_str("48b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353efffd310ac743f371de3b9f7f9cb56c0b28ad43601b4ab949f53faa07bd2c8041b").unwrap();
        let hash = b256!("5eb4f5a33c621f32a8622d5f943b6b102994dfe4e5aebbefe69bb1b2aa0fc93e");
        let expected = address!("0f65fe9276bc9a24ae7083ae28e2660ef72df99e");
        assert_eq!(sig.recover_address_from_prehash(&hash).unwrap(), expected);
    }

    #[test]
    fn recover_web3_signature() {
        // test vector taken from:
        // https://web3js.readthedocs.io/en/v1.2.2/web3-eth-accounts.html#sign
        let signature = Signature::from_str(
            "b91467e570a6466aa9e9876cbcd013baba02900b8979d43fe208a4a4f339f5fd6007e74cd82e037b800186422fc2da167c747ef045e5d18a5f5d4300f8e1a0291c"
        ).expect("could not parse signature");
        let expected = address!("2c7536E3605D9C16a7a3D7b1898e529396a65c23");
        assert_eq!(signature.recover_address_from_msg("Some data").unwrap(), expected);
    }

    #[test]
    fn signature_from_str() {
        let s1 = Signature::from_str(
            "0xaa231fbe0ed2b5418e6ba7c19bee2522852955ec50996c02a2fe3e71d30ddaf1645baf4823fea7cb4fcc7150842493847cfb6a6d63ab93e8ee928ee3f61f503500"
        ).expect("could not parse 0x-prefixed signature");

        let s2 = Signature::from_str(
            "aa231fbe0ed2b5418e6ba7c19bee2522852955ec50996c02a2fe3e71d30ddaf1645baf4823fea7cb4fcc7150842493847cfb6a6d63ab93e8ee928ee3f61f503500"
        ).expect("could not parse non-prefixed signature");

        assert_eq!(s1, s2);
    }

    #[test]
    fn signature_rlp_decode() {
        // Given a hex-encoded byte sequence
        let bytes = hex::decode("01a048b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353a010002cef538bc0c8e21c46080634a93e082408b0ad93f4a7207e63ec5463793d").unwrap();

        // Decode the byte sequence into a Signature instance
        let result = Signature::decode(&mut &bytes[..]).unwrap();

        // Assert that the decoded Signature matches the expected Signature
        assert_eq!(
        result,
        Signature::from_str("48b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353efffd310ac743f371de3b9f7f9cb56c0b28ad43601b4ab949f53faa07bd2c8041b").unwrap()
    );
    }

    #[test]
    fn signature_rlp_encode() {
        // Given a Signature instance
        let sig = Signature::from_str("48b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353efffd310ac743f371de3b9f7f9cb56c0b28ad43601b4ab949f53faa07bd2c8041b").unwrap();

        // Initialize an empty buffer
        let mut buf = vec![];

        // Encode the Signature into the buffer
        sig.encode(&mut buf);

        // Define the expected hex-encoded string
        let expected = "01a048b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353a010002cef538bc0c8e21c46080634a93e082408b0ad93f4a7207e63ec5463793d";

        // Assert that the encoded buffer matches the expected hex-encoded string
        assert_eq!(hex::encode(buf.clone()), expected);
    }

    #[test]
    fn signature_rlp_length() {
        // Given a Signature instance
        let sig = Signature::from_str("48b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353efffd310ac743f371de3b9f7f9cb56c0b28ad43601b4ab949f53faa07bd2c8041b").unwrap();

        // Assert that the length of the Signature matches the expected length
        assert_eq!(sig.length(), 67);
    }
}
