use crate::entry::KeyType;
use alloy_primitives::Address;
use std::path::PathBuf;

/// Errors that can occur while reading or interpreting a Tempo keystore.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TempoSignerError {
    /// The keystore file does not exist.
    #[error("Tempo keystore not found at {}", path.display())]
    NotFound {
        /// The path that was attempted.
        path: PathBuf,
    },

    /// The keystore file could not be parsed as TOML.
    #[error("invalid TOML in {}: {source}", path.display())]
    BadToml {
        /// The keystore path.
        path: PathBuf,
        /// Underlying TOML deserialization error.
        #[source]
        source: toml::de::Error,
    },

    /// No usable Tempo key was found for the requested address.
    #[error("no usable Tempo key for {from}")]
    NoMatch {
        /// The address that was looked up.
        from: Address,
    },

    /// More than one usable Tempo key matched the requested address.
    #[error("multiple usable Tempo keys match {from}; cannot disambiguate")]
    Ambiguous {
        /// The address that was looked up.
        from: Address,
    },

    /// The matching Tempo key has expired.
    #[error("Tempo key expired (expiry={expiry})")]
    Expired {
        /// The expiry that was recorded for the key.
        expiry: u64,
    },

    /// The matching Tempo entry uses an unsupported `key_type` (e.g. passkey).
    #[error("unsupported Tempo key_type {kind:?} (only secp256k1 is supported)")]
    UnsupportedKeyType {
        /// The unsupported key type.
        kind: KeyType,
    },

    /// A hex-encoded field in the keystore could not be decoded.
    #[error("invalid hex in keystore field `{field}`")]
    BadHex {
        /// Name of the offending field (e.g. `"key"`, `"key_authorization"`).
        field: &'static str,
    },

    /// The decoded private key bytes are not a valid secp256k1 key.
    #[error("invalid secp256k1 private key in keystore")]
    BadKey,

    /// `$TEMPO_HOME` was not set and the user's home directory could not be located.
    #[error("$TEMPO_HOME unset and no home directory available")]
    NoHome,

    /// I/O error while reading the keystore.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// An env var contained invalid hex.
    #[error("env var {var} contains invalid hex")]
    BadEnvHex {
        /// The offending env var name.
        var: &'static str,
    },

    /// An env var contained an invalid Ethereum address.
    #[error("env var {var} contains invalid Ethereum address")]
    BadEnvAddress {
        /// The offending env var name.
        var: &'static str,
    },
}
