use std::sync::{Arc, OnceLock};

use alloy_network::{Network, TransactionBuilder};
use alloy_transport::TransportResult;

use crate::{
    fillers::{FillerControlFlow, TxFiller},
    provider::SendableTx,
};

/// A [`TxFiller`] that populates the chain ID of a transaction.
///
/// If a chain ID is provided, it will be used for filling. If a chain ID
/// is not provided, the filler will attempt to fetch the chain ID from the
/// provider the first time a transaction is prepared, and will cache it for
/// future transactions.
///
/// Transactions that already have a chain_id set by the user will not be
/// modified.
///
/// # Example
///
/// ```
/// # use alloy_network::{NetworkSigner, EthereumSigner, Ethereum};
/// # use alloy_rpc_types::TransactionRequest;
/// # use alloy_provider::{ProviderBuilder, RootProvider, Provider};
/// # async fn test<S: NetworkSigner<Ethereum> + Clone>(url: url::Url, signer: S) -> Result<(), Box<dyn std::error::Error>> {
/// let provider = ProviderBuilder::new()
///     .with_chain_id(1)
///     .signer(signer)
///     .on_http(url)?;
///
/// provider.send_transaction(TransactionRequest::default()).await;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChainIdFiller(Arc<OnceLock<u64>>);

impl ChainIdFiller {
    /// Create a new [`ChainIdFiller`] with an optional chain ID.
    ///
    /// If a chain ID is provided, it will be used for filling. If a chain ID
    /// is not provided, the filler will attempt to fetch the chain ID from the
    /// provider the first time a transaction is prepared.
    pub fn new(chain_id: Option<u64>) -> Self {
        let lock = OnceLock::new();
        if let Some(chain_id) = chain_id {
            lock.set(chain_id).expect("brand new");
        }
        Self(Arc::new(lock))
    }
}

impl<N: Network> TxFiller<N> for ChainIdFiller {
    type Fillable = u64;

    fn status(&self, tx: &N::TransactionRequest) -> FillerControlFlow {
        if tx.chain_id().is_some() {
            FillerControlFlow::Finished
        } else {
            FillerControlFlow::Ready
        }
    }

    async fn prepare<P, T>(
        &self,
        provider: &P,
        _tx: &N::TransactionRequest,
    ) -> TransportResult<Self::Fillable>
    where
        P: crate::Provider<T, N>,
        T: alloy_transport::Transport + Clone,
    {
        match self.0.get().copied() {
            Some(chain_id) => Ok(chain_id),
            None => {
                let chain_id = provider.get_chain_id().await?;
                let chain_id = *self.0.get_or_init(|| chain_id);
                Ok(chain_id)
            }
        }
    }

    async fn fill(
        &self,
        fillable: Self::Fillable,
        mut tx: SendableTx<N>,
    ) -> TransportResult<SendableTx<N>> {
        if let Some(builder) = tx.as_mut_builder() {
            if builder.chain_id().is_none() {
                builder.set_chain_id(fillable)
            }
        };
        Ok(tx)
    }
}
