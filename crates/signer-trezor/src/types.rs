//! Helpers for interacting with the Ethereum Trezor App.
//!
//! [Official Docs](https://docs.trezor.io/trezor-firmware/index.html)

use alloy_primitives::{hex, B256, U256};
use std::fmt;
use thiserror::Error;
use trezor_client::client::AccessListItem as Trezor_AccessListItem;

/// Trezor wallet type.
#[derive(Clone, Debug)]
pub enum DerivationType {
    /// Trezor Live-generated HD path
    TrezorLive(usize),
    /// Any other path.
    ///
    /// **Warning**: Trezor by default forbids custom derivation paths;
    /// run `trezorctl set safety-checks prompt` to enable them.
    Other(String),
}

impl fmt::Display for DerivationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            DerivationType::TrezorLive(index) => write!(f, "m/44'/60'/{index}'/0/0"),
            DerivationType::Other(inner) => f.write_str(inner),
        }
    }
}

#[derive(Error, Debug)]
/// Error when using the Trezor transport
pub enum TrezorError {
    /// Underlying Trezor transport error.
    #[error(transparent)]
    Client(#[from] trezor_client::error::Error),
    /// Thrown when converting from a hex string.
    #[error(transparent)]
    Hex(#[from] hex::FromHexError),
    /// Thrown when converting a semver requirement.
    #[error(transparent)]
    Semver(#[from] semver::Error),
    /// [`ecdsa`](k256::ecdsa) error.
    #[error(transparent)]
    Ecdsa(#[from] k256::ecdsa::Error),
    /// Thrown when trying to sign an EIP-712 struct with an incompatible Trezor Ethereum app
    /// version.
    #[error("Trezor Ethereum app requires at least version {0:?}")]
    UnsupportedFirmwareVersion(String),
    /// No ENS support.
    #[error("Trezor does not support ENS")]
    NoEnsSupport,
    /// Could not retrieve device features.
    #[error("could not retrieve device features")]
    FeaturesError,
}

/// Trezor transaction.
#[allow(dead_code)]
pub(crate) struct TrezorTransaction {
    pub(crate) nonce: Vec<u8>,
    pub(crate) gas: Vec<u8>,
    pub(crate) gas_price: Vec<u8>,
    pub(crate) value: Vec<u8>,
    pub(crate) to: String,
    pub(crate) data: Vec<u8>,
    pub(crate) max_fee_per_gas: Vec<u8>,
    pub(crate) max_priority_fee_per_gas: Vec<u8>,
    pub(crate) access_list: Vec<Trezor_AccessListItem>,
}

impl TrezorTransaction {
    #[allow(dead_code)]
    fn to_trimmed_big_endian(value: &U256) -> Vec<u8> {
        let trimmed_value = B256::from(*value);
        trimmed_value[value.leading_zeros() / 8..].to_vec()
    }

    #[cfg(TODO)] // TODO: TypedTransaction
    pub fn load(tx: &TypedTransaction) -> Result<Self, TrezorError> {
        let to: String = match tx.to() {
            Some(v) => match v {
                NameOrAddress::Name(_) => return Err(TrezorError::NoEnsSupport),
                NameOrAddress::Address(value) => hex::encode_prefixed(value),
            },
            // Contract Creation
            None => "".to_string(),
        };

        let nonce = tx.nonce().map_or(vec![], Self::to_trimmed_big_endian);
        let gas = tx.gas().map_or(vec![], Self::to_trimmed_big_endian);
        let gas_price = tx.gas_price().map_or(vec![], |v| Self::to_trimmed_big_endian(&v));
        let value = tx.value().map_or(vec![], Self::to_trimmed_big_endian);
        let data = tx.data().map_or(vec![], |v| v.to_vec());

        match tx {
            TypedTransaction::Eip2930(_) | TypedTransaction::Legacy(_) => Ok(Self {
                nonce,
                gas,
                gas_price,
                value,
                to,
                data,
                max_fee_per_gas: vec![],
                max_priority_fee_per_gas: vec![],
                access_list: vec![],
            }),
            TypedTransaction::Eip1559(eip1559_tx) => {
                let max_fee_per_gas =
                    eip1559_tx.max_fee_per_gas.map_or(vec![], |v| Self::to_trimmed_big_endian(&v));

                let max_priority_fee_per_gas = eip1559_tx
                    .max_priority_fee_per_gas
                    .map_or(vec![], |v| Self::to_trimmed_big_endian(&v));

                let mut access_list: Vec<Trezor_AccessListItem> = Vec::new();
                for item in &eip1559_tx.access_list.0 {
                    let address: String = hex::encode_prefixed(item.address);
                    let mut storage_keys: Vec<Vec<u8>> = Vec::new();

                    for key in &item.storage_keys {
                        storage_keys.push(key.as_bytes().to_vec())
                    }

                    access_list.push(Trezor_AccessListItem { address, storage_keys })
                }

                Ok(Self {
                    nonce,
                    gas,
                    gas_price,
                    value,
                    to,
                    data,
                    max_fee_per_gas,
                    max_priority_fee_per_gas,
                    access_list,
                })
            }
            #[cfg(feature = "optimism")]
            TypedTransaction::DepositTransaction(_) => Ok(Self {
                nonce,
                gas,
                gas_price,
                value,
                to,
                data,
                max_fee_per_gas: vec![],
                max_priority_fee_per_gas: vec![],
                access_list: vec![],
            }),
        }
    }
}
