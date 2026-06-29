// Helper to encode a big-endian varint (no leading zeroes)
// Nonce limit is 2**64 - 1 https://eips.ethereum.org/EIPS/eip-2681
#[cfg(feature = "eip7702")]
pub(crate) fn be_varint(n: u64) -> Vec<u8> {
    let mut buf = n.to_be_bytes().to_vec();
    while buf.first() == Some(&0) && buf.len() > 1 {
        buf.remove(0);
    }
    buf
}

// Tlv encoding for the 7702 authorization list
#[cfg(feature = "eip7702")]
pub(crate) fn make_eip7702_tlv(
    chain_id: alloy_primitives::U256,
    delegate: &[u8; 20],
    nonce: u64,
) -> Vec<u8> {
    let mut tlv = Vec::with_capacity(9 + 20);

    // STRUCT_VERSION tag=0x00, one-byte version=1
    tlv.push(0x00);
    tlv.push(1);
    tlv.push(1);

    // DELEGATE_ADDR tag=0x01
    tlv.push(0x01);
    tlv.push(20);
    tlv.extend_from_slice(delegate);

    // CHAIN_ID tag=0x02
    let ci = be_varint(chain_id.to::<u64>());
    tlv.push(0x02);
    tlv.push(ci.len() as u8);
    tlv.extend_from_slice(&ci);

    // NONCE tag=0x03
    let nn = be_varint(nonce);
    tlv.push(0x03);
    tlv.push(nn.len() as u8);
    tlv.extend_from_slice(&nn);

    tlv
}
