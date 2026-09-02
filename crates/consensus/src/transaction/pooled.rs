//! Defines the exact transaction variant that is allowed to be propagated over the eth p2p
//! protocol.

use super::EthereumTxEnvelope;
use crate::{
    error::ValueError,
    transaction::{TxEip8141, TxEip8141Variant, TxEip8141WithSidecar},
    Signed, TransactionEnvelope, TxEip1559, TxEip2930, TxEip4844, TxEip4844Variant,
    TxEip4844WithSidecar, TxEip7702, TxLegacy,
};
use alloy_eips::eip7594::{
    BlobTransactionSidecarEip7594, BlobTransactionSidecarVariant, Encodable7594,
};
use alloy_primitives::{Sealable, Sealed};

/// Pooled transaction format for Osaka and later.
///
/// This can contain an [EIP-7594] blob transaction with its cell-proof sidecar or any non-blob
/// signed Ethereum transaction. For cross-fork [EIP-4844]/[EIP-7594] handling, use
/// `EthereumTxEnvelope<TxEip4844WithSidecar<BlobTransactionSidecarVariant>>` instead.
///
/// The difference between this and the [`EthereumTxEnvelope<TxEip4844Variant<T>>`] is that this
/// type always requires the [`TxEip4844WithSidecar`] variant, because EIP-4844 transaction can only
/// be propagated with the sidecar over p2p.
///
/// After the Osaka upgrade (EIP-7594), the blob sidecar uses
/// [`BlobTransactionSidecarEip7594`] which replaces single KZG proofs with cell proofs
/// for PeerDAS data availability sampling.
///
/// [EIP-4844]: https://eips.ethereum.org/EIPS/eip-4844
/// [EIP-7594]: https://eips.ethereum.org/EIPS/eip-7594
pub type PooledBlobTransaction =
    EthereumTxEnvelope<TxEip4844WithSidecar<BlobTransactionSidecarEip7594>>;

/// Exact pooled transaction envelope for the current protocol.
///
/// EIP-4844 transactions and EIP-8141 transactions with blobs carry an EIP-7594 sidecar in the
/// p2p representation. Non-blob EIP-8141 transactions use their canonical representation. A
/// sidecar is never included in the transaction hash or block transaction trie.
#[derive(Clone, Debug, TransactionEnvelope)]
#[envelope(
    alloy_consensus = crate,
    tx_type_name = PooledTxType,
    typed = PooledTypedTransaction,
    arbitrary_cfg(feature = "arbitrary")
)]
pub enum PooledTransaction {
    /// An untagged legacy transaction.
    #[envelope(ty = 0)]
    Legacy(Signed<TxLegacy>),
    /// An EIP-2930 transaction.
    #[envelope(ty = 1)]
    Eip2930(Signed<TxEip2930>),
    /// An EIP-1559 transaction.
    #[envelope(ty = 2)]
    Eip1559(Signed<TxEip1559>),
    /// An EIP-4844 transaction with its EIP-7594 sidecar.
    #[envelope(ty = 3)]
    Eip4844(Signed<TxEip4844WithSidecar<BlobTransactionSidecarEip7594>>),
    /// An EIP-7702 transaction.
    #[envelope(ty = 4)]
    Eip7702(Signed<TxEip7702>),
    /// An EIP-8141 transaction, optionally with its EIP-7594 sidecar.
    #[envelope(ty = 6)]
    Eip8141(Sealed<TxEip8141Variant<BlobTransactionSidecarEip7594>>),
}

impl PooledTransaction {
    /// Returns the transaction type.
    pub const fn tx_type(&self) -> crate::TxType {
        match self {
            Self::Legacy(_) => crate::TxType::Legacy,
            Self::Eip2930(_) => crate::TxType::Eip2930,
            Self::Eip1559(_) => crate::TxType::Eip1559,
            Self::Eip4844(_) => crate::TxType::Eip4844,
            Self::Eip7702(_) => crate::TxType::Eip7702,
            Self::Eip8141(_) => crate::TxType::Eip8141,
        }
    }

    /// Returns true if this is a legacy transaction.
    pub const fn is_legacy(&self) -> bool {
        matches!(self, Self::Legacy(_))
    }

    /// Returns true if this is an EIP-2930 transaction.
    pub const fn is_eip2930(&self) -> bool {
        matches!(self, Self::Eip2930(_))
    }

    /// Returns true if this is an EIP-1559 transaction.
    pub const fn is_eip1559(&self) -> bool {
        matches!(self, Self::Eip1559(_))
    }

    /// Returns true if this is an EIP-4844 transaction.
    pub const fn is_eip4844(&self) -> bool {
        matches!(self, Self::Eip4844(_))
    }

    /// Returns true if this is an EIP-7702 transaction.
    pub const fn is_eip7702(&self) -> bool {
        matches!(self, Self::Eip7702(_))
    }

    /// Returns true if this is an EIP-8141 transaction.
    pub const fn is_eip8141(&self) -> bool {
        matches!(self, Self::Eip8141(_))
    }

    /// Clears blob data while retaining commitments and proofs for eth/72 propagation.
    pub fn clear_eip7594_blobs(&mut self) {
        match self {
            Self::Eip4844(tx) => tx.tx_mut().sidecar.clear_eip7594_blobs(),
            Self::Eip8141(tx) => {
                if let Some(sidecar) = tx.inner_mut().sidecar_mut() {
                    sidecar.clear_eip7594_blobs();
                }
            }
            _ => {}
        }
    }

    /// Returns the EIP-4844 pooled transaction, if this is one.
    pub const fn as_eip4844(
        &self,
    ) -> Option<&Signed<TxEip4844WithSidecar<BlobTransactionSidecarEip7594>>> {
        match self {
            Self::Eip4844(tx) => Some(tx),
            _ => None,
        }
    }

    /// Returns the EIP-8141 pooled transaction, if this is one.
    pub const fn as_eip8141(
        &self,
    ) -> Option<&Sealed<TxEip8141Variant<BlobTransactionSidecarEip7594>>> {
        match self {
            Self::Eip8141(tx) => Some(tx),
            _ => None,
        }
    }
}

impl From<Signed<TxLegacy>> for PooledTransaction {
    fn from(value: Signed<TxLegacy>) -> Self {
        Self::Legacy(value)
    }
}

impl From<Signed<TxEip2930>> for PooledTransaction {
    fn from(value: Signed<TxEip2930>) -> Self {
        Self::Eip2930(value)
    }
}

impl From<Signed<TxEip1559>> for PooledTransaction {
    fn from(value: Signed<TxEip1559>) -> Self {
        Self::Eip1559(value)
    }
}

impl From<Signed<TxEip4844WithSidecar<BlobTransactionSidecarEip7594>>> for PooledTransaction {
    fn from(value: Signed<TxEip4844WithSidecar<BlobTransactionSidecarEip7594>>) -> Self {
        Self::Eip4844(value)
    }
}

impl From<Signed<TxEip7702>> for PooledTransaction {
    fn from(value: Signed<TxEip7702>) -> Self {
        Self::Eip7702(value)
    }
}

impl From<TxEip8141WithSidecar<BlobTransactionSidecarEip7594>> for PooledTransaction {
    fn from(value: TxEip8141WithSidecar<BlobTransactionSidecarEip7594>) -> Self {
        Self::Eip8141(TxEip8141Variant::from(value).seal_slow())
    }
}

impl From<TxEip8141> for PooledTransaction {
    fn from(value: TxEip8141) -> Self {
        Self::Eip8141(TxEip8141Variant::from(value).seal_slow())
    }
}

impl EthereumTxEnvelope<TxEip4844WithSidecar<BlobTransactionSidecarEip7594>> {
    /// Clears EIP-7594 blob payloads while retaining commitments and cell proofs.
    ///
    /// This prepares the transaction for inclusion in an eth/72 `PooledTransactions` response as
    /// specified by [EIP-8070]. This has no effect on non-blob transactions.
    ///
    /// [EIP-8070]: https://eips.ethereum.org/EIPS/eip-8070
    pub fn clear_eip7594_blobs(&mut self) {
        if let Self::Eip4844(tx) = self {
            tx.tx_mut().sidecar.clear_eip7594_blobs();
        }
    }
}

impl EthereumTxEnvelope<TxEip4844WithSidecar<BlobTransactionSidecarVariant>> {
    /// Clears EIP-7594 blob payloads while retaining commitments and cell proofs.
    ///
    /// This prepares the transaction for inclusion in an eth/72 `PooledTransactions` response as
    /// specified by [EIP-8070]. This has no effect on non-blob transactions or EIP-4844 sidecars.
    ///
    /// [EIP-8070]: https://eips.ethereum.org/EIPS/eip-8070
    pub fn clear_eip7594_blobs(&mut self) {
        if let Self::Eip4844(tx) = self {
            tx.tx_mut().sidecar.clear_eip7594_blobs();
        }
    }
}

impl<T: Encodable7594> EthereumTxEnvelope<TxEip4844WithSidecar<T>> {
    /// Converts the transaction into [`EthereumTxEnvelope<TxEip4844Variant<T>>`].
    pub fn into_envelope(self) -> EthereumTxEnvelope<TxEip4844Variant<T>> {
        match self {
            Self::Legacy(tx) => tx.into(),
            Self::Eip2930(tx) => tx.into(),
            Self::Eip1559(tx) => tx.into(),
            Self::Eip7702(tx) => tx.into(),
            Self::Eip4844(tx) => tx.into(),
            Self::Eip8141(tx) => EthereumTxEnvelope::Eip8141(tx),
        }
    }
}

impl<T: Encodable7594> TryFrom<Signed<TxEip4844Variant<T>>>
    for EthereumTxEnvelope<TxEip4844WithSidecar<T>>
{
    type Error = ValueError<Signed<TxEip4844Variant<T>>>;

    fn try_from(value: Signed<TxEip4844Variant<T>>) -> Result<Self, Self::Error> {
        let (value, signature, hash) = value.into_parts();
        match value {
            tx @ TxEip4844Variant::TxEip4844(_) => Err(ValueError::new_static(
                Signed::new_unchecked(tx, signature, hash),
                "pooled transaction requires 4844 sidecar",
            )),
            TxEip4844Variant::TxEip4844WithSidecar(tx) => {
                Ok(Signed::new_unchecked(tx, signature, hash).into())
            }
        }
    }
}

impl<T: Encodable7594> TryFrom<EthereumTxEnvelope<TxEip4844Variant<T>>>
    for EthereumTxEnvelope<TxEip4844WithSidecar<T>>
{
    type Error = ValueError<EthereumTxEnvelope<TxEip4844Variant<T>>>;

    fn try_from(value: EthereumTxEnvelope<TxEip4844Variant<T>>) -> Result<Self, Self::Error> {
        value.try_into_pooled()
    }
}

impl<T: Encodable7594> TryFrom<EthereumTxEnvelope<TxEip4844>>
    for EthereumTxEnvelope<TxEip4844WithSidecar<T>>
{
    type Error = ValueError<EthereumTxEnvelope<TxEip4844>>;

    fn try_from(value: EthereumTxEnvelope<TxEip4844>) -> Result<Self, Self::Error> {
        value.try_into_pooled()
    }
}

impl<T: Encodable7594> From<EthereumTxEnvelope<TxEip4844WithSidecar<T>>>
    for EthereumTxEnvelope<TxEip4844Variant<T>>
{
    fn from(tx: EthereumTxEnvelope<TxEip4844WithSidecar<T>>) -> Self {
        tx.into_envelope()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Transaction;
    use alloy_eips::{
        eip4844::{Blob, Bytes48},
        eip7594::CELLS_PER_EXT_BLOB,
        Decodable2718, Encodable2718,
    };
    use alloy_primitives::{address, hex, Bytes, Signature, U256};
    use alloy_rlp::Decodable;
    use std::path::PathBuf;

    fn eip7594_sidecar() -> BlobTransactionSidecarEip7594 {
        BlobTransactionSidecarEip7594::new(
            vec![Blob::repeat_byte(0x01)],
            vec![Bytes48::repeat_byte(0x02)],
            vec![Bytes48::repeat_byte(0x03); CELLS_PER_EXT_BLOB],
        )
    }

    fn signature() -> Signature {
        Signature::new(U256::from(1), U256::from(2), false)
    }

    #[test]
    fn clear_eip7594_blobs_from_pooled_envelopes() {
        let sidecar = eip7594_sidecar();
        let commitments = sidecar.commitments.clone();
        let cell_proofs = sidecar.cell_proofs.clone();
        let tx = TxEip4844WithSidecar::from_tx_and_sidecar(TxEip4844::default(), sidecar);
        let mut pooled: PooledTransaction = Signed::new_unhashed(tx, signature()).into();

        pooled.clear_eip7594_blobs();

        let sidecar = &pooled.as_eip4844().unwrap().tx().sidecar;
        assert!(sidecar.blobs.is_empty());
        assert_eq!(sidecar.commitments, commitments);
        assert_eq!(sidecar.cell_proofs, cell_proofs);

        let tx = TxEip4844WithSidecar::from_tx_and_sidecar(
            TxEip4844::default(),
            BlobTransactionSidecarVariant::Eip7594(eip7594_sidecar()),
        );
        let mut pooled = EthereumTxEnvelope::Eip4844(Signed::new_unhashed(tx, signature()));

        pooled.clear_eip7594_blobs();

        let sidecar = pooled.as_eip4844().unwrap().tx().sidecar.as_eip7594().unwrap();
        assert!(sidecar.blobs.is_empty());
        assert_eq!(sidecar.commitments, commitments);
        assert_eq!(sidecar.cell_proofs, cell_proofs);
    }

    #[test]
    fn invalid_legacy_pooled_decoding_input_too_short() {
        let input_too_short = [
            // this should fail because the payload length is longer than expected
            &hex!("d90b0280808bc5cd028083c5cdfd9e407c56565656")[..],
            // these should fail decoding
            //
            // The `c1` at the beginning is a list header, and the rest is a valid legacy
            // transaction, BUT the payload length of the list header is 1, and the payload is
            // obviously longer than one byte.
            &hex!("c10b02808083c5cd028883c5cdfd9e407c56565656"),
            &hex!("c10b0280808bc5cd028083c5cdfd9e407c56565656"),
            // this one is 19 bytes, and the buf is long enough, but the transaction will not
            // consume that many bytes.
            &hex!("d40b02808083c5cdeb8783c5acfd9e407c5656565656"),
            &hex!("d30102808083c5cd02887dc5cdfd9e64fd9e407c56"),
        ];

        for hex_data in &input_too_short {
            let input_rlp = &mut &hex_data[..];
            let res = PooledTransaction::decode(input_rlp);

            assert!(
                res.is_err(),
                "expected err after decoding rlp input: {:x?}",
                Bytes::copy_from_slice(hex_data)
            );

            // this is a legacy tx so we can attempt the same test with decode_enveloped
            let input_rlp = &mut &hex_data[..];
            let res = PooledTransaction::decode_2718(input_rlp);

            assert!(
                res.is_err(),
                "expected err after decoding enveloped rlp input: {:x?}",
                Bytes::copy_from_slice(hex_data)
            );
        }
    }

    // <https://holesky.etherscan.io/tx/0x7f60faf8a410a80d95f7ffda301d5ab983545913d3d789615df3346579f6c849>
    #[test]
    fn decode_eip1559_enveloped() {
        let data = hex!("02f903d382426882ba09832dc6c0848674742682ed9694714b6a4ea9b94a8a7d9fd362ed72630688c8898c80b90364492d24749189822d8512430d3f3ff7a2ede675ac08265c08e2c56ff6fdaa66dae1cdbe4a5d1d7809f3e99272d067364e597542ac0c369d69e22a6399c3e9bee5da4b07e3f3fdc34c32c3d88aa2268785f3e3f8086df0934b10ef92cfffc2e7f3d90f5e83302e31382e302d64657600000000000000000000000000000000000000000000569e75fc77c1a856f6daaf9e69d8a9566ca34aa47f9133711ce065a571af0cfd000000000000000000000000e1e210594771824dad216568b91c9cb4ceed361c00000000000000000000000000000000000000000000000000000000000546e00000000000000000000000000000000000000000000000000000000000e4e1c00000000000000000000000000000000000000000000000000000000065d6750c00000000000000000000000000000000000000000000000000000000000f288000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002cf600000000000000000000000000000000000000000000000000000000000000640000000000000000000000000000000000000000000000000000000000000000f1628e56fa6d8c50e5b984a58c0df14de31c7b857ce7ba499945b99252976a93d06dcda6776fc42167fbe71cb59f978f5ef5b12577a90b132d14d9c6efa528076f0161d7bf03643cfc5490ec5084f4a041db7f06c50bd97efa08907ba79ddcac8b890f24d12d8db31abbaaf18985d54f400449ee0559a4452afe53de5853ce090000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000028000000000000000000000000000000000000000000000000000000000000003e800000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000064ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff00000000000000000000000000000000000000000000000000000000c080a01428023fc54a27544abc421d5d017b9a7c5936ad501cbdecd0d9d12d04c1a033a0753104bbf1c87634d6ff3f0ffa0982710612306003eb022363b57994bdef445a"
);

        let res = PooledTransaction::decode_2718(&mut &data[..]).unwrap();
        assert_eq!(res.to(), Some(address!("714b6a4ea9b94a8a7d9fd362ed72630688c8898c")));
    }

    #[test]
    fn legacy_valid_pooled_decoding() {
        // d3 <- payload length, d3 - c0 = 0x13 = 19
        // 0b <- nonce
        // 02 <- gas_price
        // 80 <- gas_limit
        // 80 <- to (Create)
        // 83 c5cdeb <- value
        // 87 83c5acfd9e407c <- input
        // 56 <- v (eip155, so modified with a chain id)
        // 56 <- r
        // 56 <- s
        let data = &hex!("d30b02808083c5cdeb8783c5acfd9e407c565656")[..];

        let input_rlp = &mut &data[..];
        let res = PooledTransaction::decode(input_rlp);
        assert!(res.is_ok());
        assert!(input_rlp.is_empty());

        // we can also decode_enveloped
        let res = PooledTransaction::decode_2718(&mut &data[..]);
        assert!(res.is_ok());
    }

    #[test]
    fn decode_encode_raw_4844_rlp() {
        // Test data is in legacy EIP-4844 RLP format, so use BlobTransactionSidecarVariant
        // which can decode both EIP-4844 and EIP-7594.
        type VariantPooledTransaction = EthereumTxEnvelope<TxEip4844WithSidecar>;

        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/4844rlp");
        let dir = std::fs::read_dir(path).expect("Unable to read folder");
        for entry in dir {
            let entry = entry.unwrap();
            let content = std::fs::read_to_string(entry.path()).unwrap();
            let raw = hex::decode(content.trim()).unwrap();
            let tx = VariantPooledTransaction::decode_2718(&mut raw.as_ref())
                .map_err(|err| {
                    panic!("Failed to decode transaction: {:?} {:?}", err, entry.path());
                })
                .unwrap();
            // We want to test only EIP-4844 transactions
            assert!(tx.is_eip4844());
            let encoded = tx.encoded_2718();
            assert_eq!(encoded.as_slice(), &raw[..], "{:?}", entry.path());
        }
    }

    #[test]
    #[cfg(feature = "kzg")]
    fn convert_to_eip7594() {
        // Test data is in legacy EIP-4844 RLP format, so use BlobTransactionSidecarVariant
        // which can decode both EIP-4844 and EIP-7594.
        type VariantPooledTransaction = EthereumTxEnvelope<TxEip4844WithSidecar>;

        let kzg_settings = alloy_eips::eip4844::env_settings::EnvKzgSettings::default();
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/4844rlp");
        let dir = std::fs::read_dir(path).expect("Unable to read folder");
        for entry in dir {
            let entry = entry.unwrap();
            let content = std::fs::read_to_string(entry.path()).unwrap();
            let raw = hex::decode(content.trim()).unwrap();
            let VariantPooledTransaction::Eip4844(tx) =
                VariantPooledTransaction::decode_2718(&mut raw.as_ref())
                    .map_err(|err| {
                        panic!("Failed to decode transaction: {:?} {:?}", err, entry.path());
                    })
                    .unwrap()
            else {
                panic!("Expected EIP-4844 transaction");
            };
            let tx = tx.into_parts().0;
            assert!(!tx.sidecar.blobs().is_empty());
            assert!(tx.validate_blob(kzg_settings.get()).is_ok());

            let tx = tx
                .try_map_sidecar(|sidecar| {
                    sidecar.try_convert_into_eip7594_with_settings(kzg_settings.get())
                })
                .unwrap();

            assert!(!tx.sidecar.blobs().is_empty());
            assert!(tx.validate_blob(kzg_settings.get()).is_ok());
        }
    }

    /// Tests that `PooledTransaction` (with [`BlobTransactionSidecarEip7594`]) can encode and
    /// decode EIP-7594 blob transactions round-trip.
    #[test]
    #[cfg(feature = "kzg")]
    fn pooled_transaction_eip7594_roundtrip() {
        // Decode legacy 4844 test data, convert sidecar to EIP-7594, then verify
        // PooledTransaction roundtrips correctly with the new sidecar format.
        type VariantPooledTransaction = EthereumTxEnvelope<TxEip4844WithSidecar>;

        let kzg_settings = alloy_eips::eip4844::env_settings::EnvKzgSettings::default();
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/4844rlp");
        let dir = std::fs::read_dir(path).expect("Unable to read folder");
        for entry in dir {
            let entry = entry.unwrap();
            let content = std::fs::read_to_string(entry.path()).unwrap();
            let raw = hex::decode(content.trim()).unwrap();
            let VariantPooledTransaction::Eip4844(tx) =
                VariantPooledTransaction::decode_2718(&mut raw.as_ref())
                    .map_err(|err| {
                        panic!("Failed to decode transaction: {:?} {:?}", err, entry.path());
                    })
                    .unwrap()
            else {
                panic!("Expected EIP-4844 transaction");
            };

            // Convert the sidecar from EIP-4844 to EIP-7594
            let (tx_with_sidecar, sig, hash) = tx.into_parts();
            let tx_eip7594 = tx_with_sidecar
                .try_map_sidecar(|sidecar| {
                    sidecar.try_into_eip7594_with_settings(kzg_settings.get())
                })
                .unwrap();

            // Build a PooledTransaction (EIP-7594) and roundtrip encode/decode
            let pooled_tx: PooledTransaction = Signed::new_unchecked(tx_eip7594, sig, hash).into();
            assert!(pooled_tx.is_eip4844());

            let encoded = pooled_tx.encoded_2718();
            let decoded = PooledTransaction::decode_2718(&mut encoded.as_ref()).unwrap();
            assert_eq!(pooled_tx, decoded);
        }
    }

    #[test]
    fn pooled_eip8141_roundtrips_with_and_without_sidecar() {
        let tx = TxEip8141 {
            chain_id: 1,
            frames: vec![alloy_eips::eip8141::Frame::default()],
            ..Default::default()
        };

        let pooled: PooledTransaction = tx.clone().into();
        let encoded = pooled.encoded_2718();
        let decoded = PooledTransaction::decode_2718(&mut encoded.as_ref()).unwrap();
        assert_eq!(pooled, decoded);
        assert!(matches!(decoded.as_eip8141().unwrap().inner(), TxEip8141Variant::TxEip8141(_)));

        let pooled: PooledTransaction = TxEip8141WithSidecar::new(tx, eip7594_sidecar()).into();
        let encoded = pooled.encoded_2718();
        let decoded = PooledTransaction::decode_2718(&mut encoded.as_ref()).unwrap();
        assert_eq!(pooled, decoded);
        assert!(matches!(
            decoded.as_eip8141().unwrap().inner(),
            TxEip8141Variant::TxEip8141WithSidecar(_)
        ));
    }
}
