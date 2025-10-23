use crate::constants::{BASE64_REGEX, DATA_URI_REGEX, IPFS_HASH_REGEX, NETWORK_REGEX};
use std::collections::HashMap;

pub fn get_gateway(custom: Option<&str>, default_gateway: &str) {
    custom.map_or(default_gateway.trim_end_matches('/').to_string(), |c| {
        c.trim_end_matches('/').to_string()
    })
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
/// Parse an avatar URI into a canonical form or return an error.
pub fn parse_avatar_uri(
    uri: &str,
    gateway_urls: Option<HashMap<String, String>>,
) -> Result<UriItem, ParseAvatarError> {
    let uri = uri.trim();
    if uri.is_empty() {
        return Err(ParseAvatarError::EmptyInput);
    }

    // If it's a base64 blob already
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

#[derive(Debug)]
pub struct ParsedNftUri {
    pub namespace: String,
    pub contract_address: String,
    pub token_id: String,
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
}

pub fn parse_nft_uri(uri: &str) -> Result<ParsedNftUri, ParseNftError> {
    let mut uri = uri.to_lowercase();
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

    let reference = parts[0];
    let namespace_parts: Vec<&str> = reference.split(':').collect();

    if namespace_parts.len() != 2 {
        return Err(ParseNftError::InvalidReference);
    }

    // Extract parts — same as your original intent
    let namespace = namespace_parts[0].to_string();
    let contract_address = namespace_parts[1].to_string();

    let token_id = parts.last().ok_or(ParseNftError::MissingTokenId)?.to_string();

    Ok(ParsedNftUri { namespace, contract_address, token_id })
}
