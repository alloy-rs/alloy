use crate::Error;
use alloy_primitives::Address;
use alloy_provider::{FilterPollerBuilder, Network, Provider};
use alloy_rpc_types::{Filter, Log};
use alloy_sol_types::SolEvent;
use alloy_transport::{Transport, TransportResult};
use futures::Stream;
use futures_util::StreamExt;
use std::{fmt, marker::PhantomData};

/// Helper for managing the event filter before querying or streaming its logs
#[must_use = "event filters do nothing unless you `query`, `watch`, or `stream` them"]
pub struct Event<N, T, P, E> {
    /// The provider to use for querying or streaming logs.
    pub provider: P,
    /// The filter to use for querying or streaming logs.
    pub filter: Filter,
    _phantom: PhantomData<(T, N, E)>,
}

impl<N, T, P: fmt::Debug, E> fmt::Debug for Event<N, T, P, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Event")
            .field("provider", &self.provider)
            .field("filter", &self.filter)
            .field("event_type", &format_args!("{}", std::any::type_name::<E>()))
            .finish()
    }
}

#[doc(hidden)]
impl<'a, N: Network, T: Transport + Clone, P: Provider<N, T>, E: SolEvent> Event<N, T, &'a P, E> {
    // `sol!` macro constructor, see `#[sol(rpc)]`. Not public API.
    // NOTE: please avoid changing this function due to its use in the `sol!` macro.
    pub fn new_sol(provider: &'a P, address: &Address) -> Self {
        Self::new(provider, Filter::new().address(*address))
    }
}

impl<N: Network, T: Transport + Clone, P: Provider<N, T>, E: SolEvent> Event<N, T, P, E> {
    /// Creates a new event with the provided provider and filter.
    #[allow(clippy::missing_const_for_fn)]
    pub fn new(provider: P, filter: Filter) -> Self {
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
    pub async fn watch(&self) -> TransportResult<EventPoller<T, E>> {
        let poller = self.provider.watch_logs(&self.filter).await?;
        Ok(EventPoller::new(poller))
    }
}

impl<N, T, P: Clone, E> Event<N, T, &P, E> {
    /// Clones the provider and returns a new event with the cloned provider.
    pub fn with_cloned_provider(self) -> Event<N, T, P, E> {
        Event { provider: self.provider.clone(), filter: self.filter, _phantom: PhantomData }
    }
}

/// An event poller.
///
/// Polling configuration is available through the [`poller`](Self::poller) field.
pub struct EventPoller<T, E> {
    /// The inner poller.
    pub poller: FilterPollerBuilder<T, Log>,
    _phantom: PhantomData<E>,
}

impl<T, E> std::ops::Deref for EventPoller<T, E> {
    type Target = FilterPollerBuilder<T, Log>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.poller
    }
}

impl<T, E> std::ops::DerefMut for EventPoller<T, E> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.poller
    }
}

impl<T: fmt::Debug, E> fmt::Debug for EventPoller<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventPoller")
            .field("poller", &self.poller)
            .field("event_type", &format_args!("{}", std::any::type_name::<E>()))
            .finish()
    }
}

impl<T: Transport + Clone, E: SolEvent> EventPoller<T, E> {
    /// Creates a new event poller with the provided filter poller.
    #[allow(clippy::missing_const_for_fn)]
    #[inline]
    pub fn new(poller: FilterPollerBuilder<T, Log>) -> Self {
        Self { poller, _phantom: PhantomData }
    }

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

fn decode_log<E: SolEvent>(log: &Log) -> alloy_sol_types::Result<E> {
    E::decode_raw_log(log.topics.iter().copied(), &log.data, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::U256;
    use alloy_sol_types::sol;
    use test_utils::{init_tracing, spawn_anvil};

    sol! {
        // solc v0.8.24; solc a.sol --via-ir --optimize --bin
        #[sol(rpc, bytecode = "608080604052346100155760be908161001a8239f35b5f80fdfe60808060405260043610156011575f80fd5b5f3560e01c63299d8665146023575f80fd5b346084575f3660031901126084577f4e4cd44610926680098f1b54e2bdd1fb952659144c471173bbb9cf966af3a98860a0826060602a9452600560608201526468656c6c6f60d81b60808201526001602082015263deadbeef6040820152a2005b5f80fdfea2646970667358221220664e55b832143354f058e35b8948668184da14fcc5bf3300afb39dc6188c9add64736f6c63430008180033")]
        contract MyContract {
            #[derive(Debug, PartialEq)]
            event MyEvent(uint64 indexed, string, bool, bytes32);

            function doEmit() external {
                emit MyEvent(42, "hello", true, bytes32(uint256(0xdeadbeef)));
            }
        }
    }

    #[tokio::test]
    async fn event_filters() {
        init_tracing();
        let (provider, _anvil) = spawn_anvil();

        let contract = MyContract::deploy(&provider).await.unwrap();

        let event: Event<_, _, _, MyContract::MyEvent> = Event::new(&provider, Filter::new());
        let all = event.query().await.unwrap();
        assert_eq!(all.len(), 0);

        // Same as above, but generated by `sol!`.
        let event = contract.MyContractInstance_filter();

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
        assert_eq!(stream_event, expected_event);
        assert_eq!(stream_log.address, *contract.address());
        assert_eq!(stream_log.block_number, Some(U256::from(2)));

        // This is not going to return `None`
        // assert!(stream.next().await.is_none());

        let all = event.query().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].0, expected_event);
        assert_eq!(all[0].1, stream_log);
    }
}
