use crate::{AnyNetwork, NetworkSigner, TxSigner};
use alloy_consensus::{SignableTransaction, TxEnvelope, TypedTransaction};
use alloy_signer::Signature;
use async_trait::async_trait;
use std::sync::Arc;

/// A signer capable of signing any transaction for the [AnyNetwork] network.
#[derive(Clone)]
pub struct AnyNetworkSigner(Arc<dyn TxSigner<Signature> + Send + Sync>);

impl std::fmt::Debug for AnyNetworkSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("AnyNetworkSigner").finish()
    }
}

impl<S> From<S> for AnyNetworkSigner
where
    S: TxSigner<Signature> + Send + Sync + 'static,
{
    fn from(signer: S) -> Self {
        Self::new(signer)
    }
}

impl AnyNetworkSigner {
    /// Create a new AnyNetwork signer.
    pub fn new<S>(signer: S) -> Self
    where
        S: TxSigner<Signature> + Send + Sync + 'static,
    {
        Self(Arc::new(signer))
    }

    async fn sign_transaction(
        &self,
        tx: &mut dyn SignableTransaction<Signature>,
    ) -> alloy_signer::Result<Signature> {
        self.0.sign_transaction(tx).await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl NetworkSigner<AnyNetwork> for AnyNetworkSigner {
    async fn sign_transaction(&self, tx: TypedTransaction) -> alloy_signer::Result<TxEnvelope> {
        match tx {
            TypedTransaction::Legacy(mut t) => {
                let sig = self.sign_transaction(&mut t).await?;
                Ok(t.into_signed(sig).into())
            }
            TypedTransaction::Eip2930(mut t) => {
                let sig = self.sign_transaction(&mut t).await?;
                Ok(t.into_signed(sig).into())
            }
            TypedTransaction::Eip1559(mut t) => {
                let sig = self.sign_transaction(&mut t).await?;
                Ok(t.into_signed(sig).into())
            }
            TypedTransaction::Eip4844(mut t) => {
                let sig = self.sign_transaction(&mut t).await?;
                Ok(t.into_signed(sig).into())
            }
        }
    }
}
