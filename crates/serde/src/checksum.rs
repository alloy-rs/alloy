//! Serde functions for (de)serializing EIP-55 checksummed addresses.
//!
//! Can also be used for rejecting non checksummend addresses during deserialization.
//!
//! # Example
//! ```
//! use alloy_primitives::{address, Address};
//! use alloy_serde;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
//! pub struct Container {
//!     #[serde(with = "alloy_serde::checksum")]
//!     value: Address,
//! }
//!
//! let val = Container { value: address!("0xdadB0d80178819F2319190D340ce9A924f783711") };
//! let s = serde_json::to_string(&val).unwrap();
//! assert_eq!(s, "{\"value\":\"0xdadB0d80178819F2319190D340ce9A924f783711\"}");
//!
//! let deserialized: Container = serde_json::from_str(&s).unwrap();
//! assert_eq!(val, deserialized);
//!
//! let invalid = "{\"value\":\"0xdadb0d80178819F2319190D340ce9A924f783711\"}";
//! serde_json::from_str::<Container>(&invalid).unwrap_err();
//! ```

use crate::alloc::string::String;
use alloy_primitives::Address;
use serde::{Deserialize, Deserializer, Serializer};

/// Serialize an [Address] with EIP-55 checksum.
pub fn serialize<S>(value: &Address, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.collect_str(&value.to_checksum(None))
}

/// Deserialize an [Address] only if it has EIP-55 checksum.
pub fn deserialize<'de, D>(deserializer: D) -> Result<Address, D::Error>
where
    D: Deserializer<'de>,
{
    let str = String::deserialize(deserializer)?;
    Address::parse_checksummed(str, None).map_err(serde::de::Error::custom)
}
