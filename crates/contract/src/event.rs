use alloy_providers::{new::FilterPoller, Network, Provider};
use alloy_rpc_types::{Filter, Log};
use alloy_sol_types::SolEvent;
use alloy_transport::{Transport, TransportResult};
use futures_util::StreamExt;
use std::{fmt, marker::PhantomData};

use crate::Error;

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
    pub async fn watch(&self) -> TransportResult<EventPoller<(), E>> {
        // let poller = self.provider.watch_logs(&self.filter).await?;
        // Ok(EventPoller::new(poller))
        todo!()
    }

    /// Watches for events that match the filter.
    ///
    /// Returns a stream of decoded events and raw logs.
    pub async fn subscribe(&self) -> TransportResult<()> {
        // let poller = self.provider.watch_logs(&self.filter).await?;
        // Ok(EventPoller::new(poller))
        todo!()
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
    pub poller: FilterPoller<T, Log>,
    _phantom: PhantomData<E>,
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
    pub fn new(poller: FilterPoller<T, Log>) -> Self {
        Self { poller, _phantom: PhantomData }
    }

    /// Converts the event poller into a stream that yields the decoded event and the raw log.
    pub fn into_stream(self) -> impl futures::Stream<Item = alloy_sol_types::Result<(E, Log)>> {
        self.poller
            .spawn()
            .into_stream()
            .flat_map(futures_util::stream::iter)
            .map(|log| decode_log(&log).map(|e| (e, log)))
    }
}

fn decode_log<E: SolEvent>(log: &Log) -> alloy_sol_types::Result<E> {
    E::decode_raw_log(log.topics.iter().copied(), &log.data, false)
}
