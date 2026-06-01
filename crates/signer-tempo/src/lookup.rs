use alloy_primitives::{Address, Bytes};
use alloy_signer_local::PrivateKeySigner;
use std::fmt;

/// Side-channel metadata for Keychain-mode signing.
///
/// `key_authorization` is opaque RLP-encoded `SignedKeyAuthorization` bytes;
/// consumers decode it with `tempo-primitives` if a typed form is needed.
#[derive(Clone)]
pub struct TempoAccessKey {
    /// Smart-wallet (root) address. The transaction `from`.
    pub wallet_address: Address,
    /// Address derived from the access key.
    pub key_address: Address,
    /// RLP-encoded `SignedKeyAuthorization`.
    pub key_authorization: Option<Bytes>,
    /// Chain ID the access key was authorized on.
    pub chain_id: u64,
    /// Expiry as a unix timestamp, if any.
    pub expiry: Option<u64>,
}

impl fmt::Debug for TempoAccessKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TempoAccessKey")
            .field("wallet_address", &self.wallet_address)
            .field("key_address", &self.key_address)
            .field("has_key_authorization", &self.key_authorization.is_some())
            .field("chain_id", &self.chain_id)
            .field("expiry", &self.expiry)
            .finish()
    }
}

/// Result of a successful Tempo keystore lookup.
///
/// In [`Self::Direct`] the signer is the wallet itself; in [`Self::Keychain`]
/// the signer is an access key and the transaction `from` lives in the
/// accompanying [`TempoAccessKey`].
#[non_exhaustive]
pub enum TempoLookup {
    /// EOA mode: the signer is the wallet.
    Direct(PrivateKeySigner),
    /// Keychain mode: the signer is an access key authorized by a wallet.
    Keychain(PrivateKeySigner, TempoAccessKey),
}

impl TempoLookup {
    /// Borrow the underlying signer.
    pub const fn signer(&self) -> &PrivateKeySigner {
        match self {
            Self::Direct(s) | Self::Keychain(s, _) => s,
        }
    }

    /// Consume the lookup and return the underlying signer.
    pub fn into_signer(self) -> PrivateKeySigner {
        match self {
            Self::Direct(s) | Self::Keychain(s, _) => s,
        }
    }

    /// Borrow the access-key metadata, if this is a Keychain-mode lookup.
    pub const fn access_key(&self) -> Option<&TempoAccessKey> {
        match self {
            Self::Direct(_) => None,
            Self::Keychain(_, ak) => Some(ak),
        }
    }

    /// Address to use as the transaction `from`: the signer for `Direct`,
    /// the wallet (root) address for `Keychain`.
    pub const fn from_address(&self) -> Address {
        match self {
            Self::Direct(s) => s.address(),
            Self::Keychain(_, ak) => ak.wallet_address,
        }
    }

    /// Returns `true` if this is a Direct-mode lookup.
    pub const fn is_direct(&self) -> bool {
        matches!(self, Self::Direct(_))
    }

    /// Returns `true` if this is a Keychain-mode lookup.
    pub const fn is_keychain(&self) -> bool {
        matches!(self, Self::Keychain(_, _))
    }
}

impl fmt::Debug for TempoLookup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Direct(s) => {
                f.debug_struct("TempoLookup::Direct").field("from", &s.address()).finish()
            }
            Self::Keychain(s, ak) => f
                .debug_struct("TempoLookup::Keychain")
                .field("wallet", &ak.wallet_address)
                .field("key", &s.address())
                .finish(),
        }
    }
}
