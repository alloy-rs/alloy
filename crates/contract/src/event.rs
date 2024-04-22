use crate::Error;
use alloy_network::Ethereum;
use alloy_primitives::{Address, LogData};
use alloy_provider::{FilterPollerBuilder, Network, Provider};
use alloy_rpc_types::{Filter, Log};
use alloy_sol_types::SolEvent;
use alloy_transport::{Transport, TransportResult};
use futures::Stream;
use futures_util::StreamExt;
use std::{fmt, marker::PhantomData};

/// Helper for managing the event filter before querying or streaming its logs
#[must_use = "event filters do nothing unless you `query`, `watch`, or `stream` them"]
pub struct Event<T, P, E, N = Ethereum> {
    /// The provider to use for querying or streaming logs.
    pub provider: P,
    /// The filter to use for querying or streaming logs.
    pub filter: Filter,
    _phantom: PhantomData<(T, E, N)>,
}

impl<T, P: fmt::Debug, E, N> fmt::Debug for Event<T, P, E, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Event")
            .field("provider", &self.provider)
            .field("filter", &self.filter)
            .field("event_type", &format_args!("{}", std::any::type_name::<E>()))
            .finish()
    }
}

#[doc(hidden)]
impl<'a, T: Transport + Clone, P: Provider<T, N>, E: SolEvent, N: Network> Event<T, &'a P, E, N> {
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

impl<T: Transport + Clone, P: Provider<T, N>, E: SolEvent, N: Network> Event<T, P, E, N> {
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
}

impl<T, P: Clone, E, N> Event<T, &P, E, N> {
    /// Clones the provider and returns a new event with the cloned provider.
    pub fn with_cloned_provider(self) -> Event<T, P, E, N> {
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

impl<T, E> AsRef<FilterPollerBuilder<T, Log>> for EventPoller<T, E> {
    #[inline]
    fn as_ref(&self) -> &FilterPollerBuilder<T, Log> {
        &self.poller
    }
}

impl<T, E> AsMut<FilterPollerBuilder<T, Log>> for EventPoller<T, E> {
    #[inline]
    fn as_mut(&mut self) -> &mut FilterPollerBuilder<T, Log> {
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

impl<T, E> From<FilterPollerBuilder<T, Log>> for EventPoller<T, E> {
    fn from(poller: FilterPollerBuilder<T, Log>) -> Self {
        Self { poller, _phantom: PhantomData }
    }
}

impl<T: Transport + Clone, E: SolEvent> EventPoller<T, E> {
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
    let log_data: &LogData = log.as_ref();

    E::decode_raw_log(log_data.topics().iter().copied(), &log_data.data, false)
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
    use alloy_primitives::U256;
    use alloy_sol_types::sol;
    use test_utils::{init_tracing, spawn_anvil};

    sol! {
        // solc v0.8.24; solc a.sol --via-ir --optimize --bin
        #[sol(rpc, bytecode = "60808060405234601557610147908161001a8239f35b5f80fdfe6080806040526004361015610012575f80fd5b5f3560e01c908163299d8665146100a7575063ffdf4f1b14610032575f80fd5b346100a3575f3660031901126100a357602a7f6d10b8446ff0ac11bb95d154e7b10a73042fb9fc3bca0c92de5397b2fe78496c6040518061009e819060608252600560608301526468656c6c6f60d81b608083015263deadbeef604060a0840193600160208201520152565b0390a2005b5f80fd5b346100a3575f3660031901126100a3577f4e4cd44610926680098f1b54e2bdd1fb952659144c471173bbb9cf966af3a988818061009e602a949060608252600560608301526468656c6c6f60d81b608083015263deadbeef604060a084019360016020820152015256fea26469706673582212202e640cd14a7310d4165f902d2721ef5b4640a08f5ae38e9ae5c315a9f9f4435864736f6c63430008190033")]
        #[allow(dead_code)]
        contract MyContract {
            #[derive(Debug, PartialEq)]
            event MyEvent(uint64 indexed, string, bool, bytes32);

            #[derive(Debug, PartialEq)]
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
        init_tracing();

        #[cfg(feature = "ws")]
        let (provider, anvil) = spawn_anvil();

        #[cfg(not(feature = "ws"))]
        let (provider, _anvil) = spawn_anvil();

        let contract = MyContract::deploy(&provider).await.unwrap();

        let event: Event<_, _, MyContract::MyEvent, _> = Event::new(&provider, Filter::new());
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

        #[cfg(feature = "ws")]
        {
            let provider = alloy_provider::ProviderBuilder::default()
                .on_ws(anvil.ws_endpoint())
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
            assert_eq!(stream_log.address, *contract.address());
            assert_eq!(stream_log.block_number, Some(U256::from(3)));

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
}
