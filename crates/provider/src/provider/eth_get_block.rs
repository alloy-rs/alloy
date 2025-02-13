use alloy_eips::{BlockId, BlockNumberOrTag, RpcBlockHash};
use alloy_network::Network;
use alloy_network_primitives::{BlockResponse, BlockTransactionsKind};
use alloy_primitives::BlockHash;
use alloy_rpc_client::{RpcCall, WeakClient};
use alloy_transport::{TransportErrorKind, TransportResult};
use futures::FutureExt;
use std::{future::Future, marker::PhantomData, task::Poll};

type BlockResult<N> = TransportResult<Option<N>>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum BlockIdParam {
    Hash(BlockHash),
    Number(BlockNumberOrTag),
}

// Serialize implementation that will properly work with `eth_getBlockBy{Hash, Number}` calls
impl serde::Serialize for BlockIdParam {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Hash(hash) => hash.serialize(serializer),
            Self::Number(num) => num.serialize(serializer),
        }
    }
}

impl From<BlockId> for BlockIdParam {
    fn from(block: BlockId) -> Self {
        match block {
            BlockId::Hash(RpcBlockHash { block_hash, .. }) => Self::Hash(block_hash),
            BlockId::Number(number) => Self::Number(number),
        }
    }
}

/// The parameters for an `eth_getBlockBy{Hash, Number}` RPC request.
#[allow(unnameable_types)]
#[derive(Clone, Debug)]
struct EthGetBlockParams {
    block: BlockIdParam,
    kind: BlockTransactionsKind,
}

impl EthGetBlockParams {
    fn with_block(block: BlockId) -> Self {
        Self { block: block.into(), kind: BlockTransactionsKind::Hashes }
    }

    fn with_hash(block: BlockHash) -> Self {
        Self { block: BlockIdParam::Hash(block), kind: BlockTransactionsKind::Hashes }
    }

    fn with_number(block: BlockNumberOrTag) -> Self {
        Self { block: BlockIdParam::Number(block), kind: BlockTransactionsKind::Hashes }
    }

    fn set_full(&mut self) {
        self.kind = BlockTransactionsKind::Full;
    }

    fn set_kind(&mut self, kind: BlockTransactionsKind) {
        self.kind = kind;
    }

    /// Return the transaction kind
    fn kind(&self) -> BlockTransactionsKind {
        self.kind
    }
}

/// The [`EthGetBlockByFut`] future is the future type for an `eth_getBlockBy` RPC request.
#[allow(unnameable_types)]
#[doc(hidden)] // Not public API.
#[derive(Debug)]
pub struct EthGetBlockFut<N>
where
    N: Network,
{
    inner: EthGetBlockFutInner<N>,
}

enum EthGetBlockFutInner<N>
where
    N: Network,
{
    Preparing {
        client: WeakClient,
        params: EthGetBlockParams,
    },
    Running {
        fut: RpcCall<(BlockIdParam, bool), Option<N::BlockResponse>>,
        params: EthGetBlockParams,
    },
    Polling,
}

impl<N> core::fmt::Debug for EthGetBlockFutInner<N>
where
    N: Network,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Preparing { client, params, .. } => f
                .debug_struct("Preparing")
                .field("client", &client)
                .field("params", &params)
                .finish(),
            Self::Running { fut, params, .. } => {
                f.debug_struct("Runinng").field("fut", &fut).field("params", &params).finish()
            }
            Self::Polling => f.debug_tuple("Polling").finish(),
        }
    }
}

impl<N> EthGetBlockFut<N>
where
    N: Network,
{
    /// Returns `true` if the future is in the preparing state.
    const fn is_preparing(&self) -> bool {
        matches!(self.inner, EthGetBlockFutInner::Preparing { .. })
    }

    /// Returns `true` if the future is in the running state.
    const fn is_running(&self) -> bool {
        matches!(self.inner, EthGetBlockFutInner::Running { .. })
    }

    fn poll_preparing(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<BlockResult<N::BlockResponse>> {
        let EthGetBlockFutInner::Preparing { client, params } =
            std::mem::replace(&mut self.inner, EthGetBlockFutInner::<N>::Polling)
        else {
            unreachable!("bad state")
        };

        let method = match params.block {
            BlockIdParam::Hash(_) => "eth_getBlockByHash",
            BlockIdParam::Number(_) => "eth_getBlockByNumber",
        };
        let full = match params.kind {
            BlockTransactionsKind::Full => true,
            BlockTransactionsKind::Hashes => false,
        };
        let client =
            client.upgrade().ok_or_else(|| TransportErrorKind::custom_str("RPC client dropped"))?;
        let fut = client.request::<_, Option<N::BlockResponse>>(method, (params.block, full));

        self.inner = EthGetBlockFutInner::<N>::Running { fut, params };

        self.poll_running(cx)
    }

    fn poll_running(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<BlockResult<N::BlockResponse>> {
        let EthGetBlockFutInner::Running { ref mut fut, ref params } = self.inner else {
            unreachable!("bad state")
        };

        fut.poll_unpin(cx).map(|block| {
            block.map(|block| {
                block.map(|mut block| {
                    if params.kind() == BlockTransactionsKind::Hashes {
                        // this ensures an empty response for `Hashes` has the expected form
                        // this is required because deserializing [] is ambiguous
                        block.transactions_mut().convert_to_hashes();
                    }
                    block
                })
            })
        })
    }
}

impl<N> Future for EthGetBlockFut<N>
where
    N: Network,
{
    type Output = BlockResult<N::BlockResponse>;

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

/// A builder for an `"eth_getBlockByHash"` request. This type is returned by the
/// [`Provider::call`] method.
///
/// [`Provider::call`]: crate::Provider::call
#[must_use = "EthGetBlockBy must be awaited to execute the request"]
//#[derive(Clone, Debug)]
pub struct EthGetBlock<N>
where
    N: Network,
{
    client: WeakClient,
    params: EthGetBlockParams,
    _pd: PhantomData<N>,
}

impl<N> EthGetBlock<N>
where
    N: Network,
{
    /// Create a new [`EthGetBlock`] with method set to `"eth_getBlockBy{Hash, Number}"`.
    pub fn by_block(client: WeakClient, block: BlockId) -> Self {
        Self { client, params: EthGetBlockParams::with_block(block), _pd: PhantomData }
    }

    /// Create a new [`EthGetBlock`] with method set to `"eth_getBlockByHash"`.
    pub fn by_hash(client: WeakClient, block: BlockHash) -> Self {
        Self { client, params: EthGetBlockParams::with_hash(block), _pd: PhantomData }
    }

    /// Create a new [`EthGetBlock`] with method set to `"eth_getBlockByNumber"`.
    pub fn by_number(client: WeakClient, block: BlockNumberOrTag) -> Self {
        Self { client, params: EthGetBlockParams::with_number(block), _pd: PhantomData }
    }

    /// Set the transaction kind
    pub fn with_kind(mut self, kind: BlockTransactionsKind) -> Self {
        self.params.set_kind(kind);
        self
    }

    /// Set the `full:bool` argument in RPC calls
    pub fn full(mut self) -> Self {
        self.params.set_full();
        self
    }
}

impl<N> std::future::IntoFuture for EthGetBlock<N>
where
    N: Network,
{
    type Output = BlockResult<N::BlockResponse>;

    type IntoFuture = EthGetBlockFut<N>;

    fn into_future(self) -> Self::IntoFuture {
        EthGetBlockFut {
            inner: EthGetBlockFutInner::Preparing { client: self.client, params: self.params },
        }
    }
}

impl<N> core::fmt::Debug for EthGetBlock<N>
where
    N: Network,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EthGetBlock")
            .field("client", &self.client)
            .field("params", &self.params)
            .finish()
    }
}
