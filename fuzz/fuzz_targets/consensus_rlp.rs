#![no_main]

use alloy_consensus::{
    Block, BlockBody, EthereumTxEnvelope, Header, Receipt, ReceiptEnvelope, ReceiptWithBloom,
    TxEip1559, TxEip2930, TxEip4844, TxEip7702,
};
use alloy_eips::eip4895::{Withdrawal, Withdrawals};
use alloy_rlp::{Decodable, Encodable};
use libfuzzer_sys::fuzz_target;

type BasicTxEnvelope = EthereumTxEnvelope<TxEip4844>;

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

fn raw_roundtrip_properties(data: &[u8]) {
    assert_roundtrip::<Header>(data, "Header");
    assert_roundtrip::<BlockBody<BasicTxEnvelope>>(data, "BlockBody<BasicTxEnvelope>");
    assert_roundtrip::<Block<BasicTxEnvelope>>(data, "Block<BasicTxEnvelope>");
    assert_roundtrip::<TxEip2930>(data, "TxEip2930");
    assert_roundtrip::<TxEip1559>(data, "TxEip1559");
    assert_roundtrip::<TxEip4844>(data, "TxEip4844");
    assert_roundtrip::<TxEip7702>(data, "TxEip7702");
    assert_roundtrip::<ReceiptEnvelope>(data, "ReceiptEnvelope");
    assert_roundtrip::<ReceiptWithBloom<Receipt>>(data, "ReceiptWithBloom<Receipt>");
    assert_roundtrip::<Withdrawal>(data, "Withdrawal");
    assert_roundtrip::<Withdrawals>(data, "Withdrawals");
}

fuzz_target!(|data: &[u8]| {
    raw_roundtrip_properties(data);
});
