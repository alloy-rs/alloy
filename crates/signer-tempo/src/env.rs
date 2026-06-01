use crate::{
    error::TempoSignerError,
    keystore::parse_signer,
    lookup::{TempoAccessKey, TempoLookup},
};
use alloy_primitives::Address;
use std::str::FromStr;

/// Hex-encoded private key for Direct-mode signing.
pub const ENV_PRIVATE_KEY: &str = "TEMPO_PRIVATE_KEY";
/// Hex-encoded access key for Keychain-mode signing.
pub const ENV_ACCESS_KEY: &str = "TEMPO_ACCESS_KEY";
/// Smart-wallet/root address authorizing `TEMPO_ACCESS_KEY`. When absent (or
/// equal to the access-key address), the access key is treated as Direct.
pub const ENV_ROOT_ACCOUNT: &str = "TEMPO_ROOT_ACCOUNT";

/// Resolve a Tempo signer from environment variables.
///
/// `TEMPO_ACCESS_KEY` (with optional `TEMPO_ROOT_ACCOUNT`) takes precedence
/// over `TEMPO_PRIVATE_KEY`. Returns `Ok(None)` when no relevant env var is set.
pub fn tempo_signer_from_env() -> Result<Option<TempoLookup>, TempoSignerError> {
    if let Ok(val) = std::env::var(ENV_ACCESS_KEY) {
        let signer = parse_signer(val.trim()).map_err(|e| match e {
            TempoSignerError::BadHex { .. } => TempoSignerError::BadEnvHex { var: ENV_ACCESS_KEY },
            other => other,
        })?;
        let signer_addr = signer.address();

        let wallet_address = match std::env::var(ENV_ROOT_ACCOUNT) {
            Ok(s) => Address::from_str(s.trim())
                .map_err(|_| TempoSignerError::BadEnvAddress { var: ENV_ROOT_ACCOUNT })?,
            Err(_) => signer_addr,
        };

        if wallet_address == signer_addr {
            return Ok(Some(TempoLookup::Direct(signer)));
        }
        return Ok(Some(TempoLookup::Keychain(
            signer,
            TempoAccessKey {
                wallet_address,
                key_address: signer_addr,
                key_authorization: None,
                chain_id: 0,
                expiry: None,
            },
        )));
    }

    if let Ok(val) = std::env::var(ENV_PRIVATE_KEY) {
        let signer = parse_signer(val.trim()).map_err(|e| match e {
            TempoSignerError::BadHex { .. } => TempoSignerError::BadEnvHex { var: ENV_PRIVATE_KEY },
            other => other,
        })?;
        return Ok(Some(TempoLookup::Direct(signer)));
    }

    Ok(None)
}
