//! A helper stream

use futures::{Stream, StreamExt};

/// A helper type to wrap [`Unpin`] streams.
pub struct PollerStream<Resp> {
    inner: Box<dyn Stream<Item = Resp> + Send + Unpin + 'static>,
}

impl<Resp> core::fmt::Debug for PollerStream<Resp> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PollerStream").finish()
    }
}

impl<Resp> PollerStream<Resp> {
    /// Instantiates a new [`PollerStream`].
    pub fn new(inner: impl Stream<Item = Resp> + Send + Unpin + 'static) -> Self {
        Self { inner: Box::new(inner) }
    }
}

impl<Resp> Stream for PollerStream<Resp> {
    type Item = Resp;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next_unpin(cx)
    }
}
