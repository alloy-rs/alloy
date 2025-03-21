use futures::{Stream, StreamExt};
use std::pin::Pin;

#[cfg(target_arch = "wasm32")]
type InnerStream<Resp> = Pin<Box<dyn Stream<Item = Resp> + 'static>>;
#[cfg(not(target_arch = "wasm32"))]
type InnerStream<Resp> = Pin<Box<dyn Stream<Item = Resp> + Send + 'static>>;

/// A stream wrapper type.
pub struct PollerStream<Resp> {
    inner: InnerStream<Resp>,
}

impl<Resp> core::fmt::Debug for PollerStream<Resp> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PollerStream").finish()
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<Resp: 'static> PollerStream<Resp> {
    /// Instantiates a new [`PollerStream`].
    pub fn new(inner: impl Stream<Item = Resp> + Send + 'static) -> Self {
        Self { inner: Box::pin(inner) }
    }

    /// Applies [`StreamExt::flat_map`] to the inner stream and returns a new [`PollerStream`].
    pub fn flat_map<F, S>(self, f: F) -> PollerStream<S::Item>
    where
        F: FnMut(Resp) -> S + Send + 'static,
        S: Stream + Send + 'static,
        S::Item: 'static,
    {
        let f = self.inner.flat_map(f);
        PollerStream::new(f)
    }

    /// Applies [`StreamExt::map`] to the inner stream and returns a new [`PollerStream`].
    pub fn map<F, T>(self, f: F) -> PollerStream<T>
    where
        F: FnMut(Resp) -> T + Send + 'static,
        T: 'static,
    {
        let f = self.inner.map(f);
        PollerStream::new(f)
    }

    /// Applies [`StreamExt::filter_map`] to the inner stream and returns a new [`PollerStream`].
    pub fn filter_map<Fut, T, F>(self, f: F) -> PollerStream<T>
    where
        F: FnMut(Resp) -> Fut + Send + 'static,
        Fut: futures::Future<Output = Option<T>> + Send + 'static,
        T: 'static,
    {
        let f = self.inner.filter_map(f);
        PollerStream::new(f)
    }
}

#[cfg(target_arch = "wasm32")]
impl<Resp: 'static> PollerStream<Resp> {
    /// Instantiates a new [`PollerStream`].
    pub fn new(inner: impl Stream<Item = Resp> + 'static) -> Self {
        Self { inner: Box::pin(inner) }
    }

    /// Applies [`StreamExt::flat_map`] to the inner stream and returns a new [`PollerStream`].
    pub fn flat_map<F, S>(self, f: F) -> PollerStream<S::Item>
    where
        F: FnMut(Resp) -> S + 'static,
        S: Stream + 'static,
        S::Item: 'static,
    {
        let f = self.inner.flat_map(f);
        PollerStream::new(f)
    }

    /// Applies [`StreamExt::map`] to the inner stream and returns a new [`PollerStream`].
    pub fn map<F, T>(self, f: F) -> PollerStream<T>
    where
        F: FnMut(Resp) -> T + 'static,
        T: 'static,
    {
        let f = self.inner.map(f);
        PollerStream::new(f)
    }

    /// Applies [`StreamExt::filter_map`] to the inner stream and returns a new [`PollerStream`].
    pub fn filter_map<Fut, T, F>(self, f: F) -> PollerStream<T>
    where
        F: FnMut(Resp) -> Fut + 'static,
        Fut: futures::Future<Output = Option<T>> + 'static,
        T: 'static,
    {
        let f = self.inner.filter_map(f);
        PollerStream::new(f)
    }
}

impl<Resp> Stream for PollerStream<Resp> {
    type Item = Resp;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}
