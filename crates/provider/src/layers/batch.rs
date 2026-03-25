use crate::{
    bindings::{ArbSys, IMulticall3},
    Caller, Provider, ProviderCall, ProviderLayer, RootProvider, ARB_SYS_ADDRESS,
    MULTICALL3_ADDRESS,
};
use alloy_eips::BlockId;
use alloy_network::{Ethereum, Network, TransactionBuilder};
use alloy_primitives::{Address, Bytes, U256};
use alloy_rpc_client::WeakClient;
use alloy_sol_types::{SolCall, SolType, SolValue};
use alloy_transport::{utils::Spawnable, TransportErrorKind, TransportResult};
use std::{fmt, future::IntoFuture, marker::PhantomData, sync::Arc, time::Duration};
use tokio::sync::{mpsc, oneshot};

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use wasmtimer::tokio::sleep;

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use tokio::time::sleep;

/// This is chosen somewhat arbitrarily. It should be short enough to not cause a noticeable
/// delay on individual requests, but long enough to allow for batching requests issued together in
/// a short period of time, such as when using `join!` macro or similar future combinators.
const DEFAULT_WAIT: Duration = Duration::from_millis(1);

/// Provider layer that aggregates contract calls (`eth_call`) over a time period into a single
/// [Multicall3] contract call.
///
/// Some methods, such as `eth_getBlockNumber`, are first converted into contract calls to the
/// [Multicall3] contract itself and then aggregated with other `eth_call`s.
///
/// Only calls that:
/// - target the latest block ID,
/// - have no state overrides,
/// - have a target address and calldata,
/// - have no other properties (nonce, gas, etc.)
///
/// can be sent with a multicall. This of course requires that the [Multicall3] contract is
/// deployed on the network, by default at [`MULTICALL3_ADDRESS`].
///
/// This layer is useful for reducing the number of network requests made.
/// However, this only works when requests are made in parallel, for example when using the
/// [`tokio::join!`] macro or in multiple threads/tasks, as otherwise the requests will be sent one
/// by one as normal, but with an added delay.
///
/// # Examples
///
/// ```no_run
/// use alloy_provider::{layers::CallBatchLayer, Provider, ProviderBuilder};
/// use std::time::Duration;
///
/// # async fn f(url: &str) -> Result<(), Box<dyn std::error::Error>> {
/// // Build a provider with the default call batching configuration.
/// let provider = ProviderBuilder::new().with_call_batching().connect(url).await?;
///
/// // Build a provider with a custom call batching configuration.
/// let provider = ProviderBuilder::new()
///     .layer(CallBatchLayer::new().wait(Duration::from_millis(10)))
///     .connect(url)
///     .await?;
///
/// // Both of these requests will be batched together and only 1 network request will be made.
/// let (block_number_result, chain_id_result) =
///     tokio::join!(provider.get_block_number(), provider.get_chain_id());
/// let block_number = block_number_result?;
/// let chain_id = chain_id_result?;
/// println!("block number: {block_number}, chain id: {chain_id}");
/// # Ok(())
/// # }
/// ```
///
/// [Multicall3]: https://github.com/mds1/multicall3
#[derive(Debug)]
pub struct CallBatchLayer {
    m3a: Address,
    wait: Duration,
    arbsys: bool,
}

impl Default for CallBatchLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl CallBatchLayer {
    /// Create a new `CallBatchLayer` with a default wait of 1ms.
    pub const fn new() -> Self {
        Self { m3a: MULTICALL3_ADDRESS, wait: DEFAULT_WAIT, arbsys: false }
    }

    /// Set the amount of time to wait before sending the batch.
    ///
    /// This is the amount of time to wait after the first request is received before sending all
    /// the requests received in that time period.
    ///
    /// This means that every request has a maximum delay of `wait` before being sent.
    ///
    /// The default is 1ms.
    pub const fn wait(mut self, wait: Duration) -> Self {
        self.wait = wait;
        self
    }

    /// Set the multicall3 address.
    ///
    /// The default is [`MULTICALL3_ADDRESS`].
    pub const fn multicall3_address(mut self, m3a: Address) -> Self {
        self.m3a = m3a;
        self
    }

    /// Use the Arbitrum `ArbSys` precompile for block number queries.
    ///
    /// On Arbitrum, `block.number` returns the parent chain’s block number (L1).
    /// Without this setting, batched `eth_blockNumber` calls through Multicall3
    /// will therefore return the wrong value. Enabling this queries the L2 block
    /// number via `ArbSys` instead.
    ///
    /// The default is `false`.
    /// This should only be enabled when interacting with Arbitrum rollups.
    pub const fn arbitrum_compat(mut self) -> Self {
        self.arbsys = true;
        self
    }
}

impl<P, N> ProviderLayer<P, N> for CallBatchLayer
where
    P: Provider<N> + 'static,
    N: Network,
{
    type Provider = CallBatchProvider<P, N>;

    fn layer(&self, inner: P) -> Self::Provider {
        CallBatchProvider::new(inner, self)
    }
}

type CallBatchMsgTx = TransportResult<IMulticall3::Result>;

struct CallBatchMsg<N: Network> {
    kind: CallBatchMsgKind<N>,
    tx: oneshot::Sender<CallBatchMsgTx>,
}

impl<N: Network> Clone for CallBatchMsgKind<N>
where
    N::TransactionRequest: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::Call(tx) => Self::Call(tx.clone()),
            Self::BlockNumber => Self::BlockNumber,
            Self::ChainId => Self::ChainId,
            Self::Balance(addr) => Self::Balance(*addr),
        }
    }
}

impl<N: Network> fmt::Debug for CallBatchMsg<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("BatchProviderMessage(")?;
        self.kind.fmt(f)?;
        f.write_str(")")
    }
}

#[derive(Debug)]
enum CallBatchMsgKind<N: Network = Ethereum> {
    Call(N::TransactionRequest),
    BlockNumber,
    ChainId,
    Balance(Address),
}

impl<N: Network> CallBatchMsg<N> {
    fn new(kind: CallBatchMsgKind<N>) -> (Self, oneshot::Receiver<CallBatchMsgTx>) {
        let (tx, rx) = oneshot::channel();
        (Self { kind, tx }, rx)
    }
}

impl<N: Network> CallBatchMsgKind<N> {
    fn to_call3(&self, m3a: Address, arbsys: bool) -> IMulticall3::Call3 {
        let m3a_call = |data: Vec<u8>| IMulticall3::Call3 {
            target: m3a,
            allowFailure: true,
            callData: data.into(),
        };
        match self {
            Self::Call(tx) => IMulticall3::Call3 {
                target: tx.to().unwrap_or_default(),
                allowFailure: true,
                callData: tx.input().cloned().unwrap_or_default(),
            },
            Self::BlockNumber => {
                if arbsys {
                    return IMulticall3::Call3 {
                        target: ARB_SYS_ADDRESS,
                        allowFailure: false,
                        callData: ArbSys::arbBlockNumberCall {}.abi_encode().into(),
                    };
                }
                m3a_call(IMulticall3::getBlockNumberCall {}.abi_encode())
            }
            Self::ChainId => m3a_call(IMulticall3::getChainIdCall {}.abi_encode()),
            &Self::Balance(addr) => m3a_call(IMulticall3::getEthBalanceCall { addr }.abi_encode()),
        }
    }
}

/// A provider that batches multiple requests into a single request.
///
/// See [`CallBatchLayer`] for more information.
pub struct CallBatchProvider<P, N: Network = Ethereum> {
    provider: Arc<P>,
    inner: CallBatchProviderInner<N>,
    _pd: PhantomData<N>,
}

impl<P, N: Network> Clone for CallBatchProvider<P, N> {
    fn clone(&self) -> Self {
        Self { provider: self.provider.clone(), inner: self.inner.clone(), _pd: PhantomData }
    }
}

impl<P: fmt::Debug, N: Network> fmt::Debug for CallBatchProvider<P, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("BatchProvider(")?;
        self.provider.fmt(f)?;
        f.write_str(")")
    }
}

impl<P: Provider<N> + 'static, N: Network> CallBatchProvider<P, N> {
    fn new(inner: P, layer: &CallBatchLayer) -> Self {
        let inner = Arc::new(inner);
        let tx = CallBatchBackend::spawn(inner.clone(), layer);
        Self { provider: inner, inner: CallBatchProviderInner { tx }, _pd: PhantomData }
    }
}

#[derive(Clone)]
struct CallBatchProviderInner<N: Network> {
    tx: mpsc::UnboundedSender<CallBatchMsg<N>>,
}

impl<N: Network> CallBatchProviderInner<N> {
    /// We only want to perform a scheduled multicall if:
    /// - The request has no block ID or state overrides,
    /// - The request has a target address,
    /// - The request has no other properties (`nonce`, `gas`, etc cannot be sent with a multicall).
    ///
    /// Ref: <https://github.com/wevm/viem/blob/ba8319f71503af8033fd3c77cfb64c7eb235c6a9/src/actions/public/call.ts#L295>
    fn should_batch_call(&self, params: &crate::EthCallParams<N>) -> bool {
        // TODO: block ID is not yet implemented
        if params.block().is_some_and(|block| block != BlockId::latest()) {
            return false;
        }
        if params.overrides.as_ref().is_some_and(|overrides| !overrides.is_empty()) {
            return false;
        }
        let tx = params.data();
        if tx.to().is_none() {
            return false;
        }
        if let Ok(serde_json::Value::Object(obj)) = serde_json::to_value(tx) {
            if obj.keys().any(|k| !matches!(k.as_str(), "to" | "data" | "input")) {
                return false;
            }
        }
        true
    }

    async fn schedule(self, msg: CallBatchMsgKind<N>) -> TransportResult<Bytes> {
        let (msg, rx) = CallBatchMsg::new(msg);
        self.tx.send(msg).map_err(|_| TransportErrorKind::backend_gone())?;

        let IMulticall3::Result { success, returnData } =
            rx.await.map_err(|_| TransportErrorKind::backend_gone())??;

        if !success {
            let revert_data = if returnData.is_empty() {
                "".to_string()
            } else {
                format!(" with data: {returnData}")
            };
            Err(TransportErrorKind::custom_str(&format!(
                "multicall batched call reverted{revert_data}"
            )))
        } else {
            Ok(returnData)
        }
    }

    async fn schedule_and_decode<T>(self, msg: CallBatchMsgKind<N>) -> TransportResult<T>
    where
        T: SolValue + From<<T::SolType as SolType>::RustType>,
    {
        let data = self.schedule(msg).await?;
        T::abi_decode(&data).map_err(TransportErrorKind::custom)
    }
}

struct CallBatchBackend<P, N: Network = Ethereum> {
    inner: Arc<P>,
    m3a: Address,
    wait: Duration,
    arbsys: bool,
    rx: mpsc::UnboundedReceiver<CallBatchMsg<N>>,
    pending: Vec<CallBatchMsg<N>>,
    _pd: PhantomData<N>,
}

impl<P: Provider<N> + 'static, N: Network> CallBatchBackend<P, N> {
    fn spawn(inner: Arc<P>, layer: &CallBatchLayer) -> mpsc::UnboundedSender<CallBatchMsg<N>> {
        let CallBatchLayer { m3a, wait, arbsys } = *layer;
        let (tx, rx) = mpsc::unbounded_channel();
        let this = Self { inner, m3a, wait, arbsys, rx, pending: Vec::new(), _pd: PhantomData };
        this.run().spawn_task();
        tx
    }

    async fn run(mut self) {
        'outer: loop {
            // Wait for the first message.
            debug_assert!(self.pending.is_empty());
            match self.rx.recv().await {
                Some(msg) => self.process_msg(msg),
                None => break,
            }

            // Handle all remaining messages after waiting the duration.
            debug_assert!(!self.pending.is_empty());
            sleep(self.wait).await;
            'inner: loop {
                match self.rx.try_recv() {
                    Ok(msg) => self.process_msg(msg),
                    Err(mpsc::error::TryRecvError::Empty) => break 'inner,
                    Err(mpsc::error::TryRecvError::Disconnected) => break 'outer,
                }
            }
            // No more messages, send the batch.
            self.send_batch().await;
        }
    }

    fn process_msg(&mut self, msg: CallBatchMsg<N>) {
        self.pending.push(msg);
    }

    async fn send_batch(&mut self) {
        let mut pending = std::mem::take(&mut self.pending);

        // Remove requests where the client has disconnected.
        pending.retain(|msg| !msg.tx.is_closed());

        // If all clients disconnected, return early.
        if pending.is_empty() {
            return;
        }

        // If there's only a single call, avoid batching and perform the request directly.
        if pending.len() == 1 {
            let msg = pending.into_iter().next().unwrap();
            let result = self.call_one(msg.kind).await;
            let _ = msg.tx.send(result);
            return;
        }

        let result = self.send_batch_inner(&pending).await;
        match result {
            Ok(results) => {
                debug_assert_eq!(results.len(), pending.len());
                for (result, msg) in results.into_iter().zip(pending) {
                    let _ = msg.tx.send(Ok(result));
                }
            }
            Err(e) => {
                for msg in pending {
                    let _ = msg.tx.send(Err(TransportErrorKind::custom_str(&e.to_string())));
                }
            }
        }
    }

    async fn call_one(&mut self, msg: CallBatchMsgKind<N>) -> TransportResult<IMulticall3::Result> {
        let m3_res =
            |success, return_data| IMulticall3::Result { success, returnData: return_data };
        match msg {
            CallBatchMsgKind::Call(tx) => self.inner.call(tx).await.map(|res| m3_res(true, res)),
            CallBatchMsgKind::BlockNumber => {
                self.inner.get_block_number().await.map(|res| m3_res(true, res.abi_encode().into()))
            }
            CallBatchMsgKind::ChainId => {
                self.inner.get_chain_id().await.map(|res| m3_res(true, res.abi_encode().into()))
            }
            CallBatchMsgKind::Balance(addr) => {
                self.inner.get_balance(addr).await.map(|res| m3_res(true, res.abi_encode().into()))
            }
        }
    }

    async fn send_batch_inner(
        &self,
        pending: &[CallBatchMsg<N>],
    ) -> TransportResult<Vec<IMulticall3::Result>> {
        let calls: Vec<_> =
            pending.iter().map(|msg| msg.kind.to_call3(self.m3a, self.arbsys)).collect();

        let tx = N::TransactionRequest::default()
            .with_to(self.m3a)
            .with_input(IMulticall3::aggregate3Call { calls }.abi_encode());

        let bytes = self.inner.call(tx).await?;
        if bytes.is_empty() {
            return Err(TransportErrorKind::custom_str(&format!(
                "Multicall3 not deployed at {}",
                self.m3a
            )));
        }

        let ret = IMulticall3::aggregate3Call::abi_decode_returns(&bytes)
            .map_err(TransportErrorKind::custom)?;
        Ok(ret)
    }
}

impl<P: Provider<N> + 'static, N: Network> Provider<N> for CallBatchProvider<P, N> {
    fn root(&self) -> &RootProvider<N> {
        self.provider.root()
    }

    fn call(&self, tx: <N as Network>::TransactionRequest) -> crate::EthCall<N, Bytes> {
        crate::EthCall::call(CallBatchCaller::new(self), tx)
    }

    fn get_block_number(
        &self,
    ) -> crate::ProviderCall<
        alloy_rpc_client::NoParams,
        alloy_primitives::U64,
        alloy_primitives::BlockNumber,
    > {
        crate::ProviderCall::BoxedFuture(Box::pin(
            self.inner.clone().schedule_and_decode::<u64>(CallBatchMsgKind::BlockNumber),
        ))
    }

    fn get_chain_id(
        &self,
    ) -> crate::ProviderCall<
        alloy_rpc_client::NoParams,
        alloy_primitives::U64,
        alloy_primitives::ChainId,
    > {
        crate::ProviderCall::BoxedFuture(Box::pin(
            self.inner.clone().schedule_and_decode::<u64>(CallBatchMsgKind::ChainId),
        ))
    }

    fn get_balance(&self, address: Address) -> crate::RpcWithBlock<Address, U256, U256> {
        let this = self.clone();
        crate::RpcWithBlock::new_provider(move |block| {
            if block != BlockId::latest() {
                this.provider.get_balance(address).block_id(block).into_future()
            } else {
                ProviderCall::BoxedFuture(Box::pin(
                    this.inner
                        .clone()
                        .schedule_and_decode::<U256>(CallBatchMsgKind::Balance(address)),
                ))
            }
        })
    }
}

struct CallBatchCaller<N: Network> {
    inner: CallBatchProviderInner<N>,
    weak: WeakClient,
}

impl<N: Network> CallBatchCaller<N> {
    fn new<P: Provider<N>>(provider: &CallBatchProvider<P, N>) -> Self {
        Self { inner: provider.inner.clone(), weak: provider.provider.weak_client() }
    }
}

impl<N: Network> Caller<N, Bytes> for CallBatchCaller<N> {
    fn call(
        &self,
        params: crate::EthCallParams<N>,
    ) -> TransportResult<crate::ProviderCall<crate::EthCallParams<N>, Bytes>> {
        if !self.inner.should_batch_call(&params) {
            return Caller::<N, Bytes>::call(&self.weak, params);
        }

        Ok(crate::ProviderCall::BoxedFuture(Box::pin(
            self.inner.clone().schedule(CallBatchMsgKind::Call(params.into_data())),
        )))
    }

    fn estimate_gas(
        &self,
        params: crate::EthCallParams<N>,
    ) -> TransportResult<crate::ProviderCall<crate::EthCallParams<N>, Bytes>> {
        Caller::<N, Bytes>::estimate_gas(&self.weak, params)
    }

    fn call_many(
        &self,
        params: crate::EthCallManyParams<'_>,
    ) -> TransportResult<crate::ProviderCall<crate::EthCallManyParams<'static>, Bytes>> {
        Caller::<N, Bytes>::call_many(&self.weak, params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProviderBuilder;
    use alloy_primitives::{address, hex};
    use alloy_rpc_types_eth::TransactionRequest;
    use alloy_transport::mock::Asserter;

    // https://etherscan.io/address/0xcA11bde05977b3631167028862bE2a173976CA11#code
    const MULTICALL3_DEPLOYED_CODE: &[u8] = &hex!("0x6080604052600436106100f35760003560e01c80634d2301cc1161008a578063a8b0574e11610059578063a8b0574e1461025a578063bce38bd714610275578063c3077fa914610288578063ee82ac5e1461029b57600080fd5b80634d2301cc146101ec57806372425d9d1461022157806382ad56cb1461023457806386d516e81461024757600080fd5b80633408e470116100c65780633408e47014610191578063399542e9146101a45780633e64a696146101c657806342cbb15c146101d957600080fd5b80630f28c97d146100f8578063174dea711461011a578063252dba421461013a57806327e86d6e1461015b575b600080fd5b34801561010457600080fd5b50425b6040519081526020015b60405180910390f35b61012d610128366004610a85565b6102ba565b6040516101119190610bbe565b61014d610148366004610a85565b6104ef565b604051610111929190610bd8565b34801561016757600080fd5b50437fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff0140610107565b34801561019d57600080fd5b5046610107565b6101b76101b2366004610c60565b610690565b60405161011193929190610cba565b3480156101d257600080fd5b5048610107565b3480156101e557600080fd5b5043610107565b3480156101f857600080fd5b50610107610207366004610ce2565b73ffffffffffffffffffffffffffffffffffffffff163190565b34801561022d57600080fd5b5044610107565b61012d610242366004610a85565b6106ab565b34801561025357600080fd5b5045610107565b34801561026657600080fd5b50604051418152602001610111565b61012d610283366004610c60565b61085a565b6101b7610296366004610a85565b610a1a565b3480156102a757600080fd5b506101076102b6366004610d18565b4090565b60606000828067ffffffffffffffff8111156102d8576102d8610d31565b60405190808252806020026020018201604052801561031e57816020015b6040805180820190915260008152606060208201528152602001906001900390816102f65790505b5092503660005b8281101561047757600085828151811061034157610341610d60565b6020026020010151905087878381811061035d5761035d610d60565b905060200281019061036f9190610d8f565b6040810135958601959093506103886020850185610ce2565b73ffffffffffffffffffffffffffffffffffffffff16816103ac6060870187610dcd565b6040516103ba929190610e32565b60006040518083038185875af1925050503d80600081146103f7576040519150601f19603f3d011682016040523d82523d6000602084013e6103fc565b606091505b50602080850191909152901515808452908501351761046d577f08c379a000000000000000000000000000000000000000000000000000000000600052602060045260176024527f4d756c746963616c6c333a2063616c6c206661696c656400000000000000000060445260846000fd5b5050600101610325565b508234146104e6576040517f08c379a000000000000000000000000000000000000000000000000000000000815260206004820152601a60248201527f4d756c746963616c6c333a2076616c7565206d69736d6174636800000000000060448201526064015b60405180910390fd5b50505092915050565b436060828067ffffffffffffffff81111561050c5761050c610d31565b60405190808252806020026020018201604052801561053f57816020015b606081526020019060019003908161052a5790505b5091503660005b8281101561068657600087878381811061056257610562610d60565b90506020028101906105749190610e42565b92506105836020840184610ce2565b73ffffffffffffffffffffffffffffffffffffffff166105a66020850185610dcd565b6040516105b4929190610e32565b6000604051808303816000865af19150503d80600081146105f1576040519150601f19603f3d011682016040523d82523d6000602084013e6105f6565b606091505b5086848151811061060957610609610d60565b602090810291909101015290508061067d576040517f08c379a000000000000000000000000000000000000000000000000000000000815260206004820152601760248201527f4d756c746963616c6c333a2063616c6c206661696c656400000000000000000060448201526064016104dd565b50600101610546565b5050509250929050565b43804060606106a086868661085a565b905093509350939050565b6060818067ffffffffffffffff8111156106c7576106c7610d31565b60405190808252806020026020018201604052801561070d57816020015b6040805180820190915260008152606060208201528152602001906001900390816106e55790505b5091503660005b828110156104e657600084828151811061073057610730610d60565b6020026020010151905086868381811061074c5761074c610d60565b905060200281019061075e9190610e76565b925061076d6020840184610ce2565b73ffffffffffffffffffffffffffffffffffffffff166107906040850185610dcd565b60405161079e929190610e32565b6000604051808303816000865af19150503d80600081146107db576040519150601f19603f3d011682016040523d82523d6000602084013e6107e0565b606091505b506020808401919091529015158083529084013517610851577f08c379a000000000000000000000000000000000000000000000000000000000600052602060045260176024527f4d756c746963616c6c333a2063616c6c206661696c656400000000000000000060445260646000fd5b50600101610714565b6060818067ffffffffffffffff81111561087657610876610d31565b6040519080825280602002602001820160405280156108bc57816020015b6040805180820190915260008152606060208201528152602001906001900390816108945790505b5091503660005b82811015610a105760008482815181106108df576108df610d60565b602002602001015190508686838181106108fb576108fb610d60565b905060200281019061090d9190610e42565b925061091c6020840184610ce2565b73ffffffffffffffffffffffffffffffffffffffff1661093f6020850185610dcd565b60405161094d929190610e32565b6000604051808303816000865af19150503d806000811461098a576040519150601f19603f3d011682016040523d82523d6000602084013e61098f565b606091505b506020830152151581528715610a07578051610a07576040517f08c379a000000000000000000000000000000000000000000000000000000000815260206004820152601760248201527f4d756c746963616c6c333a2063616c6c206661696c656400000000000000000060448201526064016104dd565b506001016108c3565b5050509392505050565b6000806060610a2b60018686610690565b919790965090945092505050565b60008083601f840112610a4b57600080fd5b50813567ffffffffffffffff811115610a6357600080fd5b6020830191508360208260051b8501011115610a7e57600080fd5b9250929050565b60008060208385031215610a9857600080fd5b823567ffffffffffffffff811115610aaf57600080fd5b610abb85828601610a39565b90969095509350505050565b6000815180845260005b81811015610aed57602081850181015186830182015201610ad1565b81811115610aff576000602083870101525b50601f017fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe0169290920160200192915050565b600082825180855260208086019550808260051b84010181860160005b84811015610bb1578583037fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe001895281518051151584528401516040858501819052610b9d81860183610ac7565b9a86019a9450505090830190600101610b4f565b5090979650505050505050565b602081526000610bd16020830184610b32565b9392505050565b600060408201848352602060408185015281855180845260608601915060608160051b870101935082870160005b82811015610c52577fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffa0888703018452610c40868351610ac7565b95509284019290840190600101610c06565b509398975050505050505050565b600080600060408486031215610c7557600080fd5b83358015158114610c8557600080fd5b9250602084013567ffffffffffffffff811115610ca157600080fd5b610cad86828701610a39565b9497909650939450505050565b838152826020820152606060408201526000610cd96060830184610b32565b95945050505050565b600060208284031215610cf457600080fd5b813573ffffffffffffffffffffffffffffffffffffffff81168114610bd157600080fd5b600060208284031215610d2a57600080fd5b5035919050565b7f4e487b7100000000000000000000000000000000000000000000000000000000600052604160045260246000fd5b7f4e487b7100000000000000000000000000000000000000000000000000000000600052603260045260246000fd5b600082357fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff81833603018112610dc357600080fd5b9190910192915050565b60008083357fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe1843603018112610e0257600080fd5b83018035915067ffffffffffffffff821115610e1d57600080fd5b602001915036819003821315610a7e57600080fd5b8183823760009101908152919050565b600082357fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffc1833603018112610dc357600080fd5b600082357fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffa1833603018112610dc357600080fdfea2646970667358221220bb2b5c71a328032f97c676ae39a1ec2148d3e5d6f73d95e9b17910152d61f16264736f6c634300080c0033");
    const COUNTER_ADDRESS: Address = address!("0x1234123412341234123412341234123412341234");
    const COUNTER_DEPLOYED_CODE: &[u8] = &hex!("0x6080604052348015600e575f5ffd5b5060043610603a575f3560e01c80633fb5c1cb14603e5780638381f58a14604f578063d09de08a146068575b5f5ffd5b604d6049366004607d565b5f55565b005b60565f5481565b60405190815260200160405180910390f35b604d5f805490806076836093565b9190505550565b5f60208284031215608c575f5ffd5b5035919050565b5f6001820160af57634e487b7160e01b5f52601160045260245ffd5b506001019056fea2646970667358221220f423ff7a9a85bf49c3769164d3bd24403940510478df27a6b1deac980db69e5664736f6c634300081b0033");

    fn push_m3_success(asserter: &Asserter, returns: &[(bool, Vec<u8>)]) {
        asserter.push_success(
            &returns
                .iter()
                .map(|&(success, ref data)| IMulticall3::Result {
                    success,
                    returnData: Bytes::copy_from_slice(data),
                })
                .collect::<Vec<_>>()
                .abi_encode(),
        )
    }

    #[tokio::test]
    async fn basic_mocked() {
        let asserter = Asserter::new();
        let provider =
            ProviderBuilder::new().with_call_batching().connect_mocked_client(asserter.clone());
        push_m3_success(
            &asserter,
            &[
                (true, 1.abi_encode()),  // IMulticall3::getBlockNumberCall
                (true, 2.abi_encode()),  // IMulticall3::getChainIdCall
                (false, 3.abi_encode()), // IMulticall3::getBlockNumberCall
                (false, 4.abi_encode()), // IMulticall3::getChainIdCall
            ],
        );
        let (block_number_ok, chain_id_ok, block_number_err, chain_id_err) = tokio::join!(
            provider.get_block_number(),
            provider.get_chain_id(),
            provider.get_block_number(),
            provider.get_chain_id(),
        );
        assert_eq!(block_number_ok.unwrap(), 1);
        assert_eq!(chain_id_ok.unwrap(), 2);
        assert!(block_number_err.unwrap_err().to_string().contains("reverted"));
        assert!(chain_id_err.unwrap_err().to_string().contains("reverted"));
        assert!(asserter.read_q().is_empty(), "only 1 request should've been made");
    }

    #[tokio::test]
    #[cfg(feature = "anvil-api")]
    async fn basic() {
        use crate::ext::AnvilApi;
        let provider = ProviderBuilder::new().with_call_batching().connect_anvil();
        provider.anvil_set_code(COUNTER_ADDRESS, COUNTER_DEPLOYED_CODE.into()).await.unwrap();
        provider.anvil_set_balance(COUNTER_ADDRESS, U256::from(123)).await.unwrap();

        let do_calls = || async {
            tokio::join!(
                provider.call(
                    TransactionRequest::default()
                        .with_to(COUNTER_ADDRESS)
                        .with_input(hex!("0x8381f58a")) // number()
                ),
                provider.call(
                    TransactionRequest::default()
                        .with_to(MULTICALL3_ADDRESS)
                        .with_input(IMulticall3::getBlockNumberCall {}.abi_encode())
                ),
                provider.get_block_number(),
                provider.get_chain_id(),
                provider.get_balance(COUNTER_ADDRESS),
            )
        };

        // Multicall3 has not yet been deployed.
        let (a, b, c, d, e) = do_calls().await;
        assert!(a.unwrap_err().to_string().contains("Multicall3 not deployed"));
        assert!(b.unwrap_err().to_string().contains("Multicall3 not deployed"));
        assert!(c.unwrap_err().to_string().contains("Multicall3 not deployed"));
        assert!(d.unwrap_err().to_string().contains("Multicall3 not deployed"));
        assert!(e.unwrap_err().to_string().contains("Multicall3 not deployed"));

        provider.anvil_set_code(MULTICALL3_ADDRESS, MULTICALL3_DEPLOYED_CODE.into()).await.unwrap();

        let (counter, block_number_raw, block_number, chain_id, balance) = do_calls().await;
        assert_eq!(counter.unwrap(), 0u64.abi_encode());
        assert_eq!(block_number_raw.unwrap(), 1u64.abi_encode());
        assert_eq!(block_number.unwrap(), 1);
        assert_eq!(chain_id.unwrap(), alloy_chains::NamedChain::AnvilHardhat as u64);
        assert_eq!(balance.unwrap(), U256::from(123));
    }

    #[tokio::test]
    #[ignore]
    async fn arbitrum() {
        let url = "https://arbitrum.rpc.subquery.network/public";

        let batched = ProviderBuilder::new().with_call_batching().connect(url).await.unwrap();

        let batch_layer = CallBatchLayer::new().arbitrum_compat();
        let batched_compat = ProviderBuilder::new().layer(batch_layer).connect(url).await.unwrap();

        // single call so won't go through multicall3
        let block = batched.get_block_number().await.unwrap();

        // force batching
        let (b, _) = tokio::join!(batched.get_block_number(), batched.get_chain_id());
        // we expect this to be the L1 block number
        let block_wrong = b.unwrap();

        // force batch transaction
        let (b, _) = tokio::join!(batched_compat.get_block_number(), batched.get_chain_id());
        // compat mode returns correct block
        let block_compat = b.unwrap();

        dbg!(block, block_wrong, block_compat);

        // arbitrum blocks move fast so we assert with some error margin
        assert!(block.abs_diff(block_compat) < 10);
        assert!(block.abs_diff(block_wrong) > 100_000);
    }
}
