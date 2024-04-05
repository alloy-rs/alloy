use super::Ethereum;
use crate::{NetworkSigner, TxSigner};
use alloy_consensus::{SignableTransaction, TxEnvelope, TypedTransaction};
use alloy_signer::Signature;
use async_trait::async_trait;
use std::sync::Arc;

/// A signer capable of signing any transaction for the Ethereum network.
#[derive(Clone)]
pub struct EthereumSigner(Arc<dyn TxSigner<Signature> + Send + Sync>);

impl std::fmt::Debug for EthereumSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("EthereumSigner").finish()
    }
}

impl<S> From<S> for EthereumSigner
where
    S: TxSigner<Signature> + Send + Sync + 'static,
{
    fn from(signer: S) -> Self {
        Self::new(signer)
    }
}

impl EthereumSigner {
    /// Create a new Ethereum signer.
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
impl NetworkSigner<Ethereum> for EthereumSigner {
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
            #[cfg(feature = "optimism")]
            TypedTransaction::Deposit(_) => Err(alloy_signer::Error::Other(
                "Optimism transactions are not supported by the Ethereum signer".into(),
            )),
        }
    }
}
