use alloy_consensus::{
    BlobTransactionSidecar, BlobTransactionSidecarEip7594, BlobTransactionSidecarVariant, Block,
    BlockBody, EthereumReceipt, EthereumTxEnvelope, Extended, Header, Receipt, ReceiptEnvelope,
    ReceiptWithBloom, Receipts, SignableTransaction, TxEip1559, TxEip2930, TxEip4844, TxEip7702,
    TxEnvelope, TxLegacy,
};
use alloy_eips::{
    eip4844::{Blob, Bytes48},
    eip2718::{Decodable2718, Encodable2718},
    eip4895::Withdrawals,
    eip7594::CELLS_PER_EXT_BLOB,
};
use alloy_primitives::Signature;
use alloy_rlp::{Decodable, Encodable, Header as RlpHeader, EMPTY_STRING_CODE};
use std::{fmt::Debug, fs, path::Path};

include!("types.rs");

type BasicTxEnvelope = EthereumTxEnvelope<TxEip4844>;
type ExtendedTransaction = Extended<TxEip1559, TxEip2930>;

fn write_seed(corpus_dir: &Path, name: &str, bytes: &[u8]) {
    let path = corpus_dir.join(name);
    fs::write(&path, bytes).unwrap_or_else(|err| panic!("failed to write {name}: {err}"));
}

fn seed<T>(corpus_dir: &Path, name: &str, value: T)
where
    T: Debug + PartialEq + Encodable + Decodable,
{
    let bytes = alloy_rlp::encode(&value);
    let mut input = bytes.as_slice();
    let decoded = T::decode(&mut input).unwrap_or_else(|err| panic!("{name}: {err:?}"));
    assert!(input.is_empty(), "{name}: decode left trailing bytes");
    assert_eq!(decoded, value, "{name}: decode(encode(value)) differs");
    write_seed(corpus_dir, name, &bytes);
}

fn seed_2718<T>(corpus_dir: &Path, name: &str, value: T)
where
    T: Debug + PartialEq + Encodable2718 + Decodable2718,
{
    let mut bytes = Vec::with_capacity(value.encode_2718_len());
    value.encode_2718(&mut bytes);
    let decoded = T::decode_2718_exact(&bytes).unwrap_or_else(|err| panic!("{name}: {err:?}"));
    assert_eq!(decoded, value, "{name}: decode_2718(encode_2718(value)) differs");
    write_seed(corpus_dir, name, &bytes);
}

/// `trailing(no_gaps)` treats a payload item as the next optional value rather than a `None`
/// sentinel. A zero-length string is consequently rejected for this fixed-size field.
fn assert_no_explicit_none(value: &OptionalTypes) {
    let encoded = alloy_rlp::encode(value);
    let mut payload = encoded.as_slice();
    let header =
        RlpHeader::decode(&mut payload).expect("optional types must encode as an RLP list");
    assert!(header.list);
    assert_eq!(payload.len(), header.payload_length);

    let mut non_canonical = Vec::with_capacity(encoded.len() + 1);
    RlpHeader { list: true, payload_length: header.payload_length + 1 }.encode(&mut non_canonical);
    non_canonical.extend_from_slice(payload);
    non_canonical.push(EMPTY_STRING_CODE);

    assert!(OptionalTypes::decode(&mut non_canonical.as_slice()).is_err());
}

fn basic(extreme: bool) -> BasicTypes {
    BasicTypes {
        boolean: extreme,
        u8_value: if extreme { u8::MAX } else { 0 },
        u16_value: if extreme { u16::MAX } else { 0 },
        u32_value: if extreme { u32::MAX } else { 0 },
        u64_value: if extreme { u64::MAX } else { 0 },
        u128_value: if extreme { u128::MAX } else { 0 },
        bytes: if extreme { Bytes::from(vec![0xff; 56]) } else { Bytes::new() },
        string: if extreme { "RLP \u{1f600} boundary".into() } else { String::new() },
        address: if extreme { Address::repeat_byte(0xff) } else { Address::ZERO },
        hash: if extreme { B256::repeat_byte(0xff) } else { B256::ZERO },
    }
}

fn main() {
    let corpus_dir = Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("corpus")
        .join("consensus_rlp");
    fs::create_dir_all(&corpus_dir).expect("failed to create corpus directory");

    seed(&corpus_dir, "basic-zero", basic(false));
    seed(&corpus_dir, "basic-extreme", basic(true));
    seed(
        &corpus_dir,
        "vectors-empty",
        VectorTypes { integers: vec![], byte_strings: vec![], strings: vec![], basics: vec![] },
    );
    seed(
        &corpus_dir,
        "vectors-populated",
        VectorTypes {
            integers: vec![0, 1, 127, 128, u64::MAX],
            byte_strings: vec![
                Bytes::new(),
                Bytes::from_static(b"hello"),
                Bytes::from(vec![1; 56]),
            ],
            strings: vec![String::new(), "ascii".into(), "\u{1f600}".into()],
            basics: vec![basic(false), basic(true)],
        },
    );
    seed(
        &corpus_dir,
        "arrays",
        ArrayTypes {
            bytes: [0xff; 32],
            integers: vec![0, 128, u32::MAX],
            hashes: vec![B256::ZERO, B256::repeat_byte(0xff)],
        },
    );
    seed(
        &corpus_dir,
        "nested",
        NestedTypes {
            basic: basic(true),
            vectors: VectorTypes {
                integers: vec![1, 2, 3],
                byte_strings: vec![Bytes::from(vec![0x42; 56])],
                strings: vec!["nested".into()],
                basics: vec![basic(false)],
            },
            arrays: ArrayTypes {
                bytes: [0x42; 32],
                integers: vec![1, 2, 3],
                hashes: vec![B256::repeat_byte(1), B256::repeat_byte(2)],
            },
        },
    );
    let optional_none = OptionalTypes {
        required: 0,
        bytes: Bytes::new(),
        optional_hash: None,
        optional_values: None,
    };
    assert_no_explicit_none(&optional_none);
    seed(&corpus_dir, "optional-none", optional_none);
    seed(
        &corpus_dir,
        "optional-hash",
        OptionalTypes {
            required: 1,
            bytes: Bytes::from_static(b"optional"),
            optional_hash: Some(B256::repeat_byte(0x42)),
            optional_values: None,
        },
    );
    seed(
        &corpus_dir,
        "optional-all",
        OptionalTypes {
            required: u64::MAX,
            bytes: Bytes::from(vec![0xff; 56]),
            optional_hash: Some(B256::repeat_byte(0xff)),
            optional_values: Some(vec![0, 1, 127, 128, u64::MAX]),
        },
    );

    seed(&corpus_dir, "header-default", Header::default());
    seed(&corpus_dir, "block-body-empty", BlockBody::<BasicTxEnvelope>::default());
    seed(
        &corpus_dir,
        "block-empty",
        Block { header: Header::default(), body: BlockBody::<BasicTxEnvelope>::default() },
    );
    seed(&corpus_dir, "tx-legacy-default", TxLegacy::default());
    seed(&corpus_dir, "tx-eip2930-default", TxEip2930::default());
    seed(&corpus_dir, "tx-eip1559-default", TxEip1559::default());
    seed(&corpus_dir, "tx-eip4844-default", TxEip4844::default());
    seed(&corpus_dir, "tx-eip7702-default", TxEip7702::default());
    seed(&corpus_dir, "extended-built-in", ExtendedTransaction::BuiltIn(TxEip1559::default()));
    seed(&corpus_dir, "extended-other", ExtendedTransaction::Other(TxEip2930::default()));
    seed(&corpus_dir, "sidecar-eip4844-empty", BlobTransactionSidecar::default());
    seed(&corpus_dir, "sidecar-eip7594-empty", BlobTransactionSidecarEip7594::default());
    seed(&corpus_dir, "sidecar-variant-eip4844-empty", BlobTransactionSidecarVariant::default());
    seed(
        &corpus_dir,
        "sidecar-variant-eip7594-empty",
        BlobTransactionSidecarVariant::Eip7594(BlobTransactionSidecarEip7594::default()),
    );
    let sidecar_eip4844 = BlobTransactionSidecar {
        blobs: vec![Blob::ZERO],
        commitments: vec![Bytes48::ZERO],
        proofs: vec![Bytes48::ZERO],
    };
    seed(&corpus_dir, "sidecar-eip4844-one-blob", sidecar_eip4844.clone());
    seed(
        &corpus_dir,
        "sidecar-variant-eip4844-one-blob",
        BlobTransactionSidecarVariant::Eip4844(sidecar_eip4844),
    );
    let sidecar_eip7594 = BlobTransactionSidecarEip7594::new(
        vec![Blob::ZERO],
        vec![Bytes48::ZERO],
        vec![Bytes48::ZERO; CELLS_PER_EXT_BLOB],
    );
    seed(&corpus_dir, "sidecar-eip7594-one-blob", sidecar_eip7594.clone());
    seed(
        &corpus_dir,
        "sidecar-variant-eip7594-one-blob",
        BlobTransactionSidecarVariant::Eip7594(sidecar_eip7594),
    );
    let populated_receipt = ReceiptWithBloom {
        receipt: Receipt {
            status: true.into(),
            cumulative_gas_used: 21_000,
            logs: vec![Default::default()],
        },
        logs_bloom: Default::default(),
    };
    seed(
        &corpus_dir,
        "receipt-envelope-eip1559-populated",
        ReceiptEnvelope::Eip1559(populated_receipt.clone()),
    );
    seed(
        &corpus_dir,
        "receipts-envelope-populated",
        Receipts {
            receipt_vec: vec![vec![ReceiptEnvelope::Eip1559(populated_receipt.clone())]],
        },
    );
    seed(
        &corpus_dir,
        "receipts-with-bloom-populated",
        Receipts { receipt_vec: vec![vec![populated_receipt]] },
    );
    seed(
        &corpus_dir,
        "receipt-envelope-legacy-empty",
        ReceiptEnvelope::Legacy(ReceiptWithBloom::default()),
    );
    seed(
        &corpus_dir,
        "receipt-with-bloom-ethereum-empty",
        ReceiptWithBloom::<EthereumReceipt>::default(),
    );
    seed(&corpus_dir, "receipts-envelope-empty", Receipts::<ReceiptEnvelope>::default());
    seed(
        &corpus_dir,
        "receipts-with-bloom-empty",
        Receipts::<ReceiptWithBloom<Receipt>>::default(),
    );
    let signature = Signature::test_signature();
    seed_2718(
        &corpus_dir,
        "tx-envelope-legacy-default",
        TxEnvelope::Legacy(TxLegacy::default().into_signed(signature)),
    );
    seed_2718(
        &corpus_dir,
        "tx-envelope-eip1559-default",
        TxEnvelope::Eip1559(TxEip1559::default().into_signed(signature)),
    );
    seed(&corpus_dir, "withdrawals-empty", Withdrawals::default());
}
