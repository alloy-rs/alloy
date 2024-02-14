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

impl From<Option<Address>> for TxKind {
    /// Creates a `TxKind::Call` with the `Some` address, `None` otherwise.
    #[inline]
    fn from(value: Option<Address>) -> Self {
        match value {
            None => TxKind::Create,
            Some(addr) => TxKind::Call(addr),
        }
    }
}

impl From<Address> for TxKind {
    /// Creates a `TxKind::Call` with the given address.
    #[inline]
    fn from(value: Address) -> Self {
        TxKind::Call(value)
    }
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
#[cfg(test)]
mod tx_kind_tests {
    use super::*;
    use alloy_primitives::Address;
    use alloy_rlp::{Decodable, Encodable, EMPTY_STRING_CODE};

    #[test]
    fn test_from_option_address() {
        let addr = Some(Address::ZERO);
        let tx_kind = TxKind::from(addr);
        assert_eq!(tx_kind, TxKind::Call(Address::ZERO));

        let none_addr: Option<Address> = None;
        let tx_kind_none = TxKind::from(none_addr);
        assert_eq!(tx_kind_none, TxKind::Create);
    }

    #[test]
    fn test_from_address() {
        let addr = Address::ZERO;
        let tx_kind = TxKind::from(addr);
        assert_eq!(tx_kind, TxKind::Call(addr));
    }

    #[test]
    fn test_to_method() {
        assert_eq!(TxKind::Create.to(), None);
        let addr = Address::ZERO;
        assert_eq!(TxKind::Call(addr).to(), Some(addr));
    }

    #[test]
    fn test_is_create_and_is_call() {
        assert!(TxKind::Create.is_create());
        assert!(!TxKind::Create.is_call());

        let addr = Address::ZERO;
        assert!(!TxKind::Call(addr).is_create());
        assert!(TxKind::Call(addr).is_call());
    }

    #[test]
    fn test_size_method() {
        assert_eq!(TxKind::Create.size(), std::mem::size_of::<TxKind>());
        let addr = Address::ZERO;
        assert_eq!(TxKind::Call(addr).size(), std::mem::size_of::<TxKind>());
    }

    #[test]
    fn test_encode_decode() {
        let mut buf = Vec::new();
        let tx_kind_create = TxKind::Create;
        tx_kind_create.encode(&mut buf);
        assert_eq!(buf, vec![EMPTY_STRING_CODE]);
        let mut buf_slice = buf.as_slice();
        let decoded = TxKind::decode(&mut buf_slice).unwrap();
        assert_eq!(decoded, TxKind::Create);

        buf.clear();
        let addr = Address::ZERO;
        let tx_kind_call = TxKind::Call(addr);
        tx_kind_call.encode(&mut buf);
        assert_ne!(buf, vec![EMPTY_STRING_CODE]);
        buf_slice = buf.as_slice();
        let decoded = TxKind::decode(&mut buf_slice).unwrap();
        assert_eq!(decoded, TxKind::Call(addr));
    }
}
