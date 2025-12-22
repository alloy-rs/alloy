//! Winnow-based parser for EIP-4361 SIWE messages.
//!
//! This parser follows the ABNF grammar defined in EIP-4361:
//! <https://eips.ethereum.org/EIPS/eip-4361>

use alloc::{format, string::String, string::ToString, vec::Vec};
use winnow::{
    ascii::{digit1, line_ending},
    combinator::{opt, preceded, terminated},
    error::{ContextError, StrContext},
    token::{take_till, take_while},
    ModalResult, Parser,
};

use crate::{Message, ParseError, TimeStamp, Version};
use alloy_primitives::Address;
use http::uri::Authority;
use iri_string::types::UriString;

/// Winnow parser result type alias.
type PResult<T> = ModalResult<T, ContextError>;

/// Parse a complete SIWE message from a string.
pub(crate) fn parse_message(input: &str) -> Result<Message, ParseError> {
    message.parse(input).map_err(|e| ParseError::Format(format!("{e}")))
}

/// Root parser for EIP-4361 message.
fn message(input: &mut &str) -> PResult<Message> {
    let domain = domain_line.parse_next(input)?;
    let address = address_line.parse_next(input)?;

    // Empty line after address
    line_ending.parse_next(input)?;

    // Optional statement followed by empty line, or just empty line
    let statement = statement_section.parse_next(input)?;

    // Required fields
    let uri = uri_field.parse_next(input)?;
    let version = version_field.parse_next(input)?;
    let chain_id = chain_id_field.parse_next(input)?;
    let nonce = nonce_field.parse_next(input)?;
    let issued_at = issued_at_field.parse_next(input)?;

    // Optional fields
    let expiration_time = opt(expiration_time_field).parse_next(input)?;
    let not_before = opt(not_before_field).parse_next(input)?;
    let request_id = opt(request_id_field).parse_next(input)?;
    let resources = resources_section.parse_next(input)?;

    Ok(Message {
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

// ============================================================================
// Domain and Preamble
// ============================================================================

const PREAMBLE: &str = " wants you to sign in with your Ethereum account:";

/// Parse: `domain " wants you to sign in with your Ethereum account:" LF`
fn domain_line(input: &mut &str) -> PResult<Authority> {
    let domain_str = terminated(take_till(1.., |c| c == ' '), PREAMBLE)
        .context(StrContext::Label("domain"))
        .parse_next(input)?;
    line_ending.parse_next(input)?;

    domain_str
        .parse::<Authority>()
        .map_err(|_| winnow::error::ErrMode::Cut(ContextError::new()))
}

// ============================================================================
// Address
// ============================================================================

/// Parse: `"0x" 40HEXDIG LF`
fn address_line(input: &mut &str) -> PResult<Address> {
    // Match "0x" prefix and 40 hex digits, capturing only the address (not newline)
    let (_, addr_str) = ("0x", take_while(40..=40, |c: char| c.is_ascii_hexdigit()))
        .with_taken()
        .context(StrContext::Label("address"))
        .parse_next(input)?;
    line_ending.parse_next(input)?;

    // Validate EIP-55 checksum
    let addr: Address = addr_str
        .parse()
        .map_err(|_| winnow::error::ErrMode::Cut(ContextError::new()))?;

    if addr.to_checksum(None) != addr_str {
        return Err(winnow::error::ErrMode::Cut(ContextError::new()));
    }

    Ok(addr)
}

// ============================================================================
// Statement (optional)
// ============================================================================

/// Parse optional statement section: `[ statement LF ] LF`
fn statement_section(input: &mut &str) -> PResult<Option<String>> {
    // Check if we have an empty line (no statement)
    if input.starts_with('\n') || input.starts_with("\r\n") {
        line_ending.parse_next(input)?;
        return Ok(None);
    }

    // Parse statement until newline
    let stmt = take_till(1.., |c| c == '\n' || c == '\r')
        .context(StrContext::Label("statement"))
        .parse_next(input)?;
    line_ending.parse_next(input)?;

    // Empty line after statement
    line_ending.parse_next(input)?;

    Ok(Some(stmt.to_string()))
}

// ============================================================================
// Required Fields
// ============================================================================

/// Parse: `"URI: " uri LF`
fn uri_field(input: &mut &str) -> PResult<UriString> {
    let uri_str = preceded("URI: ", take_till(1.., |c| c == '\n' || c == '\r'))
        .context(StrContext::Label("URI"))
        .parse_next(input)?;
    line_ending.parse_next(input)?;

    uri_str
        .parse::<UriString>()
        .map_err(|_| winnow::error::ErrMode::Cut(ContextError::new()))
}

/// Parse: `"Version: " version LF`
fn version_field(input: &mut &str) -> PResult<Version> {
    preceded("Version: ", "1")
        .context(StrContext::Label("version"))
        .parse_next(input)?;
    line_ending.parse_next(input)?;
    Ok(Version::V1)
}

/// Parse: `"Chain ID: " 1*DIGIT LF`
fn chain_id_field(input: &mut &str) -> PResult<u64> {
    let chain_str = preceded("Chain ID: ", digit1)
        .context(StrContext::Label("chain ID"))
        .parse_next(input)?;
    line_ending.parse_next(input)?;

    chain_str
        .parse::<u64>()
        .map_err(|_| winnow::error::ErrMode::Cut(ContextError::new()))
}

/// Parse: `"Nonce: " 8*(ALPHA / DIGIT) LF`
fn nonce_field(input: &mut &str) -> PResult<String> {
    let nonce_str = preceded(
        "Nonce: ",
        take_while(8.., |c: char| c.is_ascii_alphanumeric()),
    )
    .context(StrContext::Label("nonce"))
    .parse_next(input)?;
    line_ending.parse_next(input)?;

    Ok(nonce_str.to_string())
}

/// Parse: `"Issued At: " date-time`
fn issued_at_field(input: &mut &str) -> PResult<TimeStamp> {
    let ts_str = preceded("Issued At: ", take_till(1.., |c| c == '\n' || c == '\r'))
        .context(StrContext::Label("issued at"))
        .parse_next(input)?;

    ts_str
        .parse::<TimeStamp>()
        .map_err(|_| winnow::error::ErrMode::Cut(ContextError::new()))
}

// ============================================================================
// Optional Fields
// ============================================================================

/// Parse: `LF "Expiration Time: " date-time`
fn expiration_time_field(input: &mut &str) -> PResult<TimeStamp> {
    line_ending.parse_next(input)?;
    let ts_str = preceded(
        "Expiration Time: ",
        take_till(1.., |c| c == '\n' || c == '\r'),
    )
    .context(StrContext::Label("expiration time"))
    .parse_next(input)?;

    ts_str
        .parse::<TimeStamp>()
        .map_err(|_| winnow::error::ErrMode::Cut(ContextError::new()))
}

/// Parse: `LF "Not Before: " date-time`
fn not_before_field(input: &mut &str) -> PResult<TimeStamp> {
    line_ending.parse_next(input)?;
    let ts_str = preceded("Not Before: ", take_till(1.., |c| c == '\n' || c == '\r'))
        .context(StrContext::Label("not before"))
        .parse_next(input)?;

    ts_str
        .parse::<TimeStamp>()
        .map_err(|_| winnow::error::ErrMode::Cut(ContextError::new()))
}

/// Parse: `LF "Request ID: " *pchar`
fn request_id_field(input: &mut &str) -> PResult<String> {
    line_ending.parse_next(input)?;
    let rid_str = preceded("Request ID: ", take_till(0.., |c| c == '\n' || c == '\r'))
        .context(StrContext::Label("request ID"))
        .parse_next(input)?;

    Ok(rid_str.to_string())
}

// ============================================================================
// Resources
// ============================================================================

/// Parse: `[ LF "Resources:" *( LF "- " URI ) ]`
fn resources_section(input: &mut &str) -> PResult<Vec<UriString>> {
    // Check if resources section exists
    let has_resources = opt((line_ending, "Resources:")).parse_next(input)?;

    if has_resources.is_none() {
        return Ok(Vec::new());
    }

    // Parse each resource line
    let mut resources = Vec::new();
    while opt(line_ending).parse_next(input)?.is_some() {
        if input.is_empty() || !input.starts_with("- ") {
            break;
        }
        let uri_str = preceded("- ", take_till(1.., |c| c == '\n' || c == '\r'))
            .context(StrContext::Label("resource URI"))
            .parse_next(input)?;

        let uri = uri_str
            .parse::<UriString>()
            .map_err(|_| winnow::error::ErrMode::Cut(ContextError::new()))?;
        resources.push(uri);
    }

    Ok(resources)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_domain_line() {
        let mut input = "localhost:4361 wants you to sign in with your Ethereum account:\n";
        let domain = domain_line(&mut input).unwrap();
        assert_eq!(domain.to_string(), "localhost:4361");
        assert!(input.is_empty());
    }

    #[test]
    fn test_parse_address_line() {
        let mut input = "0x6Da01670d8fc844e736095918bbE11fE8D564163\n";
        let addr = address_line(&mut input).unwrap();
        assert_eq!(
            addr,
            "0x6Da01670d8fc844e736095918bbE11fE8D564163"
                .parse::<Address>()
                .unwrap()
        );
    }

    #[test]
    fn test_parse_statement_with_content() {
        let mut input = "SIWE Notepad Example\n\n";
        let stmt = statement_section(&mut input).unwrap();
        assert_eq!(stmt, Some("SIWE Notepad Example".to_string()));
    }

    #[test]
    fn test_parse_statement_empty() {
        let mut input = "\n";
        let stmt = statement_section(&mut input).unwrap();
        assert_eq!(stmt, None);
    }

    #[test]
    fn test_parse_full_message() {
        let input = r#"localhost:4361 wants you to sign in with your Ethereum account:
0x6Da01670d8fc844e736095918bbE11fE8D564163

SIWE Notepad Example

URI: http://localhost:4361
Version: 1
Chain ID: 1
Nonce: kEWepMt9knR6lWJ6A
Issued At: 2021-12-07T18:28:18.807Z"#;

        let msg = parse_message(input).unwrap();
        assert_eq!(msg.domain.to_string(), "localhost:4361");
        assert_eq!(msg.chain_id, 1);
        assert_eq!(msg.nonce, "kEWepMt9knR6lWJ6A");
        assert_eq!(msg.statement, Some("SIWE Notepad Example".to_string()));
    }

    #[test]
    fn test_parse_message_with_resources() {
        let input = r#"service.org wants you to sign in with your Ethereum account:
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

        let msg = parse_message(input).unwrap();
        assert_eq!(msg.resources.len(), 2);
    }

    #[test]
    fn test_parse_message_no_statement() {
        let input = r#"service.org wants you to sign in with your Ethereum account:
0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2


URI: https://service.org/login
Version: 1
Chain ID: 1
Nonce: 32891756
Issued At: 2021-09-30T16:25:24Z"#;

        let msg = parse_message(input).unwrap();
        assert!(msg.statement.is_none());
    }
}
