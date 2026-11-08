use crate::{
    fillers::{FillerControlFlow, TxFiller},
    provider::SendableTx,
    Provider,
};
use alloy_network::{Network, TransactionBuilder};
use alloy_primitives::Address;
use alloy_transport::{TransportError, TransportResult};
use async_trait::async_trait;
use dashmap::DashMap;
use futures::lock::Mutex;
use std::{collections::BTreeSet, sync::Arc};

/// A trait that determines the behavior of filling nonces.
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
pub trait NonceManager: Clone + Send + Sync + std::fmt::Debug {
    /// Get the next nonce for the given account.
    async fn get_next_nonce<P, N>(&self, provider: &P, address: Address) -> TransportResult<u64>
    where
        P: Provider<N>,
        N: Network;

    /// Called after sending a transaction fails, with the nonce this manager handed out for it, so
    /// that managers that reserve nonces locally can release or resynchronize it.
    ///
    /// This is a no-op by default.
    async fn on_send_failure(&self, address: Address, nonce: u64, error: &TransportError) {
        let _ = (address, nonce, error);
    }
}

/// This [`NonceManager`] fetches the pending transaction count on every request and does not
/// reserve nonces locally. It makes more RPC calls than [`CachedNonceManager`] and is more
/// resilient to chain reorganizations, but concurrent transactions from the same address can
/// receive the same nonce.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct SimpleNonceManager;

#[cfg_attr(target_family = "wasm", async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
impl NonceManager for SimpleNonceManager {
    async fn get_next_nonce<P, N>(&self, provider: &P, address: Address) -> TransportResult<u64>
    where
        P: Provider<N>,
        N: Network,
    {
        provider.get_transaction_count(address).pending().await
    }
}

/// Cached nonce manager
///
/// This [`NonceManager`] fetches the pending transaction count the first time it sees an account,
/// then reserves subsequent nonces locally as transactions are filled. Clones share the same
/// per-address state, so they can allocate distinct nonces to concurrent requests.
///
/// The local nonce advances before broadcast. When sending fails, see
/// [`NonceManager::on_send_failure`], a nonce whose transaction the node definitively rejected for
/// a reason unrelated to the nonce is released and handed out again, lowest first. After a nonce
/// error, or an error that leaves it unknown whether the node accepted the transaction (e.g. a
/// transport failure), the account is reset and its nonce is fetched again on the next request. A
/// chain reorganization or a transaction sent through another manager can still leave it out of
/// sync with the node until a send fails with a nonce error.
///
/// There is also an alternative implementation [`SimpleNonceManager`] that does not store the
/// transaction count locally.
#[derive(Clone, Debug, Default)]
pub struct CachedNonceManager {
    nonces: Arc<DashMap<Address, Arc<Mutex<AccountNonce>>>>,
}

/// The nonces a [`CachedNonceManager`] tracks for an account.
#[derive(Debug, Default)]
struct AccountNonce {
    /// The next nonce to hand out, or `None` if it has to be fetched first.
    next: Option<u64>,
    /// Nonces below `next` that were released after their transaction was rejected.
    released: BTreeSet<u64>,
}

#[cfg_attr(target_family = "wasm", async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
impl NonceManager for CachedNonceManager {
    async fn get_next_nonce<P, N>(&self, provider: &P, address: Address) -> TransportResult<u64>
    where
        P: Provider<N>,
        N: Network,
    {
        // Locks dashmap internally for a short duration to clone the `Arc`.
        // We also don't want to hold the dashmap lock through the await point below.
        let account = {
            let rm = self.nonces.entry(address).or_default();
            Arc::clone(rm.value())
        };

        let mut account = account.lock().await;
        if let Some(nonce) = account.released.pop_first() {
            trace!(%address, nonce, "reusing released nonce");
            return Ok(nonce);
        }

        let nonce = match account.next {
            Some(nonce) => {
                trace!(%address, nonce, "incrementing nonce");
                nonce
            }
            None => {
                // Initialize the nonce if we haven't seen this account before, or it was reset.
                trace!(%address, "fetching nonce");
                provider.get_transaction_count(address).pending().await?
            }
        };
        account.next = Some(nonce + 1);
        Ok(nonce)
    }

    async fn on_send_failure(&self, address: Address, nonce: u64, error: &TransportError) {
        let Some(account) = self.nonces.get(&address).map(|rm| Arc::clone(rm.value())) else {
            return;
        };

        let mut account = account.lock().await;
        if is_definitive_rejection(error) {
            // Only nonces below `next` have been handed out since the nonce was last fetched.
            if account.next.is_some_and(|next| nonce < next) {
                trace!(%address, nonce, "releasing nonce");
                account.released.insert(nonce);
            }
        } else {
            trace!(%address, nonce, %error, "resetting nonce");
            *account = AccountNonce::default();
        }
    }
}

/// Returns `true` if sending a transaction failed with an error that definitively rejects it for a
/// reason unrelated to its nonce, so that its nonce was not used and can be handed out again.
///
/// This is a JSON-RPC error response, unless:
/// - its message is about the nonce (e.g. "nonce too low" or "already known"), which means the
///   local nonce is out of sync with the node;
/// - it carries a transaction hash, which an [EIP-7966] `eth_sendRawTransactionSync` timeout
///   returns for a transaction the node accepted.
///
/// Any other error, such as a transport failure, a timeout or a response that fails to
/// deserialize, leaves it unknown whether the node accepted the transaction. Reusing its nonce
/// could then silently replace it.
///
/// [EIP-7966]: https://eips.ethereum.org/EIPS/eip-7966
fn is_definitive_rejection(error: &TransportError) -> bool {
    const NONCE_ERRORS: &[&str] = &[
        "nonce",
        "already known",
        "known transaction",
        "already exists",
        "already imported",
        "replacement transaction underpriced",
    ];

    let Some(payload) = error.as_error_resp() else { return false };
    let message = payload.message.to_lowercase();
    if NONCE_ERRORS.iter().any(|pattern| message.contains(pattern)) {
        return false;
    }
    error.tx_hash_data().is_none()
}

/// A [`TxFiller`] that fills nonces on transactions. The behavior of filling nonces is determined
/// by the [`NonceManager`].
///
/// # Note
///
/// - If the transaction request does not have a sender set, this layer will not fill nonces.
/// - An explicit nonce is preserved and does not consult the manager.
/// - If sending a transaction fails, the nonce this layer filled in is passed to
///   [`NonceManager::on_send_failure`], which lets [`CachedNonceManager`] release or resynchronize
///   it. Nonces reserved by direct [`FillProvider::fill`](crate::fillers::FillProvider::fill)
///   calls, or by a fill that fails before sending (e.g. in gas estimation), are not reported.
/// - For concurrent sends from one address, reuse a provider or cloned [`CachedNonceManager`]. Two
///   independent cached managers can allocate conflicting nonces, while [`SimpleNonceManager`] does
///   not reserve nonces between concurrent requests.
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
/// let provider = ProviderBuilder::<_, _, Ethereum>::default()
///     .with_simple_nonce_management()
///     .wallet(pk)
///     .connect_http(url);
///
/// provider.send_transaction(TransactionRequest::default()).await;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Default)]
pub struct NonceFiller<M: NonceManager = CachedNonceManager> {
    nonce_manager: M,
}

impl<M: NonceManager> NonceFiller<M> {
    /// Creates a new [`NonceFiller`] with the specified [`NonceManager`].
    ///
    /// To instantiate with the [`SimpleNonceManager`], use [`NonceFiller::simple()`].
    ///
    /// To instantiate with the [`CachedNonceManager`], use [`NonceFiller::cached()`].
    pub const fn new(nonce_manager: M) -> Self {
        Self { nonce_manager }
    }

    /// Creates a new [`NonceFiller`] with the [`SimpleNonceManager`].
    ///
    /// [`SimpleNonceManager`] fetches the pending transaction count for every request, resulting in
    /// frequent RPC calls and no local nonce reservation.
    pub const fn simple() -> NonceFiller<SimpleNonceManager> {
        NonceFiller { nonce_manager: SimpleNonceManager }
    }

    /// Creates a new [`NonceFiller`] with the [`CachedNonceManager`].
    ///
    /// [`CachedNonceManager`] will fetch the transaction count for any new account it sees,
    /// store it locally, and increment the locally stored nonce as transactions are filled,
    /// reducing the number of RPC calls. Reservation happens before broadcast and also applies to
    /// direct [`FillProvider::fill`](crate::fillers::FillProvider::fill) calls.
    pub fn cached() -> NonceFiller<CachedNonceManager> {
        NonceFiller { nonce_manager: CachedNonceManager::default() }
    }

    /// Get a reference to the nonce manager.
    pub const fn nonce_manager(&self) -> &M {
        &self.nonce_manager
    }

    /// Get a mutable reference to the nonce manager.
    pub const fn nonce_manager_mut(&mut self) -> &mut M {
        &mut self.nonce_manager
    }
}

impl<M: NonceManager, N: Network> TxFiller<N> for NonceFiller<M> {
    type Fillable = u64;

    fn status(&self, tx: &<N as Network>::TransactionRequest) -> FillerControlFlow {
        if tx.nonce().is_some() {
            return FillerControlFlow::Finished;
        }
        if tx.from().is_none() {
            return FillerControlFlow::missing("NonceManager", vec!["from"]);
        }
        FillerControlFlow::Ready
    }

    fn fill_sync(&self, _tx: &mut SendableTx<N>) {}

    async fn prepare<P>(
        &self,
        provider: &P,
        tx: &N::TransactionRequest,
    ) -> TransportResult<Self::Fillable>
    where
        P: Provider<N>,
    {
        let from = tx.from().expect("checked by 'ready()'");
        self.nonce_manager.get_next_nonce(provider, from).await
    }

    async fn fill(
        &self,
        nonce: Self::Fillable,
        mut tx: SendableTx<N>,
    ) -> TransportResult<SendableTx<N>> {
        if let Some(builder) = tx.as_mut_builder() {
            builder.set_nonce(nonce);
        }
        Ok(tx)
    }

    async fn on_send_failure(&self, from: Address, nonce: u64, error: &TransportError) {
        self.nonce_manager.on_send_failure(from, nonce, error).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProviderBuilder, RootProvider, WalletProvider};
    use alloy_consensus::Transaction;
    use alloy_json_rpc::ErrorPayload;
    use alloy_network::NetworkTransactionBuilder;
    use alloy_node_bindings::Anvil;
    use alloy_primitives::{address, U256, U64};
    use alloy_rpc_client::RpcClient;
    use alloy_rpc_types_eth::TransactionRequest;
    use alloy_transport::{mock::Asserter, TransportErrorKind};

    async fn check_nonces<P, N, M>(
        filler: &NonceFiller<M>,
        provider: &P,
        address: Address,
        start: u64,
    ) where
        P: Provider<N>,
        N: Network,
        M: NonceManager,
    {
        for i in start..start + 5 {
            let nonce = filler.nonce_manager.get_next_nonce(&provider, address).await.unwrap();
            assert_eq!(nonce, i);
        }
    }

    #[tokio::test]
    async fn smoke_test() {
        let filler = NonceFiller::<CachedNonceManager>::default();
        let provider = ProviderBuilder::new().connect_anvil();
        let address = Address::ZERO;
        check_nonces(&filler, &provider, address, 0).await;

        #[cfg(feature = "anvil-api")]
        {
            use crate::ext::AnvilApi;
            filler.nonce_manager.nonces.clear();
            provider.anvil_set_nonce(address, 69).await.unwrap();
            check_nonces(&filler, &provider, address, 69).await;
        }
    }

    #[tokio::test]
    async fn concurrency() {
        let filler = Arc::new(NonceFiller::<CachedNonceManager>::default());
        let provider = Arc::new(ProviderBuilder::new().connect_anvil());
        let address = Address::ZERO;
        let concurrent_nonces = || async {
            let tasks = (0..5)
                .map(|_| {
                    let filler = Arc::clone(&filler);
                    let provider = Arc::clone(&provider);
                    tokio::spawn(async move {
                        filler.nonce_manager.get_next_nonce(&provider, address).await
                    })
                })
                .collect::<Vec<_>>();

            let mut ns = Vec::new();
            for task in tasks {
                ns.push(task.await.unwrap().unwrap());
            }
            ns.sort_unstable();
            ns
        };

        assert_eq!(concurrent_nonces().await, (0..5).collect::<Vec<_>>());
        assert_eq!(filler.nonce_manager.nonces.len(), 1);
        assert_eq!(
            filler.nonce_manager.nonces.get(&address).unwrap().value().lock().await.next,
            Some(5)
        );

        // Released nonces are handed out alongside new ones without duplicates.
        for nonce in [3, 1] {
            filler.nonce_manager.on_send_failure(address, nonce, &rejection()).await;
        }
        assert_eq!(concurrent_nonces().await, [1, 3, 5, 6, 7]);
    }

    #[tokio::test]
    async fn increments_nonce() {
        let provider = ProviderBuilder::new()
            .disable_recommended_fillers()
            .with_cached_nonce_management()
            .connect_anvil_with_wallet();

        let from = provider.default_signer_address();
        let tx = TransactionRequest {
            from: Some(from),
            value: Some(U256::from(100)),
            to: Some(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into()),
            gas_price: Some(20e9 as u128),
            gas: Some(21000),
            ..Default::default()
        };

        let pending = provider.send_transaction(tx.clone()).await.unwrap();
        let tx_hash = pending.watch().await.unwrap();
        let mined_tx = provider
            .get_transaction_by_hash(tx_hash)
            .await
            .expect("failed to fetch tx")
            .expect("tx not included");
        assert_eq!(mined_tx.nonce(), 0);

        let pending = provider.send_transaction(tx).await.unwrap();
        let tx_hash = pending.watch().await.unwrap();
        let mined_tx = provider
            .get_transaction_by_hash(tx_hash)
            .await
            .expect("fail to fetch tx")
            .expect("tx didn't finalize");
        assert_eq!(mined_tx.nonce(), 1);
    }

    #[tokio::test]
    async fn cloned_managers() {
        let cnm1 = CachedNonceManager::default();
        let cnm2 = cnm1.clone();

        let provider = ProviderBuilder::new().connect_anvil();
        let address = Address::ZERO;

        assert_eq!(cnm1.get_next_nonce(&provider, address).await.unwrap(), 0);
        assert_eq!(cnm2.get_next_nonce(&provider, address).await.unwrap(), 1);
        assert_eq!(cnm1.get_next_nonce(&provider, address).await.unwrap(), 2);
        assert_eq!(cnm2.get_next_nonce(&provider, address).await.unwrap(), 3);
    }

    /// Returns a provider whose responses, e.g. to `eth_getTransactionCount`, are queued on the
    /// returned asserter.
    fn mocked_provider() -> (RootProvider, Asserter) {
        let asserter = Asserter::new();
        (RootProvider::new(RpcClient::mocked(asserter.clone())), asserter)
    }

    /// An error response from the node with the given message.
    fn error_resp(message: &'static str) -> TransportError {
        TransportError::ErrorResp(ErrorPayload {
            code: -32000,
            message: message.into(),
            data: None,
        })
    }

    /// An error response that rejects the transaction for a reason unrelated to its nonce.
    fn rejection() -> TransportError {
        error_resp("insufficient funds for gas * price + value")
    }

    /// An [EIP-7966] timeout for a transaction the node accepted.
    ///
    /// [EIP-7966]: https://eips.ethereum.org/EIPS/eip-7966
    fn sync_send_timeout() -> TransportError {
        let json = r#"{"code":4,"message":"The transaction was added to the transaction pool but wasn't processed in 2s","data":"0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"}"#;
        TransportError::ErrorResp(serde_json::from_str(json).unwrap())
    }

    /// Gets the next nonce for the zero address.
    async fn next_nonce(manager: &CachedNonceManager, provider: &RootProvider) -> u64 {
        manager.get_next_nonce(provider, Address::ZERO).await.unwrap()
    }

    #[test]
    fn classifies_send_failures() {
        for message in [
            "insufficient funds for gas * price + value",
            "intrinsic gas too low",
            "transaction underpriced",
            "max fee per gas less than block base fee",
        ] {
            assert!(is_definitive_rejection(&error_resp(message)), "{message}");
        }

        for message in [
            "nonce too low",
            "nonce too high",
            "invalid nonce",
            "Nonce too low",
            "already known",
            "known transaction: 0x1234",
            "transaction already exists",
            "transaction already imported",
            "replacement transaction underpriced",
        ] {
            assert!(!is_definitive_rejection(&error_resp(message)), "{message}");
        }

        assert!(!is_definitive_rejection(&sync_send_timeout()));
        assert!(!is_definitive_rejection(&TransportErrorKind::custom_str("connection reset")));
        assert!(!is_definitive_rejection(&TransportError::deser_err(
            serde_json::from_str::<u64>("").unwrap_err(),
            ""
        )));
    }

    // <https://github.com/alloy-rs/alloy/issues/2385>
    #[tokio::test]
    async fn reuses_released_nonce() {
        let (provider, asserter) = mocked_provider();
        let manager = CachedNonceManager::default();
        asserter.push_success(&U64::from(1));

        // tx1, tx2 and tx3 get nonces 1, 2 and 3, then tx1 is rejected.
        for nonce in 1..=3 {
            assert_eq!(next_nonce(&manager, &provider).await, nonce);
        }
        manager.on_send_failure(Address::ZERO, 1, &rejection()).await;

        // tx4 reuses the released nonce and tx5 continues after tx3.
        assert_eq!(next_nonce(&manager, &provider).await, 1);
        assert_eq!(next_nonce(&manager, &provider).await, 4);
        assert_eq!(asserter.requests().len(), 1);
    }

    #[tokio::test]
    async fn reuses_released_nonces_lowest_first() {
        let (provider, asserter) = mocked_provider();
        let manager = CachedNonceManager::default();
        asserter.push_success(&U64::from(0));

        for nonce in 0..5 {
            assert_eq!(next_nonce(&manager, &provider).await, nonce);
        }
        for nonce in [3, 1, 2] {
            manager.on_send_failure(Address::ZERO, nonce, &rejection()).await;
        }

        for nonce in [1, 2, 3, 5] {
            assert_eq!(next_nonce(&manager, &provider).await, nonce);
        }
        assert_eq!(asserter.requests().len(), 1);
    }

    #[tokio::test]
    async fn resyncs_after_nonce_or_transport_error() {
        for error in [
            error_resp("nonce too low"),
            error_resp("already known"),
            error_resp("replacement transaction underpriced"),
            sync_send_timeout(),
            TransportErrorKind::custom_str("connection reset"),
        ] {
            let (provider, asserter) = mocked_provider();
            let manager = CachedNonceManager::default();
            asserter.push_success(&U64::from(0));

            for nonce in 0..3 {
                assert_eq!(next_nonce(&manager, &provider).await, nonce);
            }
            // The released nonce is dropped by the reset.
            manager.on_send_failure(Address::ZERO, 0, &rejection()).await;
            manager.on_send_failure(Address::ZERO, 2, &error).await;

            asserter.push_success(&U64::from(5));
            assert_eq!(next_nonce(&manager, &provider).await, 5, "{error}");
            assert_eq!(next_nonce(&manager, &provider).await, 6, "{error}");
            assert_eq!(asserter.requests().len(), 2, "{error}");
        }
    }

    #[tokio::test]
    async fn ignores_release_of_nonce_not_handed_out() {
        let (provider, asserter) = mocked_provider();
        let manager = CachedNonceManager::default();

        manager.on_send_failure(Address::ZERO, 0, &rejection()).await;
        assert!(manager.nonces.is_empty());

        asserter.push_success(&U64::from(0));
        assert_eq!(next_nonce(&manager, &provider).await, 0);
        assert_eq!(next_nonce(&manager, &provider).await, 1);
        for nonce in [2, 5] {
            manager.on_send_failure(Address::ZERO, nonce, &rejection()).await;
        }
        assert_eq!(next_nonce(&manager, &provider).await, 2);

        // Nothing has been handed out since the reset.
        manager.on_send_failure(Address::ZERO, 2, &error_resp("nonce too low")).await;
        manager.on_send_failure(Address::ZERO, 1, &rejection()).await;
        asserter.push_success(&U64::from(3));
        assert_eq!(next_nonce(&manager, &provider).await, 3);
        assert_eq!(next_nonce(&manager, &provider).await, 4);
        assert_eq!(asserter.requests().len(), 2);
    }

    /// A transfer with its gas and fees set, so that the node rather than gas estimation rejects
    /// it if the value exceeds the balance.
    fn transfer(value: U256) -> TransactionRequest {
        TransactionRequest {
            value: Some(value),
            to: Some(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into()),
            gas_price: Some(20e9 as u128),
            gas: Some(21000),
            ..Default::default()
        }
    }

    /// Sends a transfer, checks its nonce, then waits for it to be mined.
    ///
    /// The nonce is checked first, since a transaction after a nonce gap is never mined.
    async fn send_with_nonce<P: Provider>(provider: &P, nonce: u64) {
        let pending = provider.send_transaction(transfer(U256::from(100))).await.unwrap();
        let tx = provider.get_transaction_by_hash(*pending.tx_hash()).await.unwrap().unwrap();
        assert_eq!(tx.nonce(), nonce);
        assert!(pending.get_receipt().await.unwrap().status());
    }

    #[tokio::test]
    async fn rejected_send_releases_nonce() {
        let provider = ProviderBuilder::new().connect_anvil_with_wallet();
        let balance = provider.get_balance(provider.default_signer_address()).await.unwrap();

        let err = provider.send_transaction(transfer(balance + U256::from(1))).await.unwrap_err();
        assert!(err.as_error_resp().is_some(), "{err}");
        send_with_nonce(&provider, 0).await;

        let err =
            provider.send_transaction_sync(transfer(balance + U256::from(1))).await.unwrap_err();
        assert!(err.as_error_resp().is_some(), "{err}");
        let receipt = provider.send_transaction_sync(transfer(U256::from(100))).await.unwrap();
        let tx = provider.get_transaction_by_hash(receipt.transaction_hash).await.unwrap().unwrap();
        assert_eq!(tx.nonce(), 1);
    }

    #[tokio::test]
    async fn nonce_error_resyncs() {
        let anvil = Anvil::new().spawn();
        let wallet = anvil.wallet().unwrap();
        let provider =
            ProviderBuilder::new().wallet(wallet.clone()).connect_http(anvil.endpoint_url());
        let other = ProviderBuilder::new().wallet(wallet).connect_http(anvil.endpoint_url());

        send_with_nonce(&provider, 0).await;
        // Use the next nonces behind the provider's back.
        send_with_nonce(&other, 1).await;
        send_with_nonce(&other, 2).await;

        let err = provider.send_transaction(transfer(U256::from(100))).await.unwrap_err();
        assert!(err.to_string().contains("nonce"), "{err}");
        send_with_nonce(&provider, 3).await;
    }

    /// A [`CachedNonceManager`] that records the nonces reported to it.
    #[derive(Clone, Debug, Default)]
    struct RecordingNonceManager {
        inner: CachedNonceManager,
        failures: Arc<std::sync::Mutex<Vec<(Address, u64)>>>,
    }

    #[async_trait]
    impl NonceManager for RecordingNonceManager {
        async fn get_next_nonce<P, N>(&self, provider: &P, address: Address) -> TransportResult<u64>
        where
            P: Provider<N>,
            N: Network,
        {
            self.inner.get_next_nonce(provider, address).await
        }

        async fn on_send_failure(&self, address: Address, nonce: u64, error: &TransportError) {
            self.failures.lock().unwrap().push((address, nonce));
            self.inner.on_send_failure(address, nonce, error).await;
        }
    }

    #[tokio::test]
    async fn explicit_nonce_is_not_reported() {
        let manager = RecordingNonceManager::default();
        let provider = ProviderBuilder::new()
            .disable_recommended_fillers()
            .with_gas_estimation()
            .with_nonce_management(manager.clone())
            .fetch_chain_id()
            .connect_anvil_with_wallet();
        let from = provider.default_signer_address();
        let balance = provider.get_balance(from).await.unwrap();

        // Explicit nonces on requests and signed envelopes, rejected or too low.
        let err = provider
            .send_transaction(transfer(balance + U256::from(1)).nonce(0))
            .await
            .unwrap_err();
        assert!(err.as_error_resp().is_some(), "{err}");

        send_with_nonce(&provider, 0).await;
        let err = provider.send_transaction(transfer(U256::from(100)).nonce(0)).await.unwrap_err();
        assert!(err.to_string().contains("nonce"), "{err}");

        let envelope = transfer(U256::from(100))
            .nonce(0)
            .with_chain_id(provider.get_chain_id().await.unwrap())
            .build(provider.wallet())
            .await
            .unwrap();
        let err = provider.send_tx_envelope(envelope).await.unwrap_err();
        assert!(err.to_string().contains("nonce"), "{err}");
        assert!(manager.failures.lock().unwrap().is_empty());

        // A nonce chosen by the fillers is reported.
        let err = provider.send_transaction(transfer(balance + U256::from(1))).await.unwrap_err();
        assert!(err.as_error_resp().is_some(), "{err}");
        assert_eq!(*manager.failures.lock().unwrap(), [(from, 1)]);
    }
}
