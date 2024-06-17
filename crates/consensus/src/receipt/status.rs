use alloy_primitives::B256;
use alloy_rlp::{Buf, BufMut, Decodable, Encodable, Error, Header};

/// Captures the result of a transaction execution.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
pub enum Eip658Value {
    /// A boolean `statusCode` introduced by [EIP-658].
    ///
    /// [EIP-658]: https://eips.ethereum.org/EIPS/eip-658
    Eip658(bool),
    /// A pre-[EIP-658] hash value.
    ///
    /// [EIP-658]: https://eips.ethereum.org/EIPS/eip-658
    PostState(B256),
}

impl Eip658Value {
    /// Returns true if the transaction was successful OR if the transaction
    /// is pre-[EIP-658].
    ///
    /// [EIP-658]: https://eips.ethereum.org/EIPS/eip-658
    pub const fn coerce_status(&self) -> bool {
        matches!(self, Self::Eip658(true) | Self::PostState(_))
    }

    /// Returns true if the transaction was a pre-[EIP-658] transaction.
    ///
    /// [EIP-658]: https://eips.ethereum.org/EIPS/eip-658
    pub const fn is_post_state(&self) -> bool {
        matches!(self, Self::PostState(_))
    }

    /// Returns true if the transaction was a post-[EIP-658] transaction.
    pub const fn is_eip658(&self) -> bool {
        !matches!(self, Self::PostState(_))
    }

    /// Fallibly convert to the post state.
    pub const fn as_post_state(&self) -> Option<B256> {
        match self {
            Self::PostState(state) => Some(*state),
            _ => None,
        }
    }

    /// Fallibly convert to the [EIP-658] status code.
    ///
    /// [EIP-658]: https://eips.ethereum.org/EIPS/eip-658
    pub const fn as_eip658(&self) -> Option<bool> {
        match self {
            Self::Eip658(status) => Some(*status),
            _ => None,
        }
    }
}

impl From<bool> for Eip658Value {
    fn from(status: bool) -> Self {
        Self::Eip658(status)
    }
}

impl From<B256> for Eip658Value {
    fn from(state: B256) -> Self {
        Self::PostState(state)
    }
}

// NB: default to success
impl Default for Eip658Value {
    fn default() -> Self {
        Self::Eip658(true)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Eip658Value {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Eip658(status) => alloy_serde::quantity::serialize(status, serializer),
            Self::PostState(state) => state.serialize(serializer),
        }
    }
}

#[cfg(feature = "serde")]
// NB: some visit methods partially or wholly copied from alloy-primitives
impl<'de> serde::Deserialize<'de> for Eip658Value {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de;
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Eip658Value;

            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str("a boolean or a 32-byte hash")
            }

            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(Eip658Value::Eip658(v))
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                match v {
                    "0x" | "0x0" | "false" => Ok(Eip658Value::Eip658(false)),
                    "0x1" | "true" => Ok(Eip658Value::Eip658(true)),
                    _ => v.parse::<B256>().map(Eip658Value::PostState).map_err(de::Error::custom),
                }
            }

            fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                B256::try_from(v).map(Eip658Value::PostState).map_err(de::Error::custom)
            }

            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let len_error = |i| de::Error::invalid_length(i, &"exactly 32 bytes");
                let mut bytes = [0u8; 32];

                for (i, byte) in bytes.iter_mut().enumerate() {
                    *byte = seq.next_element()?.ok_or_else(|| len_error(i))?;
                }

                if let Ok(Some(_)) = seq.next_element::<u8>() {
                    return Err(len_error(33));
                }

                Ok(Eip658Value::PostState(bytes.into()))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl Encodable for Eip658Value {
    fn encode(&self, buf: &mut dyn BufMut) {
        match self {
            Self::Eip658(status) => {
                status.encode(buf);
            }
            Self::PostState(state) => {
                state.encode(buf);
            }
        }
    }

    fn length(&self) -> usize {
        match self {
            Self::Eip658(_) => 1,
            Self::PostState(_) => 32,
        }
    }
}

impl Decodable for Eip658Value {
    fn decode(buf: &mut &[u8]) -> Result<Self, Error> {
        let h = Header::decode(buf)?;

        match h.payload_length {
            0 => Ok(Self::Eip658(false)),
            1 => {
                let status = buf.get_u8() != 0;
                Ok(status.into())
            }
            32 => {
                if buf.remaining() < 32 {
                    return Err(Error::InputTooShort);
                }
                let mut state = B256::default();
                buf.copy_to_slice(state.as_mut_slice());
                Ok(state.into())
            }
            _ => Err(Error::UnexpectedLength),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn rlp_sanity() {
        let mut buf = Vec::new();
        let status = Eip658Value::Eip658(true);
        status.encode(&mut buf);
        assert_eq!(Eip658Value::decode(&mut buf.as_slice()), Ok(status));

        let mut buf = Vec::new();
        let state = Eip658Value::PostState(B256::default());
        state.encode(&mut buf);
        assert_eq!(Eip658Value::decode(&mut buf.as_slice()), Ok(state));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_sanity() {
        let status: Eip658Value = true.into();
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#""0x1""#);
        assert_eq!(serde_json::from_str::<Eip658Value>(&json).unwrap(), status);

        let state: Eip658Value = false.into();
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, r#""0x0""#);

        let state: Eip658Value = B256::repeat_byte(1).into();
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, r#""0x0101010101010101010101010101010101010101010101010101010101010101""#);
    }
}
