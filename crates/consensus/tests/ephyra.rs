//! RLP decoding must consume exactly one signing payload from a stream.

use alloy_consensus::{TxLegacy, TypedTransaction};
use alloy_rlp::{Decodable, Encodable};

// Cross both the 55-byte and 255-byte RLP length boundaries.
#[ephyra::test(tests = 256, gen = (
    ephyra::any::<Option<u64>>(),
    ephyra::any::<u64>(),
    ephyra::gens::vec(ephyra::gens::int::<u8>(..)).len(0..=260),
))]
fn legacy_signing_payload_in_stream(chain_id: Option<u64>, nonce: u64, input: Vec<u8>) {
    let tx = TxLegacy { chain_id, nonce, input: input.into(), ..Default::default() };
    let next = TxLegacy::default();
    let mut stream = alloy_rlp::encode(&tx);
    assert_eq!(stream.len(), tx.length());
    let mut partial = &stream[..stream.len() - 1];
    assert!(TxLegacy::decode(&mut partial).is_err());
    next.encode(&mut stream);

    let mut cursor = stream.as_slice();
    assert_eq!(
        TypedTransaction::decode_unsigned(&mut cursor).unwrap(),
        TypedTransaction::Legacy(tx)
    );
    assert_eq!(TxLegacy::decode(&mut cursor), Ok(next));
    assert!(cursor.is_empty());
}

#[test]
fn six_field_payload_leaves_the_next_item_unread() {
    let tx = TxLegacy::default();
    let mut stream = alloy_rlp::encode(&tx);
    1u8.encode(&mut stream);
    let mut cursor = stream.as_slice();
    assert_eq!(TxLegacy::decode(&mut cursor), Ok(tx));
    assert_eq!(cursor, &[1]);
}
