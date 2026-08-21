#![no_main]

use alloy_consensus::{
    BlobTransactionSidecar, BlobTransactionSidecarEip7594, BlobTransactionSidecarVariant, Block,
    BlockBody, EthereumReceipt, EthereumTxEnvelope, Extended, Header, Receipt, ReceiptEnvelope,
    ReceiptWithBloom, Receipts, TxEip1559, TxEip2930, TxEip4844, TxEip4844WithSidecar,
    TxEip7702, TxEnvelope, TxLegacy,
};
use alloy_consensus::transaction::PooledTransaction;
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

fn is_explicitly_tagged_legacy(data: &[u8]) -> bool {
    // `EthereumTxEnvelope` currently normalizes an explicitly type-0-prefixed legacy transaction
    // to the ordinary untagged legacy encoding: its `Legacy` variant does not retain whether the
    // input contained the optional 0x00 prefix. That known representation ambiguity is therefore
    // outside the injectivity oracle; all untagged legacy and nonzero typed encodings remain in
    // scope.
    data.first() == Some(&0)
}

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
    if is_explicitly_tagged_legacy(data) {
        return;
    }
    assert_roundtrip::<BasicTxEnvelope>(data, "BasicTxEnvelope");
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
    assert_roundtrip::<TxLegacy>(data, "TxLegacy");
    assert_roundtrip::<TxEip2930>(data, "TxEip2930");
    assert_roundtrip::<TxEip1559>(data, "TxEip1559");
    assert_roundtrip::<TxEip4844>(data, "TxEip4844");
    assert_roundtrip::<TxEip7702>(data, "TxEip7702");
    if !is_explicitly_tagged_legacy(data) {
        assert_roundtrip::<PooledTransaction>(data, "PooledTransaction");
        assert_roundtrip::<VariantPooledTransaction>(data, "VariantPooledTransaction");
        assert_2718_roundtrip::<PooledTransaction>(data, "PooledTransaction");
        assert_2718_roundtrip::<VariantPooledTransaction>(data, "VariantPooledTransaction");
    }
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
    assert_fallback_error_preserves_cursor(data);
}

fuzz_target!(|data: &[u8]| {
    raw_roundtrip_properties(data);
});
