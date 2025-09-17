use crate::ProviderCall;
use alloy_eips::BlockId;
use alloy_json_rpc::RpcRecv;
use alloy_network::Network;
use alloy_rpc_types_eth::{Filter, FilterBlockOption};
use alloy_transport::TransportResult;
use std::{future::Future, marker::PhantomData, sync::Arc, task::Poll};

mod params;
pub use params::EthLogsParams;

mod caller;
pub use caller::LogsCaller;

/// Options for enhanced log retrieval.
#[derive(Clone, Debug, Default)]
pub struct LogOptions {
    /// If provided, fetches logs in batches of this size when querying a block range.
    pub batch_size: Option<u64>,
    /// If provided, stops fetching when this many logs have been collected.
    pub max_count: Option<usize>,
}

impl LogOptions {
    /// Returns true if any options are set.
    pub const fn has_options(&self) -> bool {
        self.batch_size.is_some() || self.max_count.is_some()
    }
}

/// The [`EthLogsFut`] future is the future type for batch log retrieval.
#[derive(Debug)]
#[doc(hidden)] // Not public API.
#[expect(unnameable_types)]
#[pin_project::pin_project]
pub struct EthLogsFut<N, Resp, Output, Map>
where
    N: Network,
    Resp: RpcRecv,
    Output: 'static,
    Map: Fn(Resp) -> Output,
{
    inner: EthLogsFutInner<N, Resp, Output, Map>,
}

enum EthLogsFutInner<N, Resp, Output, Map>
where
    N: Network,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    Preparing { caller: Arc<dyn LogsCaller<N, Resp>>, params: EthLogsParams<N>, map: Map },
    Running { map: Map, fut: ProviderCall<EthLogsParams<N>, Resp> },
    Polling,
}

impl<N, Resp, Output, Map> core::fmt::Debug for EthLogsFutInner<N, Resp, Output, Map>
where
    N: Network,
    Resp: RpcRecv,
    Output: 'static,
    Map: Fn(Resp) -> Output,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Preparing { caller: _, params, map: _ } => {
                f.debug_struct("Preparing").field("params", params).finish()
            }
            Self::Running { .. } => f.debug_tuple("Running").finish(),
            Self::Polling => f.debug_tuple("Polling").finish(),
        }
    }
}

impl<N, Resp, Output, Map> EthLogsFut<N, Resp, Output, Map>
where
    N: Network,
    Resp: RpcRecv,
    Output: 'static,
    Map: Fn(Resp) -> Output,
{
    /// Returns `true` if the future is in the preparing state.
    const fn is_preparing(&self) -> bool {
        matches!(self.inner, EthLogsFutInner::Preparing { .. })
    }

    /// Returns `true` if the future is in the running state.
    const fn is_running(&self) -> bool {
        matches!(self.inner, EthLogsFutInner::Running { .. })
    }

    fn poll_preparing(&mut self, cx: &mut std::task::Context<'_>) -> Poll<TransportResult<Output>> {
        let EthLogsFutInner::Preparing { caller, params, map } =
            std::mem::replace(&mut self.inner, EthLogsFutInner::Polling)
        else {
            unreachable!("bad state")
        };

        let fut = caller.get_logs(params)?;

        self.inner = EthLogsFutInner::Running { map, fut };

        self.poll_running(cx)
    }

    fn poll_running(&mut self, cx: &mut std::task::Context<'_>) -> Poll<TransportResult<Output>> {
        let EthLogsFutInner::Running { ref map, ref mut fut } = self.inner else {
            unreachable!("bad state")
        };

        std::pin::Pin::new(fut).poll(cx).map(|res| res.map(map))
    }
}

impl<N, Resp, Output, Map> Future for EthLogsFut<N, Resp, Output, Map>
where
    N: Network,
    Resp: RpcRecv,
    Output: 'static,
    Map: Fn(Resp) -> Output,
{
    type Output = TransportResult<Output>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.get_mut();
        if this.is_preparing() {
            this.poll_preparing(cx)
        } else if this.is_running() {
            this.poll_running(cx)
        } else {
            panic!("unexpected state")
        }
    }
}

/// A builder for batch log retrieval. This type is returned by the
/// [`Provider::get_logs`] method.
///
/// [`Provider::get_logs`]: crate::Provider::get_logs
#[must_use = "EthLogs must be awaited to execute the log retrieval"]
#[derive(Clone)]
pub struct EthLogs<N, Resp, Output = Resp, Map = fn(Resp) -> Output>
where
    N: Network,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    caller: Arc<dyn LogsCaller<N, Resp>>,
    params: EthLogsParams<N>,
    map: Map,
    _pd: PhantomData<fn() -> (Resp, Output)>,
}

impl<N, Resp> core::fmt::Debug for EthLogs<N, Resp>
where
    N: Network,
    Resp: RpcRecv,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EthLogs").field("params", &self.params).finish()
    }
}

impl<N, Resp> EthLogs<N, Resp>
where
    N: Network,
    Resp: RpcRecv,
{
    /// Create a new [`EthLogs`].
    pub fn new(caller: impl LogsCaller<N, Resp> + 'static, filter: Filter) -> Self {
        Self {
            caller: Arc::new(caller),
            params: EthLogsParams::new(filter),
            map: std::convert::identity,
            _pd: PhantomData,
        }
    }
}

impl<N, Resp, Output, Map> EthLogs<N, Resp, Output, Map>
where
    N: Network,
    Resp: RpcRecv,
    Map: Fn(Resp) -> Output,
{
    /// Map the response to a different type.
    pub fn map_resp<NewOutput, NewMap>(self, map: NewMap) -> EthLogs<N, Resp, NewOutput, NewMap>
    where
        NewMap: Fn(Resp) -> NewOutput,
    {
        EthLogs { caller: self.caller, params: self.params, map, _pd: PhantomData }
    }

    /// Sets the batch size for log fetching.
    pub const fn with_batch_size(mut self, batch_size: u64) -> Self {
        self.params.options.batch_size = Some(batch_size);
        self
    }

    /// Sets the maximum count of logs to fetch.
    pub const fn with_max_count(mut self, max_count: usize) -> Self {
        self.params.options.max_count = Some(max_count);
        self
    }

    /// Sets the log options.
    pub const fn with_options(mut self, options: LogOptions) -> Self {
        self.params.options = options;
        self
    }

    /// Set the block to use for this log query.
    pub const fn block(mut self, block: BlockId) -> Self {
        match block {
            BlockId::Hash(hash) => {
                self.params.filter.block_option = FilterBlockOption::AtBlockHash(hash.block_hash);
            }
            BlockId::Number(number) => {
                self.params.filter.block_option =
                    FilterBlockOption::Range { from_block: Some(number), to_block: Some(number) };
            }
        }
        self
    }
}

impl<N, Resp, Output, Map> std::future::IntoFuture for EthLogs<N, Resp, Output, Map>
where
    N: Network,
    Resp: RpcRecv,
    Output: 'static,
    Map: Fn(Resp) -> Output,
{
    type Output = TransportResult<Output>;

    type IntoFuture = EthLogsFut<N, Resp, Output, Map>;

    fn into_future(self) -> Self::IntoFuture {
        EthLogsFut {
            inner: EthLogsFutInner::Preparing {
                caller: self.caller,
                params: self.params,
                map: self.map,
            },
        }
    }
}
