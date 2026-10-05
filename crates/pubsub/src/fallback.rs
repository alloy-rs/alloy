use crate::{handle::ConnectionHandle, PubSubConnect};
use alloy_transport::{impl_future, TransportResult};
use std::sync::atomic::{AtomicUsize, Ordering};

/// A [`PubSubConnect`] that round-robins across multiple connectors on
/// reconnect, always using the first connector for the initial connection.
#[derive(Debug)]
pub struct FallbackPubSubConnect<T> {
    connectors: Vec<T>,
    current: AtomicUsize,
}

impl<T: PubSubConnect> FallbackPubSubConnect<T> {
    /// Create a new fallback connector from a list of connectors.
    ///
    /// # Panics
    ///
    /// Panics if `connectors` is empty.
    pub fn new(connectors: Vec<T>) -> Self {
        assert!(!connectors.is_empty(), "FallbackPubSubConnect requires at least one connector");
        Self { connectors, current: AtomicUsize::new(0) }
    }

    /// Returns the number of connectors.
    pub const fn len(&self) -> usize {
        self.connectors.len()
    }

    /// Returns `true` if there are no connectors.
    pub const fn is_empty(&self) -> bool {
        self.connectors.is_empty()
    }
}

impl<T: PubSubConnect> PubSubConnect for FallbackPubSubConnect<T> {
    fn is_local(&self) -> bool {
        self.connectors[0].is_local()
    }

    fn connect(&self) -> impl_future!(<Output = TransportResult<ConnectionHandle>>) {
        self.connectors[0].connect()
    }

    fn try_reconnect(&self) -> impl_future!(<Output = TransportResult<ConnectionHandle>>) {
        let idx = self.current.fetch_add(1, Ordering::Relaxed) % self.connectors.len();
        self.connectors[idx].connect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{atomic::Ordering, Arc};

    #[derive(Clone, Debug)]
    struct MockConnector {
        connect_count: Arc<AtomicUsize>,
        is_local: bool,
    }

    impl MockConnector {
        fn new(is_local: bool) -> Self {
            Self { connect_count: Arc::new(AtomicUsize::new(0)), is_local }
        }

        fn connect_count(&self) -> usize {
            self.connect_count.load(Ordering::SeqCst)
        }
    }

    impl PubSubConnect for MockConnector {
        fn is_local(&self) -> bool {
            self.is_local
        }

        fn connect(&self) -> impl_future!(<Output = TransportResult<ConnectionHandle>>) {
            self.connect_count.fetch_add(1, Ordering::SeqCst);
            async move {
                let (handle, _interface) = ConnectionHandle::new();
                Ok(handle)
            }
        }
    }

    #[tokio::test]
    async fn test_connect_uses_first() {
        let c0 = MockConnector::new(false);
        let c1 = MockConnector::new(false);
        let c2 = MockConnector::new(false);

        let fallback = FallbackPubSubConnect::new(vec![c0.clone(), c1.clone(), c2.clone()]);

        let _ = fallback.connect().await.unwrap();
        let _ = fallback.connect().await.unwrap();
        let _ = fallback.connect().await.unwrap();

        assert_eq!(c0.connect_count(), 3);
        assert_eq!(c1.connect_count(), 0);
        assert_eq!(c2.connect_count(), 0);
    }

    #[tokio::test]
    async fn test_try_reconnect_cycles() {
        let c0 = MockConnector::new(false);
        let c1 = MockConnector::new(false);
        let c2 = MockConnector::new(false);

        let fallback = FallbackPubSubConnect::new(vec![c0.clone(), c1.clone(), c2.clone()]);

        let _ = fallback.try_reconnect().await.unwrap();
        let _ = fallback.try_reconnect().await.unwrap();
        let _ = fallback.try_reconnect().await.unwrap();
        let _ = fallback.try_reconnect().await.unwrap();

        assert_eq!(c0.connect_count(), 2);
        assert_eq!(c1.connect_count(), 1);
        assert_eq!(c2.connect_count(), 1);
    }

    #[tokio::test]
    async fn test_single_connector() {
        let c0 = MockConnector::new(false);

        let fallback = FallbackPubSubConnect::new(vec![c0.clone()]);

        let _ = fallback.connect().await.unwrap();
        let _ = fallback.try_reconnect().await.unwrap();
        let _ = fallback.try_reconnect().await.unwrap();

        assert_eq!(c0.connect_count(), 3);
    }
}
