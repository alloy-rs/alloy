use std::sync::{Arc, OnceLock};

use alloy_network::{Network, TransactionBuilder};
use alloy_primitives::ChainId;
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
/// # use alloy_network::{Ethereum};
/// # use alloy_rpc_types_eth::TransactionRequest;
/// # use alloy_provider::{ProviderBuilder, RootProvider, Provider};
/// # use alloy_signer_local::PrivateKeySigner;
/// # async fn test(url: url::Url) -> Result<(), Box<dyn std::error::Error>> {
/// let pk: PrivateKeySigner = "0x...".parse()?;
/// let provider =
///     ProviderBuilder::<_, _, Ethereum>::default().with_chain_id(1).wallet(pk).connect_http(url);
///
/// provider.send_transaction(TransactionRequest::default()).await;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChainIdFiller(Arc<OnceLock<ChainId>>);

impl ChainIdFiller {
    /// Create a new [`ChainIdFiller`] with an optional chain ID.
    ///
    /// If a chain ID is provided, it will be used for filling. If a chain ID
    /// is not provided, the filler will attempt to fetch the chain ID from the
    /// provider the first time a transaction is prepared.
    pub fn new(chain_id: Option<ChainId>) -> Self {
        let lock = OnceLock::new();
        if let Some(chain_id) = chain_id {
            lock.set(chain_id).expect("brand new");
        }
        Self(Arc::new(lock))
    }
}

impl<N: Network> TxFiller<N> for ChainIdFiller {
    type Fillable = ChainId;

    fn status(&self, tx: &N::TransactionRequest) -> FillerControlFlow {
        if tx.chain_id().is_some() {
            FillerControlFlow::Finished
        } else {
            FillerControlFlow::Ready
        }
    }

    fn fill_sync(&self, tx: &mut SendableTx<N>) {
        if let Some(chain_id) = self.0.get() {
            if let Some(builder) = tx.as_mut_builder() {
                if builder.chain_id().is_none() {
                    builder.set_chain_id(*chain_id)
                }
            }
        }
    }

    async fn prepare<P>(
        &self,
        provider: &P,
        _tx: &N::TransactionRequest,
    ) -> TransportResult<Self::Fillable>
    where
        P: crate::Provider<N>,
    {
        match self.0.get().copied() {
            Some(chain_id) => Ok(chain_id),
            None => {
                let chain_id = provider.get_chain_id().await?;
                Ok(*self.0.get_or_init(|| chain_id))
            }
        }
    }

    async fn fill(
        &self,
        _fillable: Self::Fillable,
        mut tx: SendableTx<N>,
    ) -> TransportResult<SendableTx<N>> {
        self.fill_sync(&mut tx);
        Ok(tx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProviderBuilder;
    use alloy_network::Ethereum;
    use alloy_rpc_types_eth::TransactionRequest;
    use alloy_transport::mock::Asserter;
    use futures::future::join_all;

    #[tokio::test]
    async fn chain_id_cached_across_multiple_calls() {
        // GIVEN: Real Anvil provider + ChainIdFiller
        let provider = ProviderBuilder::new().connect_anvil();
        let filler = ChainIdFiller::default();
        let tx = TransactionRequest::default();

        // WHEN: First prepare() call fetches from Anvil
        let chain_id_1 = filler.prepare(&provider, &tx).await.unwrap();

        // AND: Second prepare() call
        let chain_id_2 = filler.prepare(&provider, &tx).await.unwrap();

        // THEN: Both return Anvil's chain ID (31337)
        assert_eq!(chain_id_1, 31337);
        assert_eq!(chain_id_2, 31337);

        // AND: Internal cache is populated (verify via state inspection)
        assert!(filler.0.get().is_some());
        assert_eq!(*filler.0.get().unwrap(), 31337);
    }

    #[tokio::test]
    async fn preconfigured_chain_id_never_fetches() {
        // GIVEN: Real Anvil provider (returns 31337) + pre-configured ChainIdFiller (42)
        let provider = ProviderBuilder::new().connect_anvil();
        let filler = ChainIdFiller::new(Some(42));
        let tx = TransactionRequest::default();

        // WHEN: Call prepare() 10 times
        for _ in 0..10 {
            let chain_id = filler.prepare(&provider, &tx).await.unwrap();

            // THEN: Every call returns pre-configured value (42), never Anvil's (31337)
            assert_eq!(chain_id, 42);
        }

        // AND: Internal cache contains pre-configured value (never overwritten)
        assert!(filler.0.get().is_some());
        assert_eq!(*filler.0.get().unwrap(), 42);
    }

    #[tokio::test]
    async fn first_fetch_error_propagates() {
        // GIVEN: ChainIdFiller with no pre-set chain ID
        let filler = ChainIdFiller::new(None);

        // AND: Mocked provider that returns error on get_chain_id
        let asserter = Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());
        asserter.push_failure_msg("mock backend gone");

        let tx = TransactionRequest::default();

        // WHEN: First prepare() call
        let result = filler.prepare(&provider, &tx).await;

        // THEN: Error propagates to caller
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("mock backend gone"));

        // AND: Cache is NOT poisoned (remains None for retry)
        assert!(filler.0.get().is_none());
    }

    #[tokio::test]
    async fn recovery_after_initial_failure() {
        // GIVEN: ChainIdFiller with no pre-set chain ID
        let filler = ChainIdFiller::new(None);

        // AND: Mocked provider with sequence: error → success
        let asserter = Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());
        asserter.push_failure_msg("transient network error");
        asserter.push_success(&1u64); // ChainId = u64
        asserter.push_success(&999u64); // Should not be called (cached)

        let tx = TransactionRequest::default();

        // WHEN: First prepare() call fails
        let result_1 = filler.prepare(&provider, &tx).await;

        // THEN: Error propagates
        assert!(result_1.is_err());
        assert!(result_1.unwrap_err().to_string().contains("transient network error"));

        // AND: Cache still empty
        assert!(filler.0.get().is_none());

        // WHEN: Second prepare() call (retry)
        let result_2 = filler.prepare(&provider, &tx).await;

        // THEN: Retry succeeds, returns chain ID 1
        assert!(result_2.is_ok());
        assert_eq!(result_2.unwrap(), 1);

        // AND: Cache is now populated
        assert!(filler.0.get().is_some());
        assert_eq!(*filler.0.get().unwrap(), 1);

        // WHEN: Third prepare() call
        let result_3 = filler.prepare(&provider, &tx).await;

        // THEN: Uses cached value (no third RPC call to mock)
        assert!(result_3.is_ok());
        assert_eq!(result_3.unwrap(), 1);
    }

    #[tokio::test]
    async fn cached_value_persists_across_many_calls() {
        // GIVEN: Real Anvil provider + ChainIdFiller
        let provider = ProviderBuilder::new().connect_anvil();
        let filler = ChainIdFiller::default();
        let tx = TransactionRequest::default();

        // WHEN: First prepare() call populates cache from Anvil
        let chain_id_1 = filler.prepare(&provider, &tx).await.unwrap();
        assert_eq!(chain_id_1, 31337);

        // AND: 10 subsequent prepare() calls
        for _ in 0..10 {
            let chain_id = filler.prepare(&provider, &tx).await;

            // THEN: All return cached 31337 (no errors)
            assert!(chain_id.is_ok());
            assert_eq!(chain_id.unwrap(), 31337);
        }

        // AND: Cache remains populated and consistent
        assert!(filler.0.get().is_some());
        assert_eq!(*filler.0.get().unwrap(), 31337);
    }

    #[test]
    fn tx_with_chain_id_is_finished() {
        let filler = ChainIdFiller::new(None);
        let mut tx = TransactionRequest::default();
        tx.set_chain_id(1);
        
        let status = TxFiller::<Ethereum>::status(&filler, &tx);
        assert!(matches!(status, FillerControlFlow::Finished));
    }

    #[test]
    fn tx_without_chain_id_is_ready() {
        let filler = ChainIdFiller::new(Some(42));
        let tx = TransactionRequest::default();
        assert!(tx.chain_id().is_none());
        
        let status = TxFiller::<Ethereum>::status(&filler, &tx);
        assert!(matches!(status, FillerControlFlow::Ready));
    }

    #[test]
    fn fill_sync_respects_preset_chain_id() {
        let filler = ChainIdFiller::new(Some(42));
        let mut tx = TransactionRequest::default();
        tx.set_chain_id(1);
        let mut sendable = SendableTx::<Ethereum>::Builder(tx);
        
        TxFiller::<Ethereum>::fill_sync(&filler, &mut sendable);
        
        if let SendableTx::Builder(tx) = sendable {
            assert_eq!(tx.chain_id(), Some(1));
        } else {
            panic!("Expected Builder variant");
        }
    }

    #[test]
    fn chain_id_max_value() {
        // GIVEN: ChainIdFiller with u64::MAX
        let filler = ChainIdFiller::new(Some(u64::MAX));
        
        // THEN: No panic/overflow, value stored correctly
        assert!(filler.0.get().is_some());
        assert_eq!(*filler.0.get().unwrap(), u64::MAX);
    }

    #[test]
    fn chain_id_zero_value() {
        // GIVEN: ChainIdFiller with 0
        let filler = ChainIdFiller::new(Some(0));
        
        // THEN: Accepts zero, value stored correctly (documents current behavior)
        assert!(filler.0.get().is_some());
        assert_eq!(*filler.0.get().unwrap(), 0);
    }

    #[tokio::test]
    async fn cloned_filler_shares_cache() {
        // GIVEN: ChainIdFiller that fetches from Anvil
        let provider = ProviderBuilder::new().connect_anvil();
        let filler1 = ChainIdFiller::new(None);
        let tx = TransactionRequest::default();
        filler1.prepare(&provider, &tx).await.unwrap();
        
        // WHEN: Filler is cloned
        let filler2 = filler1.clone();
        
        // THEN: Both return same cached chain_id
        let chain_id_1 = filler1.prepare(&provider, &tx).await.unwrap();
        let chain_id_2 = filler2.prepare(&provider, &tx).await.unwrap();
        assert_eq!(chain_id_1, 31337);
        assert_eq!(chain_id_2, 31337);
        
        // AND: Both fillers point to same Arc<OnceLock> (pointer equality)
        assert!(Arc::ptr_eq(&filler1.0, &filler2.0));
    }

    #[test]
    fn preconfigured_filler_clone_shares_value() {
        // GIVEN: ChainIdFiller with pre-configured chain_id = 42
        let filler1 = ChainIdFiller::new(Some(42));
        
        // WHEN: Cloned
        let filler2 = filler1.clone();
        
        // THEN: Both have same internal state
        assert_eq!(filler1.0.get(), Some(&42));
        assert_eq!(filler2.0.get(), Some(&42));
        assert!(Arc::ptr_eq(&filler1.0, &filler2.0));
    }

    #[tokio::test]
    async fn concurrent_first_fetch_single_initialization() {
        // GIVEN: Shared ChainIdFiller with no pre-configured chain_id
        let filler = Arc::new(ChainIdFiller::new(None));
        let provider = Arc::new(ProviderBuilder::new().connect_anvil());
        let tx = TransactionRequest::default();

        // WHEN: 10 concurrent tasks call prepare() simultaneously
        let tasks = (0..10)
            .map(|_| {
                let filler = Arc::clone(&filler);
                let provider = Arc::clone(&provider);
                let tx = tx.clone();
                tokio::spawn(async move { filler.prepare(&provider, &tx).await })
            })
            .collect::<Vec<_>>();

        let results = join_all(tasks).await;

        // THEN: All tasks succeed and return Anvil's chain ID (31337)
        for task_result in results {
            let chain_id = task_result.unwrap().unwrap();
            assert_eq!(chain_id, 31337);
        }

        // AND: OnceLock initialized exactly once (atomic initialization guarantee)
        assert!(filler.0.get().is_some());
        assert_eq!(*filler.0.get().unwrap(), 31337);
    }

    #[tokio::test]
    async fn concurrent_access_after_cache_populated() {
        // GIVEN: ChainIdFiller with cache already populated
        let provider = Arc::new(ProviderBuilder::new().connect_anvil());
        let filler = Arc::new(ChainIdFiller::new(None));
        let tx = TransactionRequest::default();

        // Pre-populate cache from Anvil
        filler.prepare(&*provider, &tx).await.unwrap();
        assert_eq!(*filler.0.get().unwrap(), 31337);

        // WHEN: 100 concurrent tasks read from populated cache
        let tasks = (0..100)
            .map(|_| {
                let filler = Arc::clone(&filler);
                let provider = Arc::clone(&provider);
                let tx = tx.clone();
                tokio::spawn(async move { filler.prepare(&provider, &tx).await })
            })
            .collect::<Vec<_>>();

        let results = join_all(tasks).await;

        // THEN: All reads succeed without contention
        for task_result in results {
            let chain_id = task_result.unwrap().unwrap();
            assert_eq!(chain_id, 31337);
        }

        // AND: Cache remains consistent
        assert_eq!(*filler.0.get().unwrap(), 31337);
    }

    // TB-015: Test 7.1 - Default Construction
    #[tokio::test]
    async fn default_behaves_like_new_none() {
        // GIVEN: ChainIdFiller via ::default() and via ::new(None)
        let filler_default = ChainIdFiller::default();
        let filler_new_none = ChainIdFiller::new(None);

        // AND: Real Anvil provider
        let provider = ProviderBuilder::new().connect_anvil();
        let tx = TransactionRequest::default();

        // WHEN: Both call prepare()
        let chain_id_default = filler_default.prepare(&provider, &tx).await.unwrap();
        let chain_id_new_none = filler_new_none.prepare(&provider, &tx).await.unwrap();

        // THEN: Both fetch from provider and return same chain ID
        assert_eq!(chain_id_default, 31337);
        assert_eq!(chain_id_new_none, 31337);

        // AND: Both cache the value
        assert!(filler_default.0.get().is_some());
        assert!(filler_new_none.0.get().is_some());
    }

    // TB-016: Test 7.2 - PartialEq Implementation
    #[test]
    fn partial_eq_semantics() {
        // GIVEN: Two fillers with same pre-configured value
        let filler1 = ChainIdFiller::new(Some(42));
        let filler2 = ChainIdFiller::new(Some(42));

        // THEN: They are equal (same value)
        assert_eq!(filler1, filler2);

        // GIVEN: Two fillers with different pre-configured values
        let filler3 = ChainIdFiller::new(Some(1));
        let filler4 = ChainIdFiller::new(Some(2));

        // THEN: They are not equal
        assert_ne!(filler3, filler4);

        // GIVEN: Two default fillers (both empty cache)
        let filler5 = ChainIdFiller::default();
        let filler6 = ChainIdFiller::default();

        // THEN: They are equal (both have empty OnceLock)
        assert_eq!(filler5, filler6);

        // GIVEN: Empty filler vs pre-configured filler
        let filler7 = ChainIdFiller::new(None);
        let filler8 = ChainIdFiller::new(Some(100));

        // THEN: They are not equal
        assert_ne!(filler7, filler8);
    }

    // TB-017: Test 7.3 - Debug Format
    #[test]
    fn debug_format_does_not_panic() {
        // GIVEN: ChainIdFiller with pre-configured value
        let filler_preconfigured = ChainIdFiller::new(Some(42));

        // WHEN: Debug formatted
        let debug_str = format!("{:?}", filler_preconfigured);

        // THEN: No panic
        // AND: Contains type name
        assert!(debug_str.contains("ChainIdFiller"));

        // GIVEN: ChainIdFiller with empty cache
        let filler_empty = ChainIdFiller::new(None);

        // WHEN: Debug formatted
        let debug_str_empty = format!("{:?}", filler_empty);

        // THEN: No panic
        // AND: Contains type name
        assert!(debug_str_empty.contains("ChainIdFiller"));
    }
}

