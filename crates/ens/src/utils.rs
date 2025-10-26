use crate::constants::{BASE64_REGEX, DATA_URI_REGEX, IPFS_HASH_REGEX, NETWORK_REGEX};
use alloy_primitives::{ruint::aliases::U256, Address};
use base64::{engine::general_purpose, Engine as _};
use reqwest;
use serde_json::Value;
use std::{collections::HashMap, str::FromStr};
use thiserror::Error;

/// Fetch metadata from a URI and extract the avatar image URI
pub async fn get_metadata_avatar_uri(
    uri: &str,
    gateway_urls: Option<HashMap<String, String>>,
) -> Result<String, ParseAvatarError> {
    // Fetch the URI and parse as JSON
    let response = reqwest::get(uri).await.unwrap();

    let json_data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| ParseAvatarError::Other(format!("JSON parsing failed: {}", e)))?;

    // Extract image from JSON metadata
    let image_uri = get_json_image(&json_data).map_err(|e| {
        ParseAvatarError::Other(format!("Failed to extract image from metadata: {}", e))
    })?;

    // Parse the avatar URI
    let uri_item = parse_avatar_uri(&image_uri, gateway_urls)?;

    Ok(uri_item.uri)
}
pub fn get_gateway(custom: Option<&str>, default_gateway: &str) -> String {
    custom.map_or(default_gateway.trim_end_matches('/').to_string(), |c| {
        c.trim_end_matches('/').to_string()
    })
}

#[derive(Debug, Error)]
pub struct EnsAvatarNftInvalidMetadataError {
    pub data: Value,
}

impl std::fmt::Display for EnsAvatarNftInvalidMetadataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid metadata: {:?}", self.data)
    }
}

pub fn get_json_image(data: &Value) -> Result<String, EnsAvatarNftInvalidMetadataError> {
    // Check if data is an object
    let obj = match data.as_object() {
        Some(obj) => obj,
        None => return Err(EnsAvatarNftInvalidMetadataError { data: data.clone() }),
    };

    // Check if at least one of the required properties exists
    if !obj.contains_key("image")
        && !obj.contains_key("image_url")
        && !obj.contains_key("image_data")
    {
        return Err(EnsAvatarNftInvalidMetadataError { data: data.clone() });
    }

    // Return the first available image property as a string
    // If the property exists but isn't a string, return error
    for key in ["image", "image_url", "image_data"] {
        if let Some(value) = obj.get(key) {
            if let Some(string_value) = value.as_str() {
                return Ok(string_value.to_string());
            }
            // If property exists but isn't a string, that's invalid metadata
            return Err(EnsAvatarNftInvalidMetadataError { data: data.clone() });
        }
    }

    // This shouldn't be reachable given our validation above, but just in case
    Err(EnsAvatarNftInvalidMetadataError { data: data.clone() })
}
#[derive(Debug, Error)]
pub enum ParseAvatarError {
    #[error("empty input")]
    EmptyInput,
    #[error("failed to match network regex")]
    NetworkRegexNoCapture,
    #[error("missing target in ipfs/ipns/arweave URI")]
    MissingTarget,
    #[error("unsupported uri format: {0}")]
    Unsupported(String),
    #[error("other: {0}")]
    Other(String),
}

#[derive(Debug)]
pub struct UriItem {
    pub uri: String,
    pub is_on_chain: bool,
    pub is_encoded: bool,
}

pub fn decode_base64_json(resolved_nft_uri: &str) -> Result<String, Box<dyn std::error::Error>> {
    let base64_part = resolved_nft_uri.replace("data:application/json;base64,", "");
    let decoded_bytes = general_purpose::STANDARD.decode(base64_part)?;
    let decoded_string = String::from_utf8(decoded_bytes)?;
    Ok(decoded_string)
}
/// Parse an avatar URI into a canonical form or return an error.
pub fn parse_avatar_uri(
    uri: &str,
    gateway_urls: Option<HashMap<String, String>>,
) -> Result<UriItem, ParseAvatarError> {
    let uri = uri.trim();
    if uri.is_empty() {
        return Err(ParseAvatarError::EmptyInput);
    }

    if BASE64_REGEX.is_match(uri) {
        return Ok(UriItem { uri: uri.to_string(), is_on_chain: true, is_encoded: true });
    }

    // build gateways
    let gw_map = gateway_urls.unwrap_or_default();
    let ipfs_gateway = get_gateway(gw_map.get("ipfs").map(|s| s.as_str()), "https://ipfs.io");
    let arweave_gateway =
        get_gateway(gw_map.get("arweave").map(|s| s.as_str()), "https://arweave.net");

    // capture network-like URIs (ipfs:, ipns:, ar:, etc.)
    let caps_opt = NETWORK_REGEX.captures(uri);
    let caps = caps_opt.ok_or(ParseAvatarError::NetworkRegexNoCapture)?;
    let protocol = caps.name("protocol").map(|m| m.as_str());
    let subpath = caps.name("subpath").map(|m| m.as_str());
    let target = caps.name("target").map(|m| m.as_str());
    let subtarget = caps.name("subtarget").map(|m| m.as_str()).unwrap_or("");

    let is_ipns = protocol == Some("ipns:/") || subpath == Some("ipns/");
    let is_ipfs =
        protocol == Some("ipfs:/") || subpath == Some("ipfs/") || IPFS_HASH_REGEX.is_match(uri);

    // Plain HTTP(S) URLs not pointing to ipfs/ipns => possibly rewrite arweave gateway if configured
    if uri.starts_with("http") && !is_ipns && !is_ipfs {
        let replaced_uri = if let Some(gw) = gw_map.get("arweave") {
            uri.replace("https://arweave.net", gw)
        } else {
            uri.to_string()
        };
        return Ok(UriItem { uri: replaced_uri, is_on_chain: false, is_encoded: false });
    }

    // ipfs/ipns with a captured target
    if (is_ipns || is_ipfs) {
        let t = target.ok_or(ParseAvatarError::MissingTarget)?;
        let path_type = if is_ipns { "ipns" } else { "ipfs" };
        return Ok(UriItem {
            uri: format!("{}/{}/{}{}", ipfs_gateway, path_type, t, subtarget),
            is_on_chain: false,
            is_encoded: false,
        });
    }

    // arweave protocol
    if protocol == Some("ar:/") {
        let t = target.ok_or(ParseAvatarError::MissingTarget)?;
        return Ok(UriItem {
            uri: format!("{}/{}{}", arweave_gateway, t, subtarget),
            is_on_chain: false,
            is_encoded: false,
        });
    }

    // Strip possible data URI wrapper and check inline SVG / JSON / data: forms.
    // DATA_URI_REGEX is expected to be a Regex matching a leading data: wrapper.
    let mut parsed_uri = DATA_URI_REGEX.replace_all(uri, "").into_owned();

    if parsed_uri.starts_with("<svg") {
        // convert inline SVG to data:image/svg+xml;base64,...
        parsed_uri = format!("data:image/svg+xml;base64,{}", base64::encode(parsed_uri.as_bytes()));
    }

    if parsed_uri.starts_with("data:") || parsed_uri.starts_with('{') {
        return Ok(UriItem { uri: parsed_uri, is_on_chain: true, is_encoded: false });
    }

    // For the unlikely path we didn't already return: treat IPFS/IPNS one more time
    if is_ipfs {
        let t = target.ok_or(ParseAvatarError::MissingTarget)?;
        return Ok(UriItem {
            uri: format!("{}/ipfs/{}{}", ipfs_gateway, t, subtarget),
            is_on_chain: false,
            is_encoded: false,
        });
    }
    if is_ipns {
        let t = target.ok_or(ParseAvatarError::MissingTarget)?;
        return Ok(UriItem {
            uri: format!("{}/ipns/{}{}", ipfs_gateway, t, subtarget),
            is_on_chain: false,
            is_encoded: false,
        });
    }

    // Final attempt: if still looks like an inline SVG without data wrapper, base64 it
    parsed_uri = DATA_URI_REGEX.replace_all(uri, "").into_owned();
    if parsed_uri.starts_with("<svg") {
        parsed_uri = format!("data:image/svg+xml;base64,{}", base64::encode(parsed_uri.as_bytes()));
    }
    if parsed_uri.starts_with("data:") || parsed_uri.starts_with('{') {
        return Ok(UriItem { uri: parsed_uri, is_on_chain: true, is_encoded: false });
    }

    Err(ParseAvatarError::Unsupported(uri.to_string()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NftUriNamespace {
    ERC721,
    ERC1155,
}

impl NftUriNamespace {
    pub const ERC721_STR: &'static str = "erc721";
    pub const ERC1155_STR: &'static str = "erc1155";

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ERC721 => Self::ERC721_STR,
            Self::ERC1155 => Self::ERC1155_STR,
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            Self::ERC721_STR => Some(Self::ERC721),
            Self::ERC1155_STR => Some(Self::ERC1155),
            _ => None,
        }
    }
}
#[derive(Debug)]
pub struct ParsedNftUri {
    pub namespace: NftUriNamespace,
    pub contract_address: Address,
    pub token_id: U256,
}

#[derive(Debug, Error)]
pub enum ParseNftError {
    #[error("URI must start with 'did:nft:' or 'eip155:'")]
    InvalidPrefix,
    #[error("Malformed URI: missing required '/' separators")]
    MissingParts,
    #[error("Malformed reference: expected 'eip155:<chain>:<namespace>:<address>'")]
    InvalidReference,
    #[error("Missing token ID at the end of URI")]
    MissingTokenId,
    #[error("General parse failure")]
    GeneralFailure,
    #[error("Invalid token ID")]
    InvalidTokenId,
}

pub fn parse_nft_uri(uri: &str) -> Result<ParsedNftUri, ParseNftError> {
    let uri = uri.to_lowercase();
    let uri = uri.trim();

    // Normalize `did:nft:` format
    let uri = if uri.starts_with("did:nft:") {
        uri.replace("did:nft:", "").replace('_', "/")
    } else {
        uri.to_string()
    };

    // Must start with `eip155:`
    if !uri.starts_with("eip155:") {
        return Err(ParseNftError::InvalidPrefix);
    }

    let parts: Vec<&str> = uri.split('/').collect();
    if parts.len() < 2 {
        return Err(ParseNftError::MissingParts);
    }
    let reference = parts[1];
    println!("{:?}", parts);
    let namespace_parts: Vec<&str> = reference.split(':').collect();

    if namespace_parts.len() != 2 {
        return Err(ParseNftError::InvalidReference);
    }
    // Extract parts — same as your original intent
    let namespace = NftUriNamespace::from_str(namespace_parts[0]).unwrap();
    let contract_address = match Address::from_str(namespace_parts[1]) {
        Ok(address) => address,
        Err(_) => return Err(ParseNftError::InvalidReference),
    };
    let last_part = match namespace {
        NftUriNamespace::ERC721 => parts.last().ok_or(ParseNftError::MissingTokenId)?.to_string(),
        NftUriNamespace::ERC1155 => {
            let last = parts.last().ok_or(ParseNftError::MissingTokenId)?;
            format!("{:0>64}", last.strip_prefix("0x").unwrap_or(last))
        }
    };
    let token_id = match U256::from_str(&last_part) {
        Ok(id) => id,
        Err(_) => return Err(ParseNftError::InvalidTokenId),
    };

    Ok(ParsedNftUri { namespace, contract_address, token_id })
}
#[cfg(test)]
mod tests {
    use alloy_primitives::address;

    use super::*;
    #[test]
    fn test_parse_nft_uri_valid_cases() {
        // Test standard eip155 format
        let result = parse_nft_uri("eip155:1/erc1155:0x495f947276749ce646f68ac8c248420045cb7b5e/8112316025873927737505937898915153732580103913704334048512380490797008551937").unwrap();
        assert_eq!(result.namespace, NftUriNamespace::ERC1155);
        assert_eq!(result.contract_address, address!("0x495f947276749ce646f68ac8c248420045cb7b5e"));
        assert_eq!(
            result.token_id,
            U256::from_str(
                "8112316025873927737505937898915153732580103913704334048512380490797008551937u256"
            )
            .unwrap()
        );

        // // Test did:nft format with normalization (underscore to slash)
        let result =
            parse_nft_uri("did:nft:eip155:1/erc721:0x495f947276749ce646f68ac8c248420045cb7b5e/323")
                .unwrap();
        assert_eq!(result.namespace, NftUriNamespace::ERC721);
        assert_eq!(result.contract_address, address!("0x495f947276749ce646f68ac8c248420045cb7b5e"));
        assert_eq!(result.token_id, U256::from_str("323").unwrap());

        // // Test case insensitivity and whitespace trimming
        let result = parse_nft_uri(
            "   DID:NFT:EIP155:1/erc721:0x495f947276749ce646f68ac8c248420045cb7b5e/323",
        )
        .unwrap();
        assert_eq!(result.namespace, NftUriNamespace::ERC721);
        assert_eq!(result.contract_address, address!("0x495f947276749ce646f68ac8c248420045cb7b5e"));
        assert_eq!(result.token_id, U256::from_str("323").unwrap());
    }

    #[test]
    fn test_parse_nft_uri_error_cases() {
        // Invalid prefix
        assert!(matches!(parse_nft_uri("invalid:1/0x123/456"), Err(ParseNftError::InvalidPrefix)));
        assert!(matches!(parse_nft_uri("1/0x123/456"), Err(ParseNftError::InvalidPrefix)));
        assert!(matches!(parse_nft_uri(""), Err(ParseNftError::InvalidPrefix)));

        // Missing parts (no slash separator)
        assert!(matches!(parse_nft_uri("eip155:1"), Err(ParseNftError::MissingParts)));

        // Invalid reference format
        assert!(matches!(
            parse_nft_uri("eip155:1:extra/0x123/456"),
            Err(ParseNftError::InvalidReference)
        ));
    }
    #[test]
    fn test_get_gateway_with_custom() {
        // Custom gateway provided
        assert_eq!(
            get_gateway(Some("https://custom.com/"), "https://default.com"),
            "https://custom.com"
        );
        assert_eq!(
            get_gateway(Some("https://custom.com"), "https://default.com"),
            "https://custom.com"
        );

        // Custom gateway with multiple trailing slashes
        assert_eq!(
            get_gateway(Some("https://custom.com//"), "https://default.com"),
            "https://custom.com"
        );
    }
    #[test]
    fn test_parse_avatar_uri_valid_cases() {
        // HTTP URL (non-IPFS) - should pass through
        let result = parse_avatar_uri("https://example.com/image.png", None).unwrap();
        assert_eq!(result.uri, "https://example.com/image.png");
        assert!(!result.is_on_chain);
        assert!(!result.is_encoded);

        // Custom arweave gateway replacement
        let mut gateways = HashMap::new();
        gateways.insert("arweave".to_string(), "https://custom-arweave.com".to_string());
        let result = parse_avatar_uri("https://arweave.net/abc123", Some(gateways)).unwrap();
        assert_eq!(result.uri, "https://custom-arweave.com/abc123");

        // Data URI - should be on-chain
        let result = parse_avatar_uri("data:image/svg+xml;base64,PHN2Zz4=", None).unwrap();
        assert!(result.uri.starts_with("data:"));
        assert!(result.is_on_chain);
        assert!(!result.is_encoded);

        // JSON data - should be on-chain
        let result = parse_avatar_uri("{\"name\":\"test\"}", None).unwrap();
        assert_eq!(result.uri, "{\"name\":\"test\"}");
        assert!(result.is_on_chain);
        assert!(!result.is_encoded);
    }

    #[test]
    fn test_parse_avatar_uri_error_cases() {
        // Empty input
        assert!(matches!(parse_avatar_uri("", None), Err(ParseAvatarError::EmptyInput)));
        assert!(matches!(parse_avatar_uri("   ", None), Err(ParseAvatarError::EmptyInput)));

        // Unsupported format (assuming this doesn't match any regex patterns)
        let result = parse_avatar_uri("unsupported://example.com", None);
        assert!(matches!(
            result,
            Err(ParseAvatarError::NetworkRegexNoCapture) | Err(ParseAvatarError::Unsupported(_))
        ));

        // Another unsupported format
        let result = parse_avatar_uri("random-string-that-matches-no-pattern", None);
        assert!(matches!(
            result,
            Err(ParseAvatarError::NetworkRegexNoCapture) | Err(ParseAvatarError::Unsupported(_))
        ));
    }
}
