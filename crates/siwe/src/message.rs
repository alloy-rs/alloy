//! EIP-4361 Message implementation.

use crate::{TimeStamp, VerificationOpts};
use alloy_primitives::{Address, Signature};
use core::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};
use http::uri::Authority;
use iri_string::types::UriString;
use time::OffsetDateTime;

/// EIP-4361 version.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Version {
    /// Version 1.
    V1 = 1,
}

impl FromStr for Version {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "1" {
            Ok(Self::V1)
        } else {
            Err(ParseError::Format("invalid version"))
        }
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", *self as u64)
    }
}

/// EIP-4361 message.
///
/// # Example
///
/// ```
/// use alloy_siwe::Message;
///
/// let msg = r#"localhost:4361 wants you to sign in with your Ethereum account:
/// 0x6Da01670d8fc844e736095918bbE11fE8D564163
///
/// SIWE Notepad Example
///
/// URI: http://localhost:4361
/// Version: 1
/// Chain ID: 1
/// Nonce: kEWepMt9knR6lWJ6A
/// Issued At: 2021-12-07T18:28:18.807Z"#;
///
/// let message: Message = msg.parse().unwrap();
/// assert_eq!(message.chain_id, 1);
/// assert_eq!(message.nonce, "kEWepMt9knR6lWJ6A");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    /// RFC 3986 authority requesting the signing.
    pub domain: Authority,
    /// Ethereum address performing the signing (EIP-55 checksum format).
    pub address: Address,
    /// Human-readable ASCII assertion that the user will sign.
    pub statement: Option<String>,
    /// RFC 3986 URI referring to the resource that is the subject of signing.
    pub uri: UriString,
    /// Current version of the message (must be 1).
    pub version: Version,
    /// EIP-155 Chain ID to which the session is bound.
    pub chain_id: u64,
    /// Randomized token used to prevent replay attacks (min 8 alphanumeric chars).
    pub nonce: String,
    /// ISO 8601 datetime when the message was created.
    pub issued_at: TimeStamp,
    /// ISO 8601 datetime when the message expires.
    pub expiration_time: Option<TimeStamp>,
    /// ISO 8601 datetime when the message becomes valid.
    pub not_before: Option<TimeStamp>,
    /// System-specific identifier for the sign-in request.
    pub request_id: Option<String>,
    /// List of URIs the user wishes to have resolved.
    pub resources: Vec<UriString>,
}

impl Display for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}{}", &self.domain, PREAMBLE)?;
        writeln!(f, "{}", self.address.to_checksum(None))?;
        writeln!(f)?;
        if let Some(statement) = &self.statement {
            writeln!(f, "{statement}")?;
        }
        writeln!(f)?;
        writeln!(f, "{URI_TAG}{}", &self.uri)?;
        writeln!(f, "{VERSION_TAG}{}", self.version)?;
        writeln!(f, "{CHAIN_TAG}{}", &self.chain_id)?;
        writeln!(f, "{NONCE_TAG}{}", &self.nonce)?;
        write!(f, "{IAT_TAG}{}", &self.issued_at)?;
        if let Some(exp) = &self.expiration_time {
            write!(f, "\n{EXP_TAG}{exp}")?;
        }
        if let Some(nbf) = &self.not_before {
            write!(f, "\n{NBF_TAG}{nbf}")?;
        }
        if let Some(rid) = &self.request_id {
            write!(f, "\n{RID_TAG}{rid}")?;
        }
        if !self.resources.is_empty() {
            write!(f, "\n{RES_TAG}")?;
            for res in &self.resources {
                write!(f, "\n- {res}")?;
            }
        }
        Ok(())
    }
}

impl FromStr for Message {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut lines = s.split('\n');

        // Parse domain from preamble
        let domain = lines
            .next()
            .and_then(|preamble| preamble.strip_suffix(PREAMBLE))
            .map(Authority::from_str)
            .ok_or(ParseError::Format("missing preamble"))??;

        // Parse address (must be EIP-55 checksummed)
        let address_str = tagged(ADDR_TAG, lines.next())?;
        if !is_checksummed(address_str) {
            return Err(ParseError::Format("address not in EIP-55 format"));
        }
        let address: Address = address_str.parse()?;

        // Skip blank line
        lines.next();

        // Parse optional statement
        let statement = match lines.next() {
            None => return Err(ParseError::Format("unexpected end after address")),
            Some("") => None,
            Some(s) => {
                lines.next(); // Skip blank line after statement
                Some(s.to_string())
            }
        };

        // Parse required fields
        let uri = parse_line(URI_TAG, lines.next())?;
        let version = parse_line(VERSION_TAG, lines.next())?;
        let chain_id = parse_line(CHAIN_TAG, lines.next())?;
        let nonce: String = parse_line(NONCE_TAG, lines.next())?;
        if nonce.len() < 8 {
            return Err(ParseError::Format("nonce must be at least 8 characters"));
        }
        let issued_at: TimeStamp = tagged(IAT_TAG, lines.next())?.parse()?;

        // Parse optional fields
        let mut line = lines.next();

        let expiration_time = match tag_optional(EXP_TAG, line)? {
            Some(exp) => {
                line = lines.next();
                Some(exp.parse()?)
            }
            None => None,
        };

        let not_before = match tag_optional(NBF_TAG, line)? {
            Some(nbf) => {
                line = lines.next();
                Some(nbf.parse()?)
            }
            None => None,
        };

        let request_id = match tag_optional(RID_TAG, line)? {
            Some(rid) => {
                line = lines.next();
                Some(rid.into())
            }
            None => None,
        };

        let resources = match line {
            Some(RES_TAG) => lines.map(|s| parse_line("- ", Some(s))).collect(),
            Some(_) => Err(ParseError::Format("unexpected content")),
            None => Ok(vec![]),
        }?;

        Ok(Self {
            domain,
            address,
            statement,
            uri,
            version,
            chain_id,
            nonce,
            issued_at,
            expiration_time,
            not_before,
            request_id,
            resources,
        })
    }
}

impl Message {
    /// Verify the message signature using EIP-191 personal sign.
    ///
    /// Returns the recovered address on success.
    ///
    /// # Example
    ///
    /// ```
    /// use alloy_primitives::{hex, Signature};
    /// use alloy_siwe::Message;
    ///
    /// let msg: Message = r#"localhost:4361 wants you to sign in with your Ethereum account:
    /// 0x6Da01670d8fc844e736095918bbE11fE8D564163
    ///
    /// SIWE Notepad Example
    ///
    /// URI: http://localhost:4361
    /// Version: 1
    /// Chain ID: 1
    /// Nonce: kEWepMt9knR6lWJ6A
    /// Issued At: 2021-12-07T18:28:18.807Z"#.parse().unwrap();
    ///
    /// let sig_bytes = hex!("6228b3ecd7bf2df018183aeab6b6f1db1e9f4e3cbe24560404112e25363540eb679934908143224d746bbb5e1aa65ab435684081f4dbb74a0fec57f98f40f5051c");
    /// let signature = Signature::try_from(&sig_bytes[..]).unwrap();
    ///
    /// let recovered = msg.verify_eip191(&signature).unwrap();
    /// assert_eq!(recovered, msg.address);
    /// ```
    pub fn verify_eip191(&self, signature: &Signature) -> Result<Address, VerificationError> {
        let message_str = self.to_string();
        let recovered = signature.recover_address_from_msg(message_str.as_bytes())?;

        if recovered != self.address {
            return Err(VerificationError::AddressMismatch { expected: self.address, recovered });
        }

        Ok(recovered)
    }

    /// Validates time constraints at a specific point in time.
    ///
    /// Returns `true` if:
    /// - `not_before` is `None` OR `t >= not_before`
    /// - `expiration_time` is `None` OR `t < expiration_time`
    #[must_use]
    pub fn valid_at(&self, t: &OffsetDateTime) -> bool {
        let not_before_ok = self.not_before.as_ref().map(|nbf| nbf < t).unwrap_or(true);
        let not_expired = self.expiration_time.as_ref().map(|exp| exp >= t).unwrap_or(true);
        not_before_ok && not_expired
    }

    /// Verify the message with additional validation options.
    ///
    /// This validates:
    /// - Time constraints if `opts.timestamp` is provided
    /// - Domain matching if `opts.domain` is provided
    /// - Nonce matching if `opts.nonce` is provided
    /// - Signature (EIP-191)
    ///
    /// For time-sensitive validation, always provide `opts.timestamp`.
    pub fn verify(
        &self,
        signature: &Signature,
        opts: &VerificationOpts,
    ) -> Result<Address, VerificationError> {
        // Validate time (only if timestamp provided)
        if let Some(t) = &opts.timestamp {
            if !self.valid_at(t) {
                return Err(VerificationError::Time);
            }
        }

        // Validate domain
        if let Some(expected_domain) = &opts.domain {
            if *expected_domain != self.domain {
                return Err(VerificationError::DomainMismatch);
            }
        }

        // Validate nonce
        if let Some(expected_nonce) = &opts.nonce {
            if *expected_nonce != self.nonce {
                return Err(VerificationError::NonceMismatch);
            }
        }

        // Verify signature
        self.verify_eip191(signature)
    }
}

/// Error parsing a SIWE message.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// Invalid domain.
    #[error("invalid domain: {0}")]
    Domain(#[from] http::uri::InvalidUri),
    /// Invalid address.
    #[error("invalid address: {0}")]
    Address(#[from] alloy_primitives::hex::FromHexError),
    /// Invalid URI.
    #[error("invalid URI: {0}")]
    Uri(#[from] iri_string::validate::Error),
    /// Invalid timestamp.
    #[error("invalid timestamp: {0}")]
    TimeStamp(#[from] time::error::Parse),
    /// Invalid chain ID.
    #[error("invalid chain ID: {0}")]
    ChainId(#[from] std::num::ParseIntError),
    /// Format error.
    #[error("format error: {0}")]
    Format(&'static str),
}

impl From<core::convert::Infallible> for ParseError {
    fn from(x: core::convert::Infallible) -> Self {
        match x {}
    }
}

/// Error verifying a SIWE message.
#[derive(Debug, thiserror::Error)]
pub enum VerificationError {
    /// Signature error.
    #[error("signature error: {0}")]
    Signature(#[from] alloy_primitives::SignatureError),
    /// Recovered address does not match message address.
    #[error("address mismatch: expected {expected}, recovered {recovered}")]
    AddressMismatch {
        /// Expected address from the message.
        expected: Address,
        /// Recovered address from signature.
        recovered: Address,
    },
    /// Message is not valid at the current/specified time.
    #[error("message is not currently valid")]
    Time,
    /// Domain does not match expected value.
    #[error("domain mismatch")]
    DomainMismatch,
    /// Nonce does not match expected value.
    #[error("nonce mismatch")]
    NonceMismatch,
    /// EIP-1271 contract verification failed.
    #[cfg(feature = "contract")]
    #[error("contract verification failed: {0}")]
    Contract(#[from] alloy_contract::Error),
    /// Contract is not EIP-1271 compliant.
    #[cfg(feature = "contract")]
    #[error("contract is not EIP-1271 compliant")]
    Eip1271NonCompliant,
}

// Parsing helpers

const PREAMBLE: &str = " wants you to sign in with your Ethereum account:";
const ADDR_TAG: &str = "0x";
const URI_TAG: &str = "URI: ";
const VERSION_TAG: &str = "Version: ";
const CHAIN_TAG: &str = "Chain ID: ";
const NONCE_TAG: &str = "Nonce: ";
const IAT_TAG: &str = "Issued At: ";
const EXP_TAG: &str = "Expiration Time: ";
const NBF_TAG: &str = "Not Before: ";
const RID_TAG: &str = "Request ID: ";
const RES_TAG: &str = "Resources:";

fn tagged<'a>(tag: &'static str, line: Option<&'a str>) -> Result<&'a str, ParseError> {
    line.and_then(|l| l.strip_prefix(tag))
        .ok_or(ParseError::Format(tag))
}

fn parse_line<S: FromStr<Err = E>, E: Into<ParseError>>(
    tag: &'static str,
    line: Option<&str>,
) -> Result<S, ParseError> {
    tagged(tag, line).and_then(|s| S::from_str(s).map_err(Into::into))
}

fn tag_optional<'a>(
    tag: &'static str,
    line: Option<&'a str>,
) -> Result<Option<&'a str>, ParseError> {
    match tagged(tag, line).map(Some) {
        Err(ParseError::Format(t)) if t == tag => Ok(None),
        r => r,
    }
}

/// Check if an address string is EIP-55 checksummed.
fn is_checksummed(addr: &str) -> bool {
    // Parse and re-checksum, then compare
    match addr.parse::<Address>() {
        Ok(parsed) => parsed.to_checksum(None) == format!("0x{addr}"),
        Err(_) => false,
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use super::*;
    use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

    impl Serialize for Message {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(&self.to_string())
        }
    }

    impl<'de> Deserialize<'de> for Message {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let s = String::deserialize(deserializer)?;
            s.parse().map_err(de::Error::custom)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::hex;

    const TEST_MESSAGE: &str = r#"localhost:4361 wants you to sign in with your Ethereum account:
0x6Da01670d8fc844e736095918bbE11fE8D564163

SIWE Notepad Example

URI: http://localhost:4361
Version: 1
Chain ID: 1
Nonce: kEWepMt9knR6lWJ6A
Issued At: 2021-12-07T18:28:18.807Z"#;

    #[test]
    fn test_parse_message() {
        let message: Message = TEST_MESSAGE.parse().unwrap();
        assert_eq!(message.domain.to_string(), "localhost:4361");
        assert_eq!(
            message.address,
            "0x6Da01670d8fc844e736095918bbE11fE8D564163".parse::<Address>().unwrap()
        );
        assert_eq!(message.statement, Some("SIWE Notepad Example".to_string()));
        assert_eq!(message.uri.as_str(), "http://localhost:4361");
        assert_eq!(message.version, Version::V1);
        assert_eq!(message.chain_id, 1);
        assert_eq!(message.nonce, "kEWepMt9knR6lWJ6A");
    }

    #[test]
    fn test_roundtrip() {
        let message: Message = TEST_MESSAGE.parse().unwrap();
        let serialized = message.to_string();
        let reparsed: Message = serialized.parse().unwrap();
        assert_eq!(message, reparsed);
    }

    #[test]
    fn test_verify_eip191() {
        let message: Message = TEST_MESSAGE.parse().unwrap();
        let sig_bytes = hex!("6228b3ecd7bf2df018183aeab6b6f1db1e9f4e3cbe24560404112e25363540eb679934908143224d746bbb5e1aa65ab435684081f4dbb74a0fec57f98f40f5051c");
        let signature = Signature::try_from(&sig_bytes[..]).unwrap();

        let recovered = message.verify_eip191(&signature).unwrap();
        assert_eq!(recovered, message.address);
    }

    #[test]
    fn test_verify_eip191_invalid() {
        let message: Message = TEST_MESSAGE.parse().unwrap();
        // Modified signature (first byte changed)
        let sig_bytes = hex!("7228b3ecd7bf2df018183aeab6b6f1db1e9f4e3cbe24560404112e25363540eb679934908143224d746bbb5e1aa65ab435684081f4dbb74a0fec57f98f40f5051c");
        let signature = Signature::try_from(&sig_bytes[..]).unwrap();

        assert!(message.verify_eip191(&signature).is_err());
    }

    #[test]
    fn test_parse_no_statement() {
        let msg = r#"service.org wants you to sign in with your Ethereum account:
0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2


URI: https://service.org/login
Version: 1
Chain ID: 1
Nonce: 32891756
Issued At: 2021-09-30T16:25:24Z"#;

        let message: Message = msg.parse().unwrap();
        assert!(message.statement.is_none());
    }

    #[test]
    fn test_parse_with_resources() {
        let msg = r#"service.org wants you to sign in with your Ethereum account:
0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2

I accept the ServiceOrg Terms of Service: https://service.org/tos

URI: https://service.org/login
Version: 1
Chain ID: 1
Nonce: 32891756
Issued At: 2021-09-30T16:25:24Z
Resources:
- ipfs://bafybeiemxf5abjwjbikoz4mc3a3dla6ual3jsgpdr4cjr3oz3evfyavhwq/
- https://example.com/my-web2-claim.json"#;

        let message: Message = msg.parse().unwrap();
        assert_eq!(message.resources.len(), 2);
    }

    #[test]
    fn test_invalid_nonce_too_short() {
        let msg = r#"service.org wants you to sign in with your Ethereum account:
0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2


URI: https://service.org/login
Version: 1
Chain ID: 1
Nonce: short
Issued At: 2021-09-30T16:25:24Z"#;

        assert!(msg.parse::<Message>().is_err());
    }

    #[test]
    fn test_invalid_address_not_checksummed() {
        let msg = r#"service.org wants you to sign in with your Ethereum account:
0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2


URI: https://service.org/login
Version: 1
Chain ID: 1
Nonce: 32891756
Issued At: 2021-09-30T16:25:24Z"#;

        let result = msg.parse::<Message>();
        assert!(result.is_err());
    }
}
