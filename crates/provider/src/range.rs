use alloy_eips::BlockId;
use futures::{stream, Stream, StreamExt, TryStreamExt};
use std::future::Future;

/// Default concurrency for range requests.
const DEFAULT_CONCURRENCY: usize = 10;

/// A concurrent range-fetching utility that executes an async closure for each item in an
/// iterator, with configurable concurrency.
///
/// This is useful for fetching data (headers, receipts, etc.) across a range of block numbers
/// concurrently, without manually wiring up `futures::stream::iter` + `.map()` + `.buffered()`.
///
/// # Examples
///
/// Collect all headers for a range of blocks:
///
/// ```ignore
/// use alloy_provider::RangeRequest;
///
/// let headers: Vec<Header> = RangeRequest::new(100..=200, |block_id| {
///     let p = provider.clone();
///     async move { p.get_header_by_number(block_id).await }
/// })
/// .concurrency(16)
/// .await?;
/// ```
///
/// Stream results one by one:
///
/// ```ignore
/// use alloy_provider::RangeRequest;
/// use futures::TryStreamExt;
///
/// let mut stream = RangeRequest::new(100..=200, |block_id| {
///     let p = provider.clone();
///     async move { p.get_header_by_number(block_id).await }
/// })
/// .concurrency(16)
/// .into_stream();
///
/// while let Some(header) = stream.try_next().await? {
///     // process in order...
/// }
/// ```
#[must_use = "RangeRequest does nothing unless you `.await` it or call `.into_stream()`"]
pub struct RangeRequest<I, F> {
    iter: I,
    f: F,
    concurrency: usize,
}

impl<I, F> std::fmt::Debug for RangeRequest<I, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RangeRequest").field("concurrency", &self.concurrency).finish()
    }
}

impl<I, F, Fut, T, E> RangeRequest<I, F>
where
    I: IntoIterator,
    I::Item: Into<BlockId>,
    F: FnMut(BlockId) -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    /// Creates a new `RangeRequest` from an iterator and an async closure.
    ///
    /// The iterator items are converted to [`BlockId`] via [`Into<BlockId>`]. This works with
    /// `u64` ranges (e.g. `100..=200`), `Vec<BlockId>`, hash iterators, etc.
    pub const fn new(iter: I, f: F) -> Self {
        Self { iter, f, concurrency: DEFAULT_CONCURRENCY }
    }

    /// Sets the maximum number of in-flight futures (default: 10).
    pub const fn concurrency(mut self, n: usize) -> Self {
        self.concurrency = n;
        self
    }

    /// Returns an ordered stream of results.
    ///
    /// Futures are polled concurrently up to the configured concurrency limit, but results are
    /// yielded in the order of the original iterator.
    pub fn into_stream(self) -> impl Stream<Item = Result<T, E>> {
        let Self { iter, mut f, concurrency } = self;
        stream::iter(iter).map(move |item| f(item.into())).buffered(concurrency)
    }

    /// Returns an unordered stream of results.
    ///
    /// Like [`into_stream`](Self::into_stream), but results are yielded as soon as they complete,
    /// regardless of the original order.
    pub fn into_unordered_stream(self) -> impl Stream<Item = Result<T, E>> {
        let Self { iter, mut f, concurrency } = self;
        stream::iter(iter).map(move |item| f(item.into())).buffer_unordered(concurrency)
    }
}

impl<I, F, Fut, T, E> std::future::IntoFuture for RangeRequest<I, F>
where
    I: IntoIterator + Send + 'static,
    I::IntoIter: Send,
    I::Item: Into<BlockId>,
    F: FnMut(BlockId) -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, E>> + Send,
    T: Send + 'static,
    E: Send + 'static,
{
    type Output = Result<Vec<T>, E>;
    type IntoFuture = futures_utils_wasm::BoxFuture<'static, Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.into_stream().try_collect().await })
    }
}

#[cfg(all(test, feature = "anvil-node"))]
mod tests {
    use super::*;
    use crate::{ext::AnvilApi, Provider, ProviderBuilder};
    use alloy_node_bindings::Anvil;

    #[tokio::test]
    async fn range_request_collect() {
        let anvil = Anvil::new().spawn();
        let provider = ProviderBuilder::new().connect(&anvil.endpoint()).await.unwrap();

        provider.anvil_mine(Some(5), None).await.unwrap();

        let blocks = RangeRequest::new(0..=5u64, move |block_id| {
            let p = provider.clone();
            async move { p.get_block(block_id).await.map(|b| b.unwrap()) }
        })
        .concurrency(4)
        .await
        .unwrap();

        assert_eq!(blocks.len(), 6);
        for (i, block) in blocks.iter().enumerate() {
            assert_eq!(block.header.number, i as u64);
        }
    }
}
