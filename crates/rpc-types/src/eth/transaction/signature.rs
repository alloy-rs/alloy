//! Signature related RPC values
use alloy_primitives::U256;
use alloy_rlp::{Bytes, Decodable, Encodable, Error as RlpError};
use serde::{Deserialize, Serialize};

/// Container type for all signature fields in RPC
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Signature {
    /// The R field of the signature; the point on the curve.
    pub r: U256,
    /// The S field of the signature; the point on the curve.
    pub s: U256,
    // TODO: change these fields to an untagged enum for `v` XOR `y_parity` if/when CLs support it.
    // See <https://github.com/ethereum/go-ethereum/issues/27727> for more information
    /// For EIP-155, EIP-2930 and Blob transactions this is set to the parity (0 for even, 1 for
    /// odd) of the y-value of the secp256k1 signature.
    ///
    /// For legacy transactions, this is the recovery id
    ///
    /// See also <https://ethereum.github.io/execution-apis/api-documentation/> and <https://ethereum.org/en/developers/docs/apis/json-rpc/#eth_gettransactionbyhash>
    pub v: U256,
    /// The y parity of the signature. This is only used for typed (non-legacy) transactions.
    #[serde(default, rename = "yParity", skip_serializing_if = "Option::is_none")]
    pub y_parity: Option<Parity>,
}

impl Signature {
    /// Output the length of the signature without the length of the RLP header, using the legacy
    /// scheme with EIP-155 support depends on chain_id.
    pub fn payload_len_with_eip155_chain_id(&self, chain_id: Option<u64>) -> usize {
        self.v(chain_id).length() + self.r.length() + self.s.length()
    }

    /// Encode the `v`, `r`, `s` values without a RLP header.
    /// Encodes the `v` value using the legacy scheme with EIP-155 support depends on chain_id.
    pub fn encode_with_eip155_chain_id(
        &self,
        out: &mut dyn alloy_rlp::BufMut,
        chain_id: Option<u64>,
    ) {
        self.v(chain_id).encode(out);
        self.r.encode(out);
        self.s.encode(out);
    }

    /// Output the `v` of the signature depends on chain_id
    #[inline]
    pub fn v(&self, chain_id: Option<u64>) -> u64 {
        if let Some(chain_id) = chain_id {
            // EIP-155: v = {0, 1} + CHAIN_ID * 2 + 35
            let y_parity = u64::from(self.y_parity.unwrap_or(Parity(false)));
            y_parity + chain_id * 2 + 35
        } else {
            u64::from(self.y_parity.unwrap_or(Parity(false))) + 27
        }
    }

    /// Decodes the `v`, `r`, `s` values without a RLP header.
    /// This will return a chain ID if the `v` value is [EIP-155](https://github.com/ethereum/EIPs/blob/master/EIPS/eip-155.md) compatible.
    pub fn decode_with_eip155_chain_id(buf: &mut &[u8]) -> alloy_rlp::Result<(Self, Option<u64>)> {
        let v = u64::decode(buf)?;
        let r = Decodable::decode(buf)?;
        let s = Decodable::decode(buf)?;
        if v >= 35 {
            // EIP-155: v = {0, 1} + CHAIN_ID * 2 + 35
            let y_parity = ((v - 35) % 2) != 0;
            let chain_id = (v - 35) >> 1;
            Ok((
                Signature { r, s, y_parity: Some(Parity(y_parity)), v: U256::from(v) },
                Some(chain_id),
            ))
        } else {
            // non-EIP-155 legacy scheme, v = 27 for even y-parity, v = 28 for odd y-parity
            if v != 27 && v != 28 {
                return Err(RlpError::Custom("invalid Ethereum signature (V is not 27 or 28)"));
            }
            let y_parity = v == 28;
            Ok((Signature { r, s, y_parity: Some(Parity(y_parity)), v: U256::from(v) }, None))
        }
    }

    /// Output the length of the signature without the length of the RLP header
    pub fn payload_len(&self) -> usize {
        let y_parity_len = match self.y_parity {
            Some(parity) => parity.0 as usize,
            None => 0_usize,
        };
        y_parity_len + self.r.length() + self.s.length()
    }

    /// Encode the `y_parity`, `r`, `s` values without a RLP header.
    /// Panics if the y parity is not set.
    pub fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        self.y_parity.expect("y_parity not set").encode(out);
        self.r.encode(out);
        self.s.encode(out);
    }

    /// Decodes the `y_parity`, `r`, `s` values without a RLP header.
    pub fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let mut sig =
            Signature {
                y_parity: Some(Decodable::decode(buf)?),
                r: Decodable::decode(buf)?,
                s: Decodable::decode(buf)?,
                v: U256::ZERO,
            };
        sig.v = sig.y_parity.unwrap().into();
        Ok(sig)
    }

    /// Turn this signature into its byte
    /// (hex) representation.
    /// Panics: if the y_parity field is not set.
    pub fn to_bytes(&self) -> [u8; 65] {
        let mut sig = [0u8; 65];
        sig[..32].copy_from_slice(&self.r.to_be_bytes::<32>());
        sig[32..64].copy_from_slice(&self.s.to_be_bytes::<32>());
        let v = u8::from(self.y_parity.expect("y_parity not set")) + 27;
        sig[64] = v;
        sig
    }

    /// Turn this signature into its hex-encoded representation.
    pub fn to_hex_bytes(&self) -> Bytes {
        alloy_primitives::hex::encode(self.to_bytes()).into()
    }

    /// Calculates a heuristic for the in-memory size of the [Signature].
    #[inline]
    pub fn size(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

/// Type that represents the signature parity byte, meant for use in RPC.
///
/// This will be serialized as "0x0" if false, and "0x1" if true.
#[derive(Copy, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Parity(
    #[serde(serialize_with = "serialize_parity", deserialize_with = "deserialize_parity")] pub bool,
);

impl From<bool> for Parity {
    fn from(b: bool) -> Self {
        Self(b)
    }
}

impl From<U256> for Parity {
    fn from(value: U256) -> Self {
        match value {
            U256::ZERO => Self(false),
            _ => Self(true),
        }
    }
}

impl From<Parity> for U256 {
    fn from(p: Parity) -> Self {
        if p.0 {
            U256::from(1)
        } else {
            U256::ZERO
        }
    }
}

impl From<Parity> for u64 {
    fn from(p: Parity) -> Self {
        if p.0 {
            1
        } else {
            0
        }
    }
}

impl From<Parity> for u8 {
    fn from(value: Parity) -> Self {
        match value.0 {
            true => 1,
            false => 0,
        }
    }
}

impl Encodable for Parity {
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        let v = u8::from(*self);
        v.encode(out);
    }

    fn length(&self) -> usize {
        1
    }
}

impl Decodable for Parity {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let v = u8::decode(buf)?;
        Ok(Self(v != 0))
    }
}

fn serialize_parity<S>(parity: &bool, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(if *parity { "0x1" } else { "0x0" })
}

/// This implementation disallows serialization of the y parity bit that are not `"0x0"` or `"0x1"`.
fn deserialize_parity<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    match s.as_str() {
        "0x0" => Ok(false),
        "0x1" => Ok(true),
        _ => Err(serde::de::Error::custom(format!(
            "invalid parity value, parity should be either \"0x0\" or \"0x1\": {}",
            s
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn deserialize_without_parity() {
        let raw_signature_without_y_parity = r#"{
            "r":"0xc569c92f176a3be1a6352dd5005bfc751dcb32f57623dd2a23693e64bf4447b0",
            "s":"0x1a891b566d369e79b7a66eecab1e008831e22daa15f91a0a0cf4f9f28f47ee05",
            "v":"0x1"
        }"#;

        let signature: Signature = serde_json::from_str(raw_signature_without_y_parity).unwrap();
        let expected = Signature {
            r: U256::from_str("0xc569c92f176a3be1a6352dd5005bfc751dcb32f57623dd2a23693e64bf4447b0")
                .unwrap(),
            s: U256::from_str("0x1a891b566d369e79b7a66eecab1e008831e22daa15f91a0a0cf4f9f28f47ee05")
                .unwrap(),
            v: U256::from_str("1").unwrap(),
            y_parity: None,
        };

        assert_eq!(signature, expected);
    }

    #[test]
    fn deserialize_with_parity() {
        let raw_signature_with_y_parity = r#"{
            "r":"0xc569c92f176a3be1a6352dd5005bfc751dcb32f57623dd2a23693e64bf4447b0",
            "s":"0x1a891b566d369e79b7a66eecab1e008831e22daa15f91a0a0cf4f9f28f47ee05",
            "v":"0x1",
            "yParity": "0x1"
        }"#;

        let signature: Signature = serde_json::from_str(raw_signature_with_y_parity).unwrap();
        let expected = Signature {
            r: U256::from_str("0xc569c92f176a3be1a6352dd5005bfc751dcb32f57623dd2a23693e64bf4447b0")
                .unwrap(),
            s: U256::from_str("0x1a891b566d369e79b7a66eecab1e008831e22daa15f91a0a0cf4f9f28f47ee05")
                .unwrap(),
            v: U256::from_str("1").unwrap(),
            y_parity: Some(Parity(true)),
        };

        assert_eq!(signature, expected);
    }

    #[test]
    fn serialize_both_parity() {
        // this test should be removed if the struct moves to an enum based on tx type
        let signature = Signature {
            r: U256::from_str("0xc569c92f176a3be1a6352dd5005bfc751dcb32f57623dd2a23693e64bf4447b0")
                .unwrap(),
            s: U256::from_str("0x1a891b566d369e79b7a66eecab1e008831e22daa15f91a0a0cf4f9f28f47ee05")
                .unwrap(),
            v: U256::from_str("1").unwrap(),
            y_parity: Some(Parity(true)),
        };

        let serialized = serde_json::to_string(&signature).unwrap();
        assert_eq!(
            serialized,
            r#"{"r":"0xc569c92f176a3be1a6352dd5005bfc751dcb32f57623dd2a23693e64bf4447b0","s":"0x1a891b566d369e79b7a66eecab1e008831e22daa15f91a0a0cf4f9f28f47ee05","v":"0x1","yParity":"0x1"}"#
        );
    }

    #[test]
    fn serialize_v_only() {
        // this test should be removed if the struct moves to an enum based on tx type
        let signature = Signature {
            r: U256::from_str("0xc569c92f176a3be1a6352dd5005bfc751dcb32f57623dd2a23693e64bf4447b0")
                .unwrap(),
            s: U256::from_str("0x1a891b566d369e79b7a66eecab1e008831e22daa15f91a0a0cf4f9f28f47ee05")
                .unwrap(),
            v: U256::from_str("1").unwrap(),
            y_parity: None,
        };

        let expected = r#"{"r":"0xc569c92f176a3be1a6352dd5005bfc751dcb32f57623dd2a23693e64bf4447b0","s":"0x1a891b566d369e79b7a66eecab1e008831e22daa15f91a0a0cf4f9f28f47ee05","v":"0x1"}"#;

        let serialized = serde_json::to_string(&signature).unwrap();
        assert_eq!(serialized, expected);
    }

    #[test]
    fn serialize_parity() {
        let parity = Parity(true);
        let serialized = serde_json::to_string(&parity).unwrap();
        assert_eq!(serialized, r#""0x1""#);

        let parity = Parity(false);
        let serialized = serde_json::to_string(&parity).unwrap();
        assert_eq!(serialized, r#""0x0""#);
    }

    #[test]
    fn deserialize_parity() {
        let raw_parity = r#""0x1""#;
        let parity: Parity = serde_json::from_str(raw_parity).unwrap();
        assert_eq!(parity, Parity(true));

        let raw_parity = r#""0x0""#;
        let parity: Parity = serde_json::from_str(raw_parity).unwrap();
        assert_eq!(parity, Parity(false));
    }

    #[test]
    fn deserialize_parity_invalid() {
        let raw_parity = r#""0x2""#;
        let parity: Result<Parity, _> = serde_json::from_str(raw_parity);
        assert!(parity.is_err());

        let raw_parity = r#""0x""#;
        let parity: Result<Parity, _> = serde_json::from_str(raw_parity);
        assert!(parity.is_err());

        // In the spec this is defined as a uint, which requires 0x
        // yParity:
        // <https://github.com/ethereum/execution-apis/blob/8fcafbbc86257f6e61fddd9734148e38872a71c9/src/schemas/transaction.yaml#L157>
        //
        // uint:
        // <https://github.com/ethereum/execution-apis/blob/8fcafbbc86257f6e61fddd9734148e38872a71c9/src/schemas/base-types.yaml#L47>
        let raw_parity = r#""1""#;
        let parity: Result<Parity, _> = serde_json::from_str(raw_parity);
        assert!(parity.is_err());

        let raw_parity = r#""0""#;
        let parity: Result<Parity, _> = serde_json::from_str(raw_parity);
        assert!(parity.is_err());
    }
}
