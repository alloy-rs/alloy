//! [YubiHSM2](yubihsm) signer implementation.

use super::{LocalSigner, LocalSignerError};
use alloy_signer::utils::raw_public_key_to_address;
use k256::Secp256k1;
use yubihsm::{
    asymmetric::Algorithm::EcK256, ecdsa::Signer as YubiSigner, object, object::Label, Capability,
    Client, Connector, Credentials, Domain,
};

impl LocalSigner<YubiSigner<Secp256k1>> {
    /// Connects to a YubiHSM ECDSA key at the provided ID.
    ///
    /// # Panics
    ///
    /// Panics if the HSM connection cannot be opened or the key cannot be loaded. Use
    /// [`try_connect`](Self::try_connect) to handle these errors.
    pub fn connect(connector: Connector, credentials: Credentials, id: object::Id) -> Self {
        Self::try_connect(connector, credentials, id).expect("failed to connect to YubiHSM signer")
    }

    /// Attempts to connect to a YubiHSM ECDSA key at the provided ID.
    pub fn try_connect(
        connector: Connector,
        credentials: Credentials,
        id: object::Id,
    ) -> Result<Self, LocalSignerError> {
        let client = Client::open(connector, credentials, true)?;
        let signer = YubiSigner::create(client, id)?;
        Ok(signer.into())
    }

    /// Creates a new random ECDSA keypair on the YubiHSM at the provided ID.
    ///
    /// # Panics
    ///
    /// Panics if the HSM connection cannot be opened or the key cannot be generated or loaded. Use
    /// [`try_new`](Self::try_new) to handle these errors.
    pub fn new(
        connector: Connector,
        credentials: Credentials,
        id: object::Id,
        label: Label,
        domain: Domain,
    ) -> Self {
        Self::try_new(connector, credentials, id, label, domain)
            .expect("failed to create YubiHSM signer")
    }

    /// Attempts to create a new random ECDSA keypair on the YubiHSM at the provided ID.
    pub fn try_new(
        connector: Connector,
        credentials: Credentials,
        id: object::Id,
        label: Label,
        domain: Domain,
    ) -> Result<Self, LocalSignerError> {
        let client = Client::open(connector, credentials, true)?;
        let id =
            client.generate_asymmetric_key(id, label, domain, Capability::SIGN_ECDSA, EcK256)?;
        let signer = YubiSigner::create(client, id)?;
        Ok(signer.into())
    }

    /// Uploads the provided keypair to the YubiHSM at the provided ID.
    ///
    /// # Panics
    ///
    /// Panics if the HSM connection cannot be opened or the key cannot be imported or loaded. Use
    /// [`try_from_key`](Self::try_from_key) to handle these errors.
    pub fn from_key(
        connector: Connector,
        credentials: Credentials,
        id: object::Id,
        label: Label,
        domain: Domain,
        key: impl Into<Vec<u8>>,
    ) -> Self {
        Self::try_from_key(connector, credentials, id, label, domain, key)
            .expect("failed to import YubiHSM signer")
    }

    /// Attempts to upload the provided keypair to the YubiHSM at the provided ID.
    pub fn try_from_key(
        connector: Connector,
        credentials: Credentials,
        id: object::Id,
        label: Label,
        domain: Domain,
        key: impl Into<Vec<u8>>,
    ) -> Result<Self, LocalSignerError> {
        let client = Client::open(connector, credentials, true)?;
        let id =
            client.put_asymmetric_key(id, label, domain, Capability::SIGN_ECDSA, EcK256, key)?;
        let signer = YubiSigner::create(client, id)?;
        Ok(signer.into())
    }
}

impl From<YubiSigner<Secp256k1>> for LocalSigner<YubiSigner<Secp256k1>> {
    fn from(credential: YubiSigner<Secp256k1>) -> Self {
        let pubkey = credential.as_ref().to_encoded_point(false);
        let bytes = pubkey.as_bytes();
        debug_assert_eq!(bytes[0], 0x04);
        let address = raw_public_key_to_address(&bytes[1..]);
        Self::new_with_credential(credential, address, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SignerSync;
    use alloy_primitives::{address, hex};

    #[test]
    fn from_key() {
        let key = hex::decode("2d8c44dc2dd2f0bea410e342885379192381e82d855b1b112f9b55544f1e0900")
            .unwrap();

        let connector = yubihsm::Connector::mockhsm();
        let signer = LocalSigner::try_from_key(
            connector,
            Credentials::default(),
            0,
            Label::from_bytes(&[]).unwrap(),
            Domain::at(1).unwrap(),
            key,
        )
        .unwrap();

        let msg = "Some data";
        let sig = signer.sign_message_sync(msg.as_bytes()).unwrap();
        assert_eq!(sig.recover_address_from_msg(msg).unwrap(), signer.address());
        assert_eq!(signer.address(), address!("2DE2C386082Cff9b28D62E60983856CE1139eC49"));
    }

    #[test]
    fn new_key() {
        let connector = yubihsm::Connector::mockhsm();
        let signer = LocalSigner::<YubiSigner<Secp256k1>>::try_new(
            connector,
            Credentials::default(),
            0,
            Label::from_bytes(&[]).unwrap(),
            Domain::at(1).unwrap(),
        )
        .unwrap();

        let msg = "Some data";
        let sig = signer.sign_message_sync(msg.as_bytes()).unwrap();
        assert_eq!(sig.recover_address_from_msg(msg).unwrap(), signer.address());
    }

    #[test]
    fn missing_key_returns_error() {
        let result = LocalSigner::<YubiSigner<Secp256k1>>::try_connect(
            yubihsm::Connector::mockhsm(),
            Credentials::default(),
            0x1234,
        );

        assert!(result.is_err());
    }

    #[test]
    fn invalid_key_returns_error() {
        let result = LocalSigner::<YubiSigner<Secp256k1>>::try_from_key(
            yubihsm::Connector::mockhsm(),
            Credentials::default(),
            1,
            Label::from_bytes(&[]).unwrap(),
            Domain::at(1).unwrap(),
            [0; 31],
        );

        assert!(matches!(result, Err(LocalSignerError::YubiHsmError(_))));
    }
}
