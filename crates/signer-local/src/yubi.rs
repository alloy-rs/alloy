//! [YubiHSM2](yubihsm) signer implementation.

use super::{LocalSigner, LocalSignerError};
use alloy_consensus::SignableTransaction;
use alloy_network::{TxSigner, TxSignerSync};
use alloy_primitives::{Address, ChainId, Signature, B256};
use alloy_signer::{
    sign_transaction_with_chain_id, utils::raw_public_key_to_address, Error, Result, Signer,
    SignerSync,
};
use async_trait::async_trait;
use k256::{
    ecdsa::{signature::hazmat::PrehashSigner, RecoveryId, Signature as K256Signature},
    Secp256k1,
};
use std::{fmt, sync::Arc};
use yubihsm::{
    asymmetric::Algorithm::EcK256, ecdsa::Signer as YubiCredential, object, object::Label,
    Capability, Client, Connector, Credentials, Domain,
};

type Credential = YubiCredential<Secp256k1>;

/// An Ethereum signer backed by a YubiHSM ECDSA key.
///
/// The YubiHSM transports are synchronous. Asynchronous signing moves their blocking I/O to
/// Tokio's blocking thread pool, while the [`SignerSync`] and [`TxSignerSync`] implementations call
/// the HSM directly.
///
/// Asynchronous signing must run inside a Tokio runtime. Once dispatched, an HSM operation keeps
/// running even if the future waiting for it is dropped.
pub struct YubiSigner {
    credential: Arc<Credential>,
    address: Address,
    chain_id: Option<ChainId>,
}

impl YubiSigner {
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
        let signer = YubiCredential::create(client, id)?;
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
        let signer = YubiCredential::create(client, id)?;
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
        let signer = YubiCredential::create(client, id)?;
        Ok(signer.into())
    }

    /// Constructs a signer from a YubiHSM credential and its Ethereum address.
    ///
    /// `address` is trusted and is not derived from or checked against `credential`.
    pub fn new_with_credential(
        credential: Credential,
        address: Address,
        chain_id: Option<ChainId>,
    ) -> Self {
        Self { credential: Arc::new(credential), address, chain_id }
    }

    /// Returns this signer's YubiHSM credential.
    pub fn credential(&self) -> &Credential {
        &self.credential
    }

    /// Consumes this signer and returns its YubiHSM credential if no background signing operation
    /// still holds it.
    pub fn try_into_credential(self) -> std::result::Result<Credential, Self> {
        let Self { credential, address, chain_id } = self;
        Arc::try_unwrap(credential).map_err(|credential| Self { credential, address, chain_id })
    }

    /// Consumes this signer and returns its YubiHSM credential.
    ///
    /// # Panics
    ///
    /// Panics if a cancelled asynchronous signing future left its blocking operation running. Use
    /// [`try_into_credential`](Self::try_into_credential) to handle this case.
    pub fn into_credential(self) -> Credential {
        self.try_into_credential()
            .unwrap_or_else(|_| panic!("YubiHSM signing operation is still running"))
    }

    /// Returns this signer's address.
    pub const fn address(&self) -> Address {
        self.address
    }

    /// Returns this signer's chain ID.
    pub const fn chain_id(&self) -> Option<ChainId> {
        self.chain_id
    }
}

impl From<Credential> for YubiSigner {
    fn from(credential: Credential) -> Self {
        let pubkey = credential.as_ref().to_encoded_point(false);
        let bytes = pubkey.as_bytes();
        debug_assert_eq!(bytes[0], 0x04);
        let address = raw_public_key_to_address(&bytes[1..]);
        Self::new_with_credential(credential, address, None)
    }
}

impl From<LocalSigner<Credential>> for YubiSigner {
    fn from(signer: LocalSigner<Credential>) -> Self {
        let LocalSigner { credential, address, chain_id } = signer;
        Self::new_with_credential(credential, address, chain_id)
    }
}

impl fmt::Debug for YubiSigner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("YubiSigner")
            .field("address", &self.address)
            .field("chain_id", &self.chain_id)
            .finish()
    }
}

async fn run_blocking<T>(operation: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T>
where
    T: Send + 'static,
{
    let handle = tokio::runtime::Handle::try_current().map_err(Error::other)?;
    handle.spawn_blocking(operation).await.map_err(Error::other)?
}

fn sign_hash(credential: &Credential, hash: &B256) -> Result<Signature> {
    let signature: (K256Signature, RecoveryId) = credential.sign_prehash(hash.as_ref())?;
    Ok(signature.into())
}

#[async_trait]
impl Signer for YubiSigner {
    async fn sign_hash(&self, hash: &B256) -> Result<Signature> {
        let credential = Arc::clone(&self.credential);
        let hash = *hash;
        run_blocking(move || sign_hash(&credential, &hash)).await
    }

    fn address(&self) -> Address {
        self.address
    }

    fn chain_id(&self) -> Option<ChainId> {
        self.chain_id
    }

    fn set_chain_id(&mut self, chain_id: Option<ChainId>) {
        self.chain_id = chain_id;
    }
}

impl SignerSync for YubiSigner {
    fn sign_hash_sync(&self, hash: &B256) -> Result<Signature> {
        sign_hash(&self.credential, hash)
    }

    fn chain_id_sync(&self) -> Option<ChainId> {
        self.chain_id
    }
}

#[async_trait]
impl TxSigner<Signature> for YubiSigner {
    fn address(&self) -> Address {
        self.address
    }

    async fn sign_transaction(
        &self,
        tx: &mut dyn SignableTransaction<Signature>,
    ) -> Result<Signature> {
        sign_transaction_with_chain_id!(self, tx, self.sign_hash(&tx.signature_hash()).await)
    }
}

impl TxSignerSync<Signature> for YubiSigner {
    fn address(&self) -> Address {
        self.address
    }

    fn sign_transaction_sync(
        &self,
        tx: &mut dyn SignableTransaction<Signature>,
    ) -> Result<Signature> {
        sign_transaction_with_chain_id!(self, tx, self.sign_hash_sync(&tx.signature_hash()))
    }
}

alloy_network::impl_into_wallet!(YubiSigner);

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_consensus::TxLegacy;
    use alloy_primitives::{address, hex};
    use std::{
        sync::{
            atomic::{AtomicBool, Ordering},
            mpsc, Arc,
        },
        time::Duration,
    };

    #[test]
    fn from_key() {
        let key = hex::decode("2d8c44dc2dd2f0bea410e342885379192381e82d855b1b112f9b55544f1e0900")
            .unwrap();

        let connector = yubihsm::Connector::mockhsm();
        let signer = YubiSigner::try_from_key(
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
        let signer = YubiSigner::try_new(
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
        let result =
            YubiSigner::try_connect(yubihsm::Connector::mockhsm(), Credentials::default(), 0x1234);

        assert!(result.is_err());
    }

    #[test]
    fn invalid_key_returns_error() {
        let result = YubiSigner::try_from_key(
            yubihsm::Connector::mockhsm(),
            Credentials::default(),
            1,
            Label::from_bytes(&[]).unwrap(),
            Domain::at(1).unwrap(),
            [0; 31],
        );

        assert!(matches!(result, Err(LocalSignerError::YubiHsmError(_))));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn async_signing_matches_sync_signing() {
        let signer = YubiSigner::try_new(
            yubihsm::Connector::mockhsm(),
            Credentials::default(),
            0,
            Label::from_bytes(&[]).unwrap(),
            Domain::at(1).unwrap(),
        )
        .unwrap();
        let message = b"Some data";

        let sync = signer.sign_message_sync(message).unwrap();
        let asynchronous = signer.sign_message(message).await.unwrap();

        assert_eq!(sync.recover_address_from_msg(message).unwrap(), signer.address());
        assert_eq!(asynchronous.recover_address_from_msg(message).unwrap(), signer.address());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn async_transaction_signing_applies_chain_id() {
        let mut signer = YubiSigner::try_new(
            yubihsm::Connector::mockhsm(),
            Credentials::default(),
            0,
            Label::from_bytes(&[]).unwrap(),
            Domain::at(1).unwrap(),
        )
        .unwrap();
        signer.set_chain_id(Some(1));
        let mut tx = TxLegacy::default();

        let signature = signer.sign_transaction(&mut tx).await.unwrap();

        assert_eq!(tx.chain_id, Some(1));
        assert_eq!(
            signature.recover_address_from_prehash(&tx.signature_hash()).unwrap(),
            signer.address()
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn blocking_work_does_not_stall_executor() {
        let (release_tx, release_rx) = mpsc::channel();
        let progressed = Arc::new(AtomicBool::new(false));
        let task_progressed = Arc::clone(&progressed);
        let unrelated = tokio::spawn(async move {
            task_progressed.store(true, Ordering::SeqCst);
        });

        let blocking = run_blocking(move || {
            release_rx.recv_timeout(Duration::from_secs(2)).map_err(Error::other)?;
            Ok(())
        });
        let release = async move {
            for _ in 0..100 {
                if progressed.load(Ordering::SeqCst) {
                    break;
                }
                tokio::task::yield_now().await;
            }
            assert!(progressed.load(Ordering::SeqCst));
            release_tx.send(()).unwrap();
        };

        let (result, ()) = tokio::join!(blocking, release);
        result.unwrap();
        unrelated.await.unwrap();
    }
}
