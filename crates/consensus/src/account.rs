use alloy_primitives::{B256, U256};
use alloy_rlp::{Decodable, Encodable, Header as RlpHeader};

/// Represents an Account in the account trie.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Account {
    /// The account's balance.
    pub balance: U256,
    /// The hash of the code of the account.
    pub code_hash: B256,
    /// The account's nonce.
    #[cfg_attr(feature = "serde", serde(with = "alloy_serde::u64_via_ruint"))]
    pub nonce: u64,
    /// The hash of the storage account data.
    pub storage_root: B256,
}
impl Encodable for Account {
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        let list_header = RlpHeader {
            list: true,
            payload_length: self.balance.length()
                + self.code_hash.length()
                + self.nonce.length()
                + self.storage_root.length(),
        };

        list_header.encode(out);

        self.balance.encode(out);
        self.code_hash.encode(out);
        self.nonce.encode(out);
        self.storage_root.encode(out);
    }
    fn length(&self) -> usize {
        let payload_length = self.balance.length()
            + self.code_hash.length()
            + self.nonce.length()
            + self.storage_root.length();
        RlpHeader { list: true, payload_length }.length() + payload_length
    }
}
impl Decodable for Account {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let rlp_header = RlpHeader::decode(buf)?;

        if !rlp_header.list {
            return Err(alloy_rlp::Error::UnexpectedString);
        }

        let balance = U256::decode(buf)?;
        let code_hash = B256::decode(buf)?;
        let nonce = u64::decode(buf)?;
        let storage_root = B256::decode(buf)?;

        let consumed = rlp_header.payload_length;
        let remaining = buf.len();
        if remaining != 0 {
            return Err(alloy_rlp::Error::ListLengthMismatch {
                expected: consumed,
                got: consumed - remaining,
            });
        }

        Ok(Account { balance, code_hash, nonce, storage_root })
    }
}
