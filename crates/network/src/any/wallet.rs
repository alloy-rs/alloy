use crate::{Network, NetworkWallet, TxSigner};
use alloy_consensus::{SignableTransaction, TypedTransaction};
use alloy_primitives::{map::AddressHashMap, Address, PrimitiveSignature as Signature};
use std::sync::Arc;

use super::{AnyTxEnvelope, AnyTypedTransaction};

/// A wallet capable of signing any transaction for [`AnyNetwork`](crate::AnyNetwork).
#[derive(Clone, Default)]
pub struct AnyNetworkWallet {
    default: Address,
    signers: AddressHashMap<Arc<dyn TxSigner<Signature> + Send + Sync>>,
}

impl std::fmt::Debug for AnyNetworkWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnyNetworkWallet")
            .field("default_signer", &self.default)
            .field("credentials", &self.signers.len())
            .finish()
    }
}

impl<S> From<S> for AnyNetworkWallet
where
    S: TxSigner<Signature> + Send + Sync + 'static,
{
    fn from(signer: S) -> Self {
        Self::new(signer)
    }
}

impl AnyNetworkWallet {
    /// Create a new signer with the given signer as the default signer.
    pub fn new<S>(signer: S) -> Self
    where
        S: TxSigner<Signature> + Send + Sync + 'static,
    {
        let mut this = Self::default();
        this.register_default_signer(signer);
        this
    }

    /// Register a new signer on this object. This signer will be used to sign
    /// [`TransactionRequest`] and [`AnyTypedTransaction`] object that specify the
    /// signer's address in the `from` field.
    ///
    /// [`TransactionRequest`]: alloy_rpc_types_eth::TransactionRequest
    pub fn register_signer<S>(&mut self, signer: S)
    where
        S: TxSigner<Signature> + Send + Sync + 'static,
    {
        self.signers.insert(signer.address(), Arc::new(signer));
    }

    /// Register a new signer on this object, and set it as the default signer.
    /// This signer will be used to sign [`TransactionRequest`] and
    /// [`AnyTypedTransaction`] objects that do not specify a signer address in the
    /// `from` field.
    ///
    /// [`TransactionRequest`]: alloy_rpc_types_eth::TransactionRequest
    pub fn register_default_signer<S>(&mut self, signer: S)
    where
        S: TxSigner<Signature> + Send + Sync + 'static,
    {
        self.default = signer.address();
        self.register_signer(signer);
    }

    /// Get the default signer.
    pub fn default_signer(&self) -> Arc<dyn TxSigner<Signature> + Send + Sync + 'static> {
        self.signers.get(&self.default).cloned().expect("invalid signer")
    }

    /// Get the signer for the given address.
    pub fn signer_by_address(
        &self,
        address: Address,
    ) -> Option<Arc<dyn TxSigner<Signature> + Send + Sync + 'static>> {
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
                alloy_signer::Error::other(format!("Missing signing credential for {}", sender))
            })?
            .sign_transaction(tx)
            .await
    }
}

impl<N> NetworkWallet<N> for AnyNetworkWallet
where
    N: Network<UnsignedTx = AnyTypedTransaction, TxEnvelope = AnyTxEnvelope>,
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
        tx: AnyTypedTransaction,
    ) -> alloy_signer::Result<AnyTxEnvelope> {
        match tx {
            AnyTypedTransaction::Ethereum(t) => match t {
                TypedTransaction::Legacy(mut t) => {
                    let sig = self.sign_transaction_inner(sender, &mut t).await?;

                    let signed = t.into_signed(sig);
                    Ok(AnyTxEnvelope::Ethereum(signed.into()))
                }
                TypedTransaction::Eip2930(mut t) => {
                    let sig = self.sign_transaction_inner(sender, &mut t).await?;
                    let signed = t.into_signed(sig);
                    Ok(AnyTxEnvelope::Ethereum(signed.into()))
                }
                TypedTransaction::Eip1559(mut t) => {
                    let sig = self.sign_transaction_inner(sender, &mut t).await?;
                    let signed = t.into_signed(sig);
                    Ok(AnyTxEnvelope::Ethereum(signed.into()))
                }
                TypedTransaction::Eip4844(mut t) => {
                    let sig = self.sign_transaction_inner(sender, &mut t).await?;
                    let signed = t.into_signed(sig);
                    Ok(AnyTxEnvelope::Ethereum(signed.into()))
                }
                TypedTransaction::Eip7702(mut t) => {
                    let sig = self.sign_transaction_inner(sender, &mut t).await?;
                    let signed = t.into_signed(sig);
                    Ok(AnyTxEnvelope::Ethereum(signed.into()))
                }
            },
            _ => unimplemented!("cannot sign UnknownTypedTransaction"),
        }
    }
}
