use crate::Error;
use alloy_network::Ethereum;
use alloy_primitives::{Address, LogData, B256};
use alloy_provider::{FilterPollerBuilder, Network, Provider};
use alloy_rpc_types_eth::{BlockNumberOrTag, Filter, FilterBlockOption, Log, Topic, ValueOrArray};
use alloy_sol_types::SolEvent;
use alloy_transport::{RpcError, TransportResult};
use futures::Stream;
use futures_util::StreamExt;
use std::{fmt, marker::PhantomData};

/// Helper for managing the event filter before querying or streaming its logs
#[must_use = "event filters do nothing unless you `query`, `watch`, or `stream` them"]
pub struct Event<P, E, N = Ethereum> {
    /// The provider to use for querying or streaming logs.
    pub provider: P,
    /// The filter to use for querying or streaming logs.
    pub filter: Filter,
    _phantom: PhantomData<(E, N)>,
}

impl<P: fmt::Debug, E, N> fmt::Debug for Event<P, E, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Event")
            .field("provider", &self.provider)
            .field("filter", &self.filter)
            .field("event_type", &format_args!("{}", std::any::type_name::<E>()))
            .finish()
    }
}

#[doc(hidden)]
impl<'a, P: Provider<N>, E: SolEvent, N: Network> Event<&'a P, E, N> {
    // `sol!` macro constructor, see `#[sol(rpc)]`. Not public API.
    // NOTE: please avoid changing this function due to its use in the `sol!` macro.
    pub fn new_sol(provider: &'a P, address: &Address) -> Self {
        // keccak256 hash of the event signature needed for the filter to actually filter by event
        // check that the event is not anonymous to include the event signature in the filter
        if E::ANONYMOUS {
            Self::new(provider, Filter::new().address(*address))
        } else {
            Self::new(provider, Filter::new().address(*address).event_signature(E::SIGNATURE_HASH))
        }
    }
}

impl<P: Provider<N>, E: SolEvent, N: Network> Event<P, E, N> {
    /// Creates a new event with the provided provider and filter.
    pub const fn new(provider: P, filter: Filter) -> Self {
        Self { provider, filter, _phantom: PhantomData }
    }

    /// Queries the blockchain for the selected filter and returns a vector of matching event logs.
    pub async fn query(&self) -> Result<Vec<(E, Log)>, Error> {
        let logs = self.query_raw().await?;
        logs.into_iter().map(|log| Ok((decode_log(&log)?, log))).collect()
    }

    /// Queries the blockchain for the selected filter and returns a vector of matching event logs,
    /// without decoding them.
    pub async fn query_raw(&self) -> TransportResult<Vec<Log>> {
        self.provider.get_logs(&self.filter).await
    }

    /// Watches for events that match the filter.
    ///
    /// Returns a stream of decoded events and raw logs.
    #[doc(alias = "stream")]
    #[doc(alias = "stream_with_meta")]
    pub async fn watch(&self) -> TransportResult<EventPoller<E>> {
        let poller = self.provider.watch_logs(&self.filter).await?;
        Ok(poller.into())
    }

    /// Subscribes to the stream of events that match the filter.
    ///
    /// Returns a stream of decoded events and raw logs.
    #[cfg(feature = "pubsub")]
    pub async fn subscribe(&self) -> TransportResult<subscription::EventSubscription<E>> {
        let sub = self.provider.subscribe_logs(&self.filter).await?;
        Ok(sub.into())
    }

    /// Sets the inner filter object
    ///
    /// See [`Filter::select`].
    pub fn select(mut self, filter: impl Into<FilterBlockOption>) -> Self {
        self.filter.block_option = filter.into();
        self
    }

    /// Sets the from block number
    pub fn from_block<B: Into<BlockNumberOrTag>>(mut self, block: B) -> Self {
        self.filter.block_option = self.filter.block_option.with_from_block(block.into());
        self
    }

    /// Sets the to block number
    pub fn to_block<B: Into<BlockNumberOrTag>>(mut self, block: B) -> Self {
        self.filter.block_option = self.filter.block_option.with_to_block(block.into());
        self
    }

    /// Return `true` if filter configured to match pending block.
    ///
    /// This means that both `from_block` and `to_block` are set to the pending
    /// tag.
    pub fn is_pending_block_filter(&self) -> bool {
        self.filter.block_option.get_from_block().is_some_and(BlockNumberOrTag::is_pending)
            && self.filter.block_option.get_to_block().is_some_and(BlockNumberOrTag::is_pending)
    }

    /// Pins the block hash for the filter
    pub fn at_block_hash<A: Into<B256>>(mut self, hash: A) -> Self {
        self.filter.block_option = self.filter.block_option.with_block_hash(hash.into());
        self
    }

    /// Sets the address to query with this filter.
    ///
    /// See [`Filter::address`].
    pub fn address<A: Into<ValueOrArray<Address>>>(mut self, address: A) -> Self {
        self.filter.address = address.into().into();
        self
    }

    /// Given the event signature in string form, it hashes it and adds it to the topics to monitor
    pub fn event(mut self, event_name: &str) -> Self {
        self.filter = self.filter.event(event_name);
        self
    }

    /// Hashes all event signatures and sets them as array to event_signature(topic0)
    pub fn events(mut self, events: impl IntoIterator<Item = impl AsRef<[u8]>>) -> Self {
        self.filter = self.filter.events(events);
        self
    }

    /// Sets event_signature(topic0) (the event name for non-anonymous events)
    pub fn event_signature<TO: Into<Topic>>(mut self, topic: TO) -> Self {
        self.filter.topics[0] = topic.into();
        self
    }

    /// Sets the 1st indexed topic
    pub fn topic1<TO: Into<Topic>>(mut self, topic: TO) -> Self {
        self.filter.topics[1] = topic.into();
        self
    }

    /// Sets the 2nd indexed topic
    pub fn topic2<TO: Into<Topic>>(mut self, topic: TO) -> Self {
        self.filter.topics[2] = topic.into();
        self
    }

    /// Sets the 3rd indexed topic
    pub fn topic3<TO: Into<Topic>>(mut self, topic: TO) -> Self {
        self.filter.topics[3] = topic.into();
        self
    }
}

impl<P: Provider<N> + Clone, E: SolEvent, N: Network> Event<P, E, N> {
    /// Queries the blockchain for the selected filter using chunked requests to handle large
    /// block ranges, and returns a vector of matching event logs.
    ///
    /// First tries the full range optimistically. If that fails, splits the range into chunks
    /// of `chunk_size` blocks and queries them concurrently (up to 5 in parallel).
    /// If an individual chunk fails, falls back to block-by-block queries for that chunk.
    pub async fn query_chunked(&self, chunk_size: u64) -> Result<Vec<(E, Log)>, Error> {
        let logs = self.query_raw_chunked(chunk_size).await?;
        logs.into_iter().map(|log| Ok((decode_log(&log)?, log))).collect()
    }

    /// Queries the blockchain for the selected filter using chunked requests to handle large
    /// block ranges, and returns a vector of matching event logs, without decoding them.
    ///
    /// First tries the full range optimistically. If that fails, splits the range into chunks
    /// of `chunk_size` blocks and queries them concurrently (up to 5 in parallel).
    /// If an individual chunk fails, falls back to block-by-block queries for that chunk.
    pub async fn query_raw_chunked(&self, chunk_size: u64) -> TransportResult<Vec<Log>> {
        if chunk_size == 0 {
            return Err(RpcError::local_usage_str("chunk_size must be greater than 0"));
        }

        // Try the full range first
        if let Ok(logs) = self.provider.get_logs(&self.filter).await {
            return Ok(logs);
        }

        // Full-range failed; return the chunked fallback result (including its own error)
        self.get_logs_chunked_concurrent(chunk_size).await
    }

    /// Retrieves logs using concurrent chunked requests with rate limiting.
    ///
    /// Divides the block range into chunks and processes them with a maximum of
    /// 5 concurrent requests. Falls back to single-block queries if chunks fail.
    async fn get_logs_chunked_concurrent(&self, chunk_size: u64) -> TransportResult<Vec<Log>> {
        let (from_block, to_block) = extract_block_range(&self.filter);
        let (Some(from), Some(to)) = (from_block, to_block) else {
            // No concrete numeric range; chunking is not possible
            return Err(RpcError::local_usage_str(
                "chunked queries require numeric from_block and to_block",
            ));
        };

        if from > to {
            return Ok(vec![]);
        }

        // Create chunk ranges lazily (inclusive on both ends) using u64 arithmetic
        // to avoid OOM on huge ranges.
        let chunk_ranges = std::iter::successors(Some(from), move |&prev| {
            let end = prev.saturating_add(chunk_size - 1).min(to);
            if end >= to {
                None
            } else {
                end.checked_add(1)
            }
        })
        .map(move |chunk_start| (chunk_start, chunk_start.saturating_add(chunk_size - 1).min(to)));

        // Process chunks with controlled concurrency using buffered stream
        let all_results: Vec<TransportResult<(u64, Vec<Log>)>> =
            futures::stream::iter(chunk_ranges)
                .map(|(start_block, end_block)| {
                    let chunk_filter =
                        self.filter.clone().from_block(start_block).to_block(end_block);
                    let provider = self.provider.clone();

                    async move {
                        match provider.get_logs(&chunk_filter).await {
                            Ok(logs) => Ok((start_block, logs)),
                            Err(_) => {
                                // Fallback: try individual blocks in this chunk (best-effort)
                                let mut fallback_logs = Vec::new();
                                for block in start_block..=end_block {
                                    let single_filter =
                                        chunk_filter.clone().from_block(block).to_block(block);
                                    if let Ok(logs) = provider.get_logs(&single_filter).await {
                                        fallback_logs.extend(logs);
                                    }
                                }
                                Ok((start_block, fallback_logs))
                            }
                        }
                    }
                })
                .buffered(5)
                .collect()
                .await;

        // Collect results, propagating any errors
        let mut resolved: Vec<(u64, Vec<Log>)> =
            all_results.into_iter().collect::<TransportResult<Vec<_>>>()?;

        // Sort by start block and flatten
        resolved.sort_by_key(|(block_num, _)| *block_num);
        Ok(resolved.into_iter().flat_map(|(_, logs)| logs).collect())
    }
}

impl<P: Clone, E, N> Event<&P, E, N> {
    /// Clones the provider and returns a new event with the cloned provider.
    pub fn with_cloned_provider(self) -> Event<P, E, N> {
        Event { provider: self.provider.clone(), filter: self.filter, _phantom: PhantomData }
    }
}

/// An event poller.
///
/// Polling configuration is available through the [`poller`](Self::poller) field.
pub struct EventPoller<E> {
    /// The inner poller.
    pub poller: FilterPollerBuilder<Log>,
    _phantom: PhantomData<E>,
}

impl<E> AsRef<FilterPollerBuilder<Log>> for EventPoller<E> {
    #[inline]
    fn as_ref(&self) -> &FilterPollerBuilder<Log> {
        &self.poller
    }
}

impl<E> AsMut<FilterPollerBuilder<Log>> for EventPoller<E> {
    #[inline]
    fn as_mut(&mut self) -> &mut FilterPollerBuilder<Log> {
        &mut self.poller
    }
}

impl<E> fmt::Debug for EventPoller<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventPoller")
            .field("poller", &self.poller)
            .field("event_type", &format_args!("{}", std::any::type_name::<E>()))
            .finish()
    }
}

impl<E> From<FilterPollerBuilder<Log>> for EventPoller<E> {
    fn from(poller: FilterPollerBuilder<Log>) -> Self {
        Self { poller, _phantom: PhantomData }
    }
}

impl<E: SolEvent> EventPoller<E> {
    /// Starts the poller and returns a stream that yields the decoded event and the raw log.
    ///
    /// Note that this stream will not return `None` until the provider is dropped.
    pub fn into_stream(self) -> impl Stream<Item = alloy_sol_types::Result<(E, Log)>> + Unpin {
        self.poller
            .into_stream()
            .flat_map(futures_util::stream::iter)
            .map(|log| decode_log(&log).map(|e| (e, log)))
    }
}

fn extract_block_range(filter: &Filter) -> (Option<u64>, Option<u64>) {
    let FilterBlockOption::Range { from_block, to_block } = &filter.block_option else {
        return (None, None);
    };
    (from_block.and_then(|b| b.as_number()), to_block.and_then(|b| b.as_number()))
}

fn decode_log<E: SolEvent>(log: &Log) -> alloy_sol_types::Result<E> {
    let log_data: &LogData = log.as_ref();

    E::decode_raw_log(log_data.topics().iter().copied(), &log_data.data)
}

#[cfg(feature = "pubsub")]
pub(crate) mod subscription {
    use super::*;
    use alloy_pubsub::Subscription;

    /// An event subscription.
    ///
    /// Underlying subscription is available through the [`sub`](Self::sub) field.
    pub struct EventSubscription<E> {
        /// The inner poller.
        pub sub: Subscription<Log>,
        _phantom: PhantomData<E>,
    }

    impl<E> AsRef<Subscription<Log>> for EventSubscription<E> {
        #[inline]
        fn as_ref(&self) -> &Subscription<Log> {
            &self.sub
        }
    }

    impl<E> AsMut<Subscription<Log>> for EventSubscription<E> {
        #[inline]
        fn as_mut(&mut self) -> &mut Subscription<Log> {
            &mut self.sub
        }
    }

    impl<E> fmt::Debug for EventSubscription<E> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("EventSubscription")
                .field("sub", &self.sub)
                .field("event_type", &format_args!("{}", std::any::type_name::<E>()))
                .finish()
        }
    }

    impl<E> From<Subscription<Log>> for EventSubscription<E> {
        fn from(sub: Subscription<Log>) -> Self {
            Self { sub, _phantom: PhantomData }
        }
    }

    impl<E: SolEvent> EventSubscription<E> {
        /// Converts the subscription into a stream.
        pub fn into_stream(self) -> impl Stream<Item = alloy_sol_types::Result<(E, Log)>> + Unpin {
            self.sub.into_stream().map(|log| decode_log(&log).map(|e| (e, log)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_network::EthereumWallet;
    use alloy_primitives::U256;
    use alloy_signer_local::PrivateKeySigner;
    use alloy_sol_types::sol;

    sol! {
        // solc v0.8.24; solc a.sol --via-ir --optimize --bin
        #[sol(rpc, bytecode = "60808060405234601557610147908161001a8239f35b5f80fdfe6080806040526004361015610012575f80fd5b5f3560e01c908163299d8665146100a7575063ffdf4f1b14610032575f80fd5b346100a3575f3660031901126100a357602a7f6d10b8446ff0ac11bb95d154e7b10a73042fb9fc3bca0c92de5397b2fe78496c6040518061009e819060608252600560608301526468656c6c6f60d81b608083015263deadbeef604060a0840193600160208201520152565b0390a2005b5f80fd5b346100a3575f3660031901126100a3577f4e4cd44610926680098f1b54e2bdd1fb952659144c471173bbb9cf966af3a988818061009e602a949060608252600560608301526468656c6c6f60d81b608083015263deadbeef604060a084019360016020820152015256fea26469706673582212202e640cd14a7310d4165f902d2721ef5b4640a08f5ae38e9ae5c315a9f9f4435864736f6c63430008190033")]
        #[allow(dead_code)]
        contract MyContract {
            #[derive(Debug, PartialEq, Eq)]
            event MyEvent(uint64 indexed, string, bool, bytes32);

            #[derive(Debug, PartialEq, Eq)]
            event WrongEvent(uint64 indexed, string, bool, bytes32);

            function doEmit() external {
                emit MyEvent(42, "hello", true, bytes32(uint256(0xdeadbeef)));
            }

            function doEmitWrongEvent() external {
                emit WrongEvent(42, "hello", true, bytes32(uint256(0xdeadbeef)));
            }
        }
    }

    #[tokio::test]
    async fn event_filters() {
        let _ = tracing_subscriber::fmt::try_init();

        let anvil = alloy_node_bindings::Anvil::new().spawn();

        let pk: PrivateKeySigner =
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".parse().unwrap();
        let wallet = EthereumWallet::from(pk);
        let provider = alloy_provider::ProviderBuilder::new()
            .wallet(wallet.clone())
            .connect_http(anvil.endpoint_url());

        // let from = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
        let contract = MyContract::deploy(&provider).await.unwrap();

        let event: Event<_, MyContract::MyEvent, _> = Event::new(&provider, Filter::new());
        let all = event.query().await.unwrap();
        assert_eq!(all.len(), 0);

        // Same as above, but generated by `sol!`.
        let event = contract.MyEvent_filter();

        let poller = event.watch().await.unwrap();

        let _receipt =
            contract.doEmit().send().await.unwrap().get_receipt().await.expect("no receipt");

        let expected_event = MyContract::MyEvent {
            _0: 42,
            _1: "hello".to_string(),
            _2: true,
            _3: U256::from(0xdeadbeefu64).into(),
        };

        let mut stream = poller.into_stream();
        let (stream_event, stream_log) = stream.next().await.unwrap().unwrap();
        assert_eq!(MyContract::MyEvent::SIGNATURE_HASH.0, stream_log.topics().first().unwrap().0); // add check that the received event signature is the same as the one we expect
        assert_eq!(stream_event, expected_event);
        assert_eq!(stream_log.inner.address, *contract.address());
        assert_eq!(stream_log.block_number, Some(2));

        // This is not going to return `None`
        // assert!(stream.next().await.is_none());

        let all = event.query().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].0, expected_event);
        assert_eq!(all[0].1, stream_log);

        // send the wrong event and make sure it is NOT picked up by the event filter
        let _wrong_receipt = contract
            .doEmitWrongEvent()
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .expect("no receipt");

        // we sent the wrong event
        // so no events should be returned when querying event.query() (MyEvent)
        let all = event.query().await.unwrap();
        assert_eq!(all.len(), 0);

        #[cfg(feature = "pubsub")]
        {
            let provider = alloy_provider::ProviderBuilder::new()
                .wallet(wallet)
                .connect(&anvil.ws_endpoint())
                .await
                .unwrap();

            let contract = MyContract::new(*contract.address(), provider);
            let event = contract.MyEvent_filter();

            let sub = event.subscribe().await.unwrap();

            contract.doEmit().send().await.unwrap().get_receipt().await.expect("no receipt");

            let mut stream = sub.into_stream();

            let (stream_event, stream_log) = stream.next().await.unwrap().unwrap();
            assert_eq!(
                MyContract::MyEvent::SIGNATURE_HASH.0,
                stream_log.topics().first().unwrap().0
            );
            assert_eq!(stream_event, expected_event);
            assert_eq!(stream_log.address(), *contract.address());
            assert_eq!(stream_log.block_number, Some(4));

            // send the request to emit the wrong event
            contract
                .doEmitWrongEvent()
                .send()
                .await
                .unwrap()
                .get_receipt()
                .await
                .expect("no receipt");

            // we sent the wrong event
            // so no events should be returned when querying event.query() (MyEvent)
            let all = event.query().await.unwrap();
            assert_eq!(all.len(), 0);
        }
    }

    /// Same test as above, but using builder methods.
    #[tokio::test]
    async fn event_builder_filters() {
        let _ = tracing_subscriber::fmt::try_init();

        let anvil = alloy_node_bindings::Anvil::new().spawn();
        let pk: PrivateKeySigner =
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".parse().unwrap();
        let wallet = EthereumWallet::from(pk);
        let provider = alloy_provider::ProviderBuilder::new()
            .wallet(wallet.clone())
            .connect_http(anvil.endpoint_url());

        let contract = MyContract::deploy(&provider).await.unwrap();

        let event: Event<_, MyContract::MyEvent, _> = Event::new(&provider, Filter::new())
            .address(*contract.address())
            .event_signature(MyContract::MyEvent::SIGNATURE_HASH);
        let all = event.query().await.unwrap();
        assert_eq!(all.len(), 0);

        let poller = event.watch().await.unwrap();

        let _receipt =
            contract.doEmit().send().await.unwrap().get_receipt().await.expect("no receipt");

        let expected_event = MyContract::MyEvent {
            _0: 42,
            _1: "hello".to_string(),
            _2: true,
            _3: U256::from(0xdeadbeefu64).into(),
        };

        let mut stream = poller.into_stream();
        let (stream_event, stream_log) = stream.next().await.unwrap().unwrap();
        assert_eq!(MyContract::MyEvent::SIGNATURE_HASH.0, stream_log.topics().first().unwrap().0); // add check that the received event signature is the same as the one we expect
        assert_eq!(stream_event, expected_event);
        assert_eq!(stream_log.inner.address, *contract.address());
        assert_eq!(stream_log.block_number, Some(2));

        // This is not going to return `None`
        // assert!(stream.next().await.is_none());

        let all = event.query().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].0, expected_event);
        assert_eq!(all[0].1, stream_log);

        // send the wrong event and make sure it is NOT picked up by the event filter
        let _wrong_receipt = contract
            .doEmitWrongEvent()
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .expect("no receipt");

        // we sent the wrong event
        // so no events should be returned when querying event.query() (MyEvent)
        let all = event.query().await.unwrap();
        assert_eq!(all.len(), 0);

        #[cfg(feature = "pubsub")]
        {
            let provider = alloy_provider::ProviderBuilder::new()
                .wallet(wallet)
                .connect(&anvil.ws_endpoint())
                .await
                .unwrap();

            let contract = MyContract::new(*contract.address(), &provider);
            let event: Event<_, MyContract::MyEvent, _> = Event::new(&provider, Filter::new())
                .address(*contract.address())
                .event_signature(MyContract::MyEvent::SIGNATURE_HASH);

            let sub = event.subscribe().await.unwrap();

            contract.doEmit().send().await.unwrap().get_receipt().await.expect("no receipt");

            let mut stream = sub.into_stream();

            let (stream_event, stream_log) = stream.next().await.unwrap().unwrap();
            assert_eq!(
                MyContract::MyEvent::SIGNATURE_HASH.0,
                stream_log.topics().first().unwrap().0
            );
            assert_eq!(stream_event, expected_event);
            assert_eq!(stream_log.address(), *contract.address());
            assert_eq!(stream_log.block_number, Some(4));

            // send the request to emit the wrong event
            contract
                .doEmitWrongEvent()
                .send()
                .await
                .unwrap()
                .get_receipt()
                .await
                .expect("no receipt");

            // we sent the wrong event
            // so no events should be returned when querying event.query() (MyEvent)
            let all = event.query().await.unwrap();
            assert_eq!(all.len(), 0);
        }
    }

    #[tokio::test]
    async fn query_chunked_hits_chunking_path() {
        use alloy_provider::ext::AnvilApi;

        let _ = tracing_subscriber::fmt::try_init();

        let anvil = alloy_node_bindings::Anvil::new().spawn();

        let pk: PrivateKeySigner =
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".parse().unwrap();
        let wallet = EthereumWallet::from(pk);
        let provider = alloy_provider::ProviderBuilder::new()
            .wallet(wallet)
            .connect_http(anvil.endpoint_url());

        let contract = MyContract::deploy(&provider).await.unwrap();

        // Emit events at the start, middle, and end of a 10k block range
        // to stress the merge across chunks
        contract.doEmit().send().await.unwrap().get_receipt().await.expect("no receipt");
        provider.anvil_mine(Some(4998), None).await.unwrap();
        contract.doEmit().send().await.unwrap().get_receipt().await.expect("no receipt");
        provider.anvil_mine(Some(4998), None).await.unwrap();
        contract.doEmit().send().await.unwrap().get_receipt().await.expect("no receipt");
        provider.anvil_mine(Some(1), None).await.unwrap();

        // chunk_size=7 mirrors the Foundry PR (23879634–23889634), forcing ~1429 chunks
        let event = contract.MyEvent_filter().from_block(0u64).to_block(10_000u64);
        let chunked = event.get_logs_chunked_concurrent(7).await.unwrap();
        let full = event.query_raw().await.unwrap();

        assert_eq!(chunked.len(), 3);
        assert_eq!(chunked.len(), full.len());
        for (c, f) in chunked.iter().zip(full.iter()) {
            assert_eq!(c, f);
        }
    }
}
