use alloy_primitives::Address;
use alloy_rlp::{Buf, BufMut, Decodable, Encodable, EMPTY_STRING_CODE};

/// The `to` field of a transaction. Either a target address, or empty for a
/// contract creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TxKind {
    /// A transaction that creates a contract.
    #[default]
    Create,
    /// A transaction that calls a contract or transfer.
    Call(Address),
}

impl TxKind {
    /// Returns the address of the contract that will be called or will receive the transfer.
    pub const fn to(self) -> Option<Address> {
        match self {
            TxKind::Create => None,
            TxKind::Call(to) => Some(to),
        }
    }

    /// Returns true if the transaction is a contract creation.
    #[inline]
    pub const fn is_create(self) -> bool {
        matches!(self, TxKind::Create)
    }

    /// Returns true if the transaction is a contract call.
    #[inline]
    pub const fn is_call(self) -> bool {
        matches!(self, TxKind::Call(_))
    }

    /// Calculates a heuristic for the in-memory size of this object.
    #[inline]
    pub const fn size(self) -> usize {
        std::mem::size_of::<Self>()
    }
}

impl Encodable for TxKind {
    fn encode(&self, out: &mut dyn BufMut) {
        match self {
            TxKind::Call(to) => to.encode(out),
            TxKind::Create => out.put_u8(EMPTY_STRING_CODE),
        }
    }
    fn length(&self) -> usize {
        match self {
            TxKind::Call(to) => to.length(),
            TxKind::Create => 1, // EMPTY_STRING_CODE is a single byte
        }
    }
}

impl Decodable for TxKind {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        if let Some(&first) = buf.first() {
            if first == EMPTY_STRING_CODE {
                buf.advance(1);
                Ok(TxKind::Create)
            } else {
                let addr = <Address as Decodable>::decode(buf)?;
                Ok(TxKind::Call(addr))
            }
        } else {
            Err(alloy_rlp::Error::InputTooShort)
        }
    }
}
