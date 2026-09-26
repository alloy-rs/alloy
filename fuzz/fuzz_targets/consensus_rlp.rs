#![no_main]

use alloy_consensus::{
    BlobTransactionSidecar, BlobTransactionSidecarEip7594, BlobTransactionSidecarVariant, Block,
    BlockBody, EthereumReceipt, EthereumTxEnvelope, Extended, Header, Receipt, ReceiptEnvelope,
    ReceiptWithBloom, Receipts, TxEip1559, TxEip2930, TxEip4844, TxEip4844WithSidecar,
    TxEip7702, TxEnvelope, TxLegacy,
};
use alloy_consensus::transaction::PooledTransaction;
use alloy_consensus_any::AnyReceiptEnvelope;
use alloy_eips::{
    eip2718::{Decodable2718, Encodable2718},
    eip4895::{Withdrawal, Withdrawals},
    eip7594::{Decodable7594, Encodable7594},
};
use alloy_rlp::{Decodable, Encodable};
use libfuzzer_sys::fuzz_target;

include!("../types.rs");

type BasicTxEnvelope = EthereumTxEnvelope<TxEip4844>;
type ExtendedTransaction = Extended<TxEip1559, TxEip2930>;
type VariantPooledTransaction =
    EthereumTxEnvelope<TxEip4844WithSidecar<BlobTransactionSidecarVariant>>;

fn assert_roundtrip<T>(data: &[u8], name: &str)
where
    T: Decodable + Encodable,
{
    let mut input = data;
    let Ok(decoded) = T::decode(&mut input) else {
        return;
    };

    let consumed = data.len() - input.len();
    let mut reencoded = Vec::with_capacity(consumed);
    decoded.encode(&mut reencoded);
    assert_eq!(&reencoded[..], &data[..consumed], "{name}: encode(decode(bytes)) != bytes");
}

fn assert_legacy_compatibility(data: &[u8]) {
    let mut input = data;
    let Ok(decoded) = TxLegacy::decode(&mut input) else {
        return;
    };

    let mut canonical = Vec::new();
    decoded.encode(&mut canonical);
    let mut canonical_input = canonical.as_slice();
    let canonical_decoded = TxLegacy::decode(&mut canonical_input)
        .expect("TxLegacy canonical re-encoding must decode");
    assert!(canonical_input.is_empty(), "TxLegacy canonical re-encoding left a suffix");

    let mut second = Vec::new();
    canonical_decoded.encode(&mut second);
    assert_eq!(second, canonical, "TxLegacy compatibility canonicalization is not stable");
}

fn assert_typed_network_compatibility<T>(data: &[u8], name: &str)
where
    T: Decodable + Encodable,
{
    let mut input = data;
    let Ok(decoded) = T::decode(&mut input) else {
        return;
    };

    let consumed = data.len() - input.len();
    let mut canonical = Vec::new();
    decoded.encode(&mut canonical);
    if canonical == data[..consumed] {
        return;
    }

    assert!(
        data.first().is_some_and(|ty| (1..=0x7f).contains(ty)),
        "{name}: undocumented non-canonical network encoding"
    );
    let mut canonical_input = canonical.as_slice();
    let canonical_decoded = T::decode(&mut canonical_input)
        .expect("canonical typed network re-encoding must decode");
    assert!(canonical_input.is_empty(), "{name}: canonical network encoding left a suffix");

    let mut second = Vec::new();
    canonical_decoded.encode(&mut second);
    assert_eq!(second, canonical, "{name}: network canonicalization is not stable");
}

fn assert_sealed_block_roundtrip(data: &[u8]) {
    let mut input = data;
    let Ok(sealed) = Block::<BasicTxEnvelope>::decode_sealed(&mut input) else {
        return;
    };

    let consumed = data.len() - input.len();
    let mut reencoded = Vec::with_capacity(consumed);
    sealed.inner().encode(&mut reencoded);

    assert_eq!(
        &reencoded[..],
        &data[..consumed],
        "Block::decode_sealed: encode(decode_sealed(bytes)) != bytes"
    );
}

fn assert_sealed_header_roundtrip(data: &[u8]) {
    let mut input = data;
    let Ok(sealed) = Header::decode_sealed(&mut input) else {
        return;
    };

    let consumed = data.len() - input.len();
    let mut reencoded = Vec::with_capacity(consumed);
    sealed.inner().encode(&mut reencoded);

    assert_eq!(
        &reencoded[..],
        &data[..consumed],
        "Header::decode_sealed: encode(decode_sealed(bytes)) != bytes"
    );
}

fn assert_2718_roundtrip<T>(data: &[u8], name: &str)
where
    T: Decodable2718 + Encodable2718,
{
    let mut input = data;
    let Ok(decoded) = T::decode_2718(&mut input) else {
        return;
    };

    let consumed = data.len() - input.len();
    let mut reencoded = Vec::with_capacity(consumed);
    decoded.encode_2718(&mut reencoded);
    assert_eq!(
        &reencoded[..],
        &data[..consumed],
        "{name}: encode_2718(decode_2718(bytes)) != bytes"
    );

    if input.is_empty() {
        let exact = T::decode_2718_exact(data).expect("successful exact decode must be repeatable");
        let mut exact_reencoded = Vec::with_capacity(data.len());
        exact.encode_2718(&mut exact_reencoded);
        assert_eq!(exact_reencoded, data, "{name}: exact EIP-2718 decode is not canonical");
    } else {
        assert!(T::decode_2718_exact(data).is_err(), "{name}: exact decode accepted a suffix");
    }
}

fn assert_2718_network_roundtrip<T>(data: &[u8], name: &str)
where
    T: Decodable2718 + Encodable2718,
{
    let mut input = data;
    let Ok(decoded) = T::network_decode(&mut input) else {
        return;
    };

    let consumed = data.len() - input.len();
    let mut reencoded = Vec::with_capacity(consumed);
    decoded.network_encode(&mut reencoded);

    // `network_decode` intentionally accepts a bare typed EIP-2718 envelope for backwards
    // compatibility. It canonicalizes that form by adding the RLP string wrapper.
    if reencoded != data[..consumed]
        && data.first().is_some_and(|ty| (1..=0x7f).contains(ty))
    {
        let mut canonical_input = reencoded.as_slice();
        let canonical = T::network_decode(&mut canonical_input)
            .expect("canonical network re-encoding must decode");
        assert!(canonical_input.is_empty(), "canonical network re-encoding left a suffix");

        let mut second = Vec::new();
        canonical.network_encode(&mut second);
        assert_eq!(second, reencoded, "{name}: network canonicalization is not stable");
        return;
    }

    assert_eq!(
        &reencoded[..],
        &data[..consumed],
        "{name}: network_encode(network_decode(bytes)) != bytes"
    );
}

fn assert_7594_roundtrip<T>(data: &[u8], name: &str)
where
    T: Decodable7594 + Encodable7594,
{
    let mut input = data;
    let Ok(decoded) = T::decode_7594(&mut input) else {
        return;
    };

    let consumed = data.len() - input.len();
    let mut reencoded = Vec::with_capacity(consumed);
    decoded.encode_7594(&mut reencoded);
    assert_eq!(
        &reencoded[..],
        &data[..consumed],
        "{name}: encode_7594(decode_7594(bytes)) != bytes"
    );
}

fn assert_fallback_error_preserves_cursor(data: &[u8]) {
    let mut input = data;
    if TxEnvelope::fallback_decode(&mut input).is_err() {
        assert_eq!(input, data, "TxEnvelope::fallback_decode advanced the cursor on error");
    }
}

fn assert_tx_envelope_roundtrips(data: &[u8]) {
    assert_typed_network_compatibility::<BasicTxEnvelope>(data, "BasicTxEnvelope");
    assert_2718_roundtrip::<TxEnvelope>(data, "TxEnvelope");
}

fn raw_roundtrip_properties(data: &[u8]) {
    assert_roundtrip::<BasicTypes>(data, "BasicTypes");
    assert_roundtrip::<VectorTypes>(data, "VectorTypes");
    assert_roundtrip::<ArrayTypes>(data, "ArrayTypes");
    assert_roundtrip::<NestedTypes>(data, "NestedTypes");
    assert_roundtrip::<OptionalTypes>(data, "OptionalTypes");
    assert_roundtrip::<Header>(data, "Header");
    assert_roundtrip::<BlockBody<BasicTxEnvelope>>(data, "BlockBody<BasicTxEnvelope>");
    assert_roundtrip::<Block<BasicTxEnvelope>>(data, "Block<BasicTxEnvelope>");
    assert_legacy_compatibility(data);
    assert_roundtrip::<TxEip2930>(data, "TxEip2930");
    assert_roundtrip::<TxEip1559>(data, "TxEip1559");
    assert_roundtrip::<TxEip4844>(data, "TxEip4844");
    assert_roundtrip::<TxEip7702>(data, "TxEip7702");
    assert_typed_network_compatibility::<PooledTransaction>(data, "PooledTransaction");
    assert_typed_network_compatibility::<VariantPooledTransaction>(
        data,
        "VariantPooledTransaction",
    );
    assert_2718_roundtrip::<PooledTransaction>(data, "PooledTransaction");
    assert_2718_roundtrip::<VariantPooledTransaction>(data, "VariantPooledTransaction");
    assert_2718_network_roundtrip::<PooledTransaction>(data, "PooledTransaction");
    assert_2718_network_roundtrip::<VariantPooledTransaction>(data, "VariantPooledTransaction");
    assert_roundtrip::<ExtendedTransaction>(data, "Extended<TxEip1559, TxEip2930>");
    assert_roundtrip::<BlobTransactionSidecar>(data, "BlobTransactionSidecar");
    assert_roundtrip::<BlobTransactionSidecarEip7594>(data, "BlobTransactionSidecarEip7594");
    assert_roundtrip::<BlobTransactionSidecarVariant>(data, "BlobTransactionSidecarVariant");
    assert_7594_roundtrip::<BlobTransactionSidecar>(data, "BlobTransactionSidecar::7594");
    assert_7594_roundtrip::<BlobTransactionSidecarEip7594>(
        data,
        "BlobTransactionSidecarEip7594::7594",
    );
    assert_7594_roundtrip::<BlobTransactionSidecarVariant>(
        data,
        "BlobTransactionSidecarVariant::7594",
    );
    assert_roundtrip::<ReceiptEnvelope>(data, "ReceiptEnvelope");
    assert_roundtrip::<ReceiptWithBloom<Receipt>>(data, "ReceiptWithBloom<Receipt>");
    assert_roundtrip::<ReceiptWithBloom<EthereumReceipt>>(
        data,
        "ReceiptWithBloom<EthereumReceipt>",
    );
    assert_2718_roundtrip::<ReceiptWithBloom<EthereumReceipt>>(
        data,
        "ReceiptWithBloom<EthereumReceipt>",
    );
    assert_2718_network_roundtrip::<ReceiptWithBloom<EthereumReceipt>>(
        data,
        "ReceiptWithBloom<EthereumReceipt>",
    );
    assert_roundtrip::<Receipts<ReceiptEnvelope>>(data, "Receipts<ReceiptEnvelope>");
    assert_roundtrip::<Receipts<ReceiptWithBloom<Receipt>>>(
        data,
        "Receipts<ReceiptWithBloom<Receipt>>",
    );
    assert_roundtrip::<Withdrawal>(data, "Withdrawal");
    assert_roundtrip::<Withdrawals>(data, "Withdrawals");
    assert_sealed_block_roundtrip(data);
    assert_sealed_header_roundtrip(data);
    assert_tx_envelope_roundtrips(data);
    assert_2718_roundtrip::<ReceiptEnvelope>(data, "ReceiptEnvelope");
    assert_2718_network_roundtrip::<ReceiptEnvelope>(data, "ReceiptEnvelope");
    assert_2718_roundtrip::<AnyReceiptEnvelope>(data, "AnyReceiptEnvelope");
    assert_2718_network_roundtrip::<AnyReceiptEnvelope>(data, "AnyReceiptEnvelope");
    assert_fallback_error_preserves_cursor(data);
}

fuzz_target!(|data: &[u8]| {
    raw_roundtrip_properties(data);
});
