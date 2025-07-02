use crate::{
    AnyNetwork, AnyTxEnvelope, AnyTypedTransaction, FullSigner, Network, NetworkWallet, TxSigner,
};
use alloy_consensus::{SignableTransaction, TxEnvelope, TypedTransaction};
use alloy_primitives::{map::AddressHashMap, Address, Signature};
use std::{fmt::Debug, ops::Deref, sync::Arc};

use super::Ethereum;

/// A wallet capable of signing any transaction for the Ethereum network.
#[derive(Clone, Default)]
pub struct EthereumWallet {
    default: Address,
    signers: AddressHashMap<ArcFullSigner>,
}

impl std::fmt::Debug for EthereumWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EthereumWallet")
            .field("default_signer", &self.default)
            .field("credentials", &self.signers.len())
            .finish()
    }
}

impl<S> From<S> for EthereumWallet
where
    S: FullSigner<Signature> + Send + Sync + 'static,
{
    fn from(signer: S) -> Self {
        Self::new(signer)
    }
}

impl EthereumWallet {
    /// Create a new signer with the given signer as the default signer.
    pub fn new<S>(signer: S) -> Self
    where
        S: FullSigner<Signature> + Send + Sync + 'static,
    {
        let mut this = Self::default();
        this.register_default_signer(signer);
        this
    }

    /// Register a new signer on this object. This signer will be used to sign
    /// [`TransactionRequest`] and [`TypedTransaction`] object that specify the
    /// signer's address in the `from` field.
    ///
    /// [`TransactionRequest`]: alloy_rpc_types_eth::TransactionRequest
    pub fn register_signer<S>(&mut self, signer: S)
    where
        S: FullSigner<Signature> + Send + Sync + 'static,
    {
        let arc_signer = ArcFullSigner::new(signer);
        self.signers.insert(arc_signer.address(), arc_signer);
    }

    /// Register a new signer on this object, and set it as the default signer.
    /// This signer will be used to sign [`TransactionRequest`] and
    /// [`TypedTransaction`] objects that do not specify a signer address in the
    /// `from` field.
    ///
    /// [`TransactionRequest`]: alloy_rpc_types_eth::TransactionRequest
    pub fn register_default_signer<S>(&mut self, signer: S)
    where
        S: FullSigner<Signature> + Send + Sync + 'static,
    {
        self.default = TxSigner::address(&signer);
        self.register_signer(signer);
    }

    /// Sets the default signer to the given address.
    ///
    /// The default signer is used to sign [`TransactionRequest`] and [`TypedTransaction`] objects
    /// that do not specify a signer address in the `from` field.
    ///
    /// The provided address must be a registered signer otherwise an error is returned.
    ///
    /// If you're looking to add a new signer and set it as default, use
    /// [`EthereumWallet::register_default_signer`].
    ///
    /// [`TransactionRequest`]: alloy_rpc_types_eth::TransactionRequest
    pub fn set_default_signer(&mut self, address: Address) -> alloy_signer::Result<()> {
        if self.signers.contains_key(&address) {
            self.default = address;
            Ok(())
        } else {
            Err(alloy_signer::Error::message(format!(
                "{address} is not a registered signer. Use `register_default_signer`"
            )))
        }
    }

    /// Get the default signer.
    pub fn default_signer(&self) -> ArcFullSigner {
        self.signers.get(&self.default).cloned().expect("invalid signer")
    }

    /// Get the signer for the given address.
    pub fn signer_by_address(&self, address: Address) -> Option<ArcFullSigner> {
        self.signers.get(&address).cloned()
    }

    #[doc(alias = "sign_tx_inner")]
    async fn sign_transaction_inner(
        &self,
        sender: Address,
        tx: &mut dyn SignableTransaction<Signature>,
    ) -> alloy_signer::Result<Signature> {
        self.signer_by_address(sender)
            .ok_or_else(|| {
                alloy_signer::Error::other(format!("Missing signing credential for {sender}"))
            })?
            .sign_transaction(tx)
            .await
    }

    /// Signs a hash with the given signer address.
    ///
    /// Hash can be arbitrary data or EIP-712 typed data hash.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use alloy_sol_types::{sol, eip712_domain};
    /// use alloy_primitives::{address, keccak256, B256};
    /// use alloy_signer_local::PrivateKeySigner;
    /// sol! {
    ///     struct Test {
    ///         uint256 value;
    ///     }
    /// }
    ///
    /// #[tokio::main]
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///   let domain = eip712_domain! {
    ///      name: "Test",
    ///      version: "1.0",
    ///      chain_id: 1,
    ///      verifying_contract: address!("0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"),
    ///      salt: keccak256("test_salt"),
    ///   };
    ///   
    ///   let alice: PrivateKeySigner = "0x...".parse()?;
    ///   let bob: PrivateKeySigner = "0x...".parse()?;
    ///
    ///    let wallet = EthereumWallet::new(alice);
    ///    wallet.register_signer(bob);
    ///
    ///    let t = Test { value: U256::from(0x42) };
    ///
    ///    let hash = t.eip712_signing_hash(&domain);
    ///
    ///    let signature = wallet.sign_hash_with(alice.address(), &hash).await?;
    ///    
    ///    Ok(())
    /// }
    /// ```
    #[cfg(feature = "eip712")]
    pub async fn sign_hash_with(
        &self,
        signer: Address,
        hash: &alloy_primitives::B256,
    ) -> alloy_signer::Result<Signature> {
        self.signer_by_address(signer)
            .ok_or_else(|| {
                alloy_signer::Error::other(format!("Missing signing credential for {signer}"))
            })?
            .sign_hash(hash)
            .await
    }
}

impl<N> NetworkWallet<N> for EthereumWallet
where
    N: Network<UnsignedTx = TypedTransaction, TxEnvelope = TxEnvelope>,
{
    fn default_signer_address(&self) -> Address {
        self.default
    }

    fn has_signer_for(&self, address: &Address) -> bool {
        self.signers.contains_key(address)
    }

    fn signer_addresses(&self) -> impl Iterator<Item = Address> {
        self.signers.keys().copied()
    }

    #[doc(alias = "sign_tx_from")]
    async fn sign_transaction_from(
        &self,
        sender: Address,
        tx: TypedTransaction,
    ) -> alloy_signer::Result<TxEnvelope> {
        match tx {
            TypedTransaction::Legacy(mut t) => {
                let sig = self.sign_transaction_inner(sender, &mut t).await?;
                Ok(t.into_signed(sig).into())
            }
            TypedTransaction::Eip2930(mut t) => {
                let sig = self.sign_transaction_inner(sender, &mut t).await?;
                Ok(t.into_signed(sig).into())
            }
            TypedTransaction::Eip1559(mut t) => {
                let sig = self.sign_transaction_inner(sender, &mut t).await?;
                Ok(t.into_signed(sig).into())
            }
            TypedTransaction::Eip4844(mut t) => {
                let sig = self.sign_transaction_inner(sender, &mut t).await?;
                Ok(t.into_signed(sig).into())
            }
            TypedTransaction::Eip7702(mut t) => {
                let sig = self.sign_transaction_inner(sender, &mut t).await?;
                Ok(t.into_signed(sig).into())
            }
        }
    }
}

impl NetworkWallet<AnyNetwork> for EthereumWallet {
    fn default_signer_address(&self) -> Address {
        self.default
    }

    fn has_signer_for(&self, address: &Address) -> bool {
        self.signers.contains_key(address)
    }

    fn signer_addresses(&self) -> impl Iterator<Item = Address> {
        self.signers.keys().copied()
    }

    #[doc(alias = "sign_tx_from")]
    async fn sign_transaction_from(
        &self,
        sender: Address,
        tx: AnyTypedTransaction,
    ) -> alloy_signer::Result<AnyTxEnvelope> {
        match tx {
            AnyTypedTransaction::Ethereum(t) => Ok(AnyTxEnvelope::Ethereum(
                NetworkWallet::<Ethereum>::sign_transaction_from(self, sender, t).await?,
            )),
            _ => Err(alloy_signer::Error::other("cannot sign UnknownTypedTransaction")),
        }
    }
}

/// A trait for converting a signer into a [`NetworkWallet`].
pub trait IntoWallet<N: Network = Ethereum>: Send + Sync + Debug {
    /// The wallet type for the network.
    type NetworkWallet: NetworkWallet<N>;
    /// Convert the signer into a wallet.
    fn into_wallet(self) -> Self::NetworkWallet;
}

impl<W: NetworkWallet<N>, N: Network> IntoWallet<N> for W {
    type NetworkWallet = W;

    fn into_wallet(self) -> Self::NetworkWallet {
        self
    }
}

/// Wrapper type for [`FullSigner`] that is used in [`EthereumWallet`].
///
/// This is useful to disambiguate the function calls on a signer via [`EthereumWallet`] as
/// [`TxSigner`] and [`Signer`] have the same methods e.g [`TxSigner::address`] and
/// [`Signer::address`]
///
/// [`Signer`]: alloy_signer::Signer
/// [`Signer::address`]: alloy_signer::Signer::address
#[derive(Clone)]
pub struct ArcFullSigner(Arc<dyn FullSigner<Signature> + Send + Sync>);

impl Debug for ArcFullSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArcFullSigner").field("address", &self.address()).finish()
    }
}

impl ArcFullSigner {
    /// Create a new [`ArcFullSigner`] from a given [`FullSigner`].
    pub fn new<S>(signer: S) -> Self
    where
        S: FullSigner<Signature> + Send + Sync + 'static,
    {
        Self(Arc::new(signer))
    }

    /// Get the address of the signer.
    pub fn address(&self) -> Address {
        self.0.address()
    }

    /// Get the underlying [`FullSigner`] as a reference.
    pub fn signer(&self) -> &dyn FullSigner<Signature> {
        self.0.as_ref()
    }

    /// Get the underlying [`FullSigner`] as an owned value.
    pub fn into_signer(self) -> Arc<dyn FullSigner<Signature> + Send + Sync> {
        self.0
    }
}

impl Deref for ArcFullSigner {
    type Target = dyn FullSigner<Signature> + Send + Sync;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
