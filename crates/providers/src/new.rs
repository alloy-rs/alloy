use crate::{
    chain::ChainStreamPoller,
    heart::{Heartbeat, HeartbeatHandle, PendingTransaction, WatchConfig},
};
use alloy_network::{Network, Transaction};
use alloy_primitives::{hex, Address, BlockNumber, B256, U256, U64};
use alloy_rpc_client::{ClientRef, RpcClient, WeakClient};
use alloy_rpc_types::Block;
use alloy_transport::{BoxTransport, Transport, TransportErrorKind, TransportResult};
use std::{
    marker::PhantomData,
    sync::{Arc, OnceLock, Weak},
};

/// A [`Provider`] in a [`Weak`] reference.
pub type WeakProvider<P> = Weak<P>;

/// A borrowed [`Provider`].
pub type ProviderRef<'a, P> = &'a P;

/// The root provider manages the RPC client and the heartbeat. It is at the
/// base of every provider stack.
pub struct RootProvider<N, T> {
    /// The inner state of the root provider.
    pub(crate) inner: Arc<RootProviderInner<N, T>>,
}

impl<N: Network, T: Transport> RootProvider<N, T> {
    pub(crate) fn new(client: RpcClient<T>) -> Self {
        Self { inner: Arc::new(RootProviderInner::new(client)) }
    }
}

impl<N: Network, T: Transport + Clone> RootProvider<N, T> {
    async fn new_pending_transaction(&self, tx_hash: B256) -> TransportResult<PendingTransaction> {
        // TODO: Make this configurable.
        let cfg = WatchConfig::new(tx_hash);
        self.get_heart().watch_tx(cfg).await.map_err(|_| TransportErrorKind::backend_gone())
    }

    #[inline]
    fn get_heart(&self) -> &HeartbeatHandle {
        self.inner.heart.get_or_init(|| {
            let poller = ChainStreamPoller::from_root(self);
            // TODO: Can we avoid `Box::pin` here?
            Heartbeat::new(Box::pin(poller.into_stream())).spawn()
        })
    }
}

/// The root provider manages the RPC client and the heartbeat. It is at the
/// base of every provider stack.
pub(crate) struct RootProviderInner<N, T> {
    client: RpcClient<T>,
    heart: OnceLock<HeartbeatHandle>,
    _network: PhantomData<N>,
}

impl<N, T> RootProviderInner<N, T> {
    pub(crate) fn new(client: RpcClient<T>) -> Self {
        Self { client, heart: OnceLock::new(), _network: PhantomData }
    }

    fn weak_client(&self) -> WeakClient<T> {
        self.client.get_weak()
    }

    fn client_ref(&self) -> ClientRef<'_, T> {
        self.client.get_ref()
    }
}

/// Provider is parameterized with a network and a transport. The default
/// transport is type-erased, but you can do `Provider<N, Http>`.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[auto_impl::auto_impl(&, &mut, Rc, Arc, Box)]
pub trait Provider<N: Network, T: Transport + Clone = BoxTransport>: Send + Sync {
    /// Returns the RPC client used to send requests.
    fn client(&self) -> ClientRef<'_, T>;

    /// Returns a [`Weak`] RPC client used to send requests.
    fn weak_client(&self) -> WeakClient<T>;

    async fn new_pending_transaction(&self, tx_hash: B256) -> TransportResult<PendingTransaction>;

    async fn estimate_gas(&self, tx: &N::TransactionRequest) -> TransportResult<U256> {
        self.client().prepare("eth_estimateGas", (tx,)).await
    }

    /// Get the last block number available.
    async fn get_block_number(&self) -> TransportResult<BlockNumber> {
        self.client().prepare("eth_blockNumber", ()).await.map(|num: U64| num.to::<u64>())
    }

    /// Get the transaction count for an address. Used for finding the
    /// appropriate nonce.
    ///
    /// TODO: block number/hash/tag
    async fn get_transaction_count(&self, address: Address) -> TransportResult<U256> {
        self.client().prepare("eth_getTransactionCount", (address, "latest")).await
    }

    /// Get a block by its number.
    ///
    /// TODO: Network associate
    async fn get_block_by_number(
        &self,
        number: BlockNumber,
        hydrate: bool,
    ) -> TransportResult<Block> {
        self.client().prepare("eth_getBlockByNumber", (number, hydrate)).await
    }

    /// Populate the gas limit for a transaction.
    async fn populate_gas(&self, tx: &mut N::TransactionRequest) -> TransportResult<()> {
        let gas = self.estimate_gas(&*tx).await?;
        if let Ok(gas) = gas.try_into() {
            tx.set_gas_limit(gas);
        }
        Ok(())
    }

    /// Broadcasts a transaction, returning a [`PendingTransaction`] that resolves once the
    /// transaction has been confirmed.
    async fn send_transaction(
        &self,
        tx: &N::TransactionRequest,
    ) -> TransportResult<PendingTransaction> {
        let tx_hash = self.client().prepare("eth_sendTransaction", (tx,)).await?;
        self.new_pending_transaction(tx_hash).await
    }

    /// Broadcasts a transaction's raw RLP bytes, returning a [`PendingTransaction`] that resolves
    /// once the transaction has been confirmed.
    async fn send_raw_transaction(&self, rlp_bytes: &[u8]) -> TransportResult<PendingTransaction> {
        let rlp_hex = hex::encode(rlp_bytes);
        let tx_hash = self.client().prepare("eth_sendRawTransaction", (rlp_hex,)).await?;
        self.new_pending_transaction(tx_hash).await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<N: Network, T: Transport + Clone> Provider<N, T> for RootProvider<N, T> {
    #[inline]
    fn client(&self) -> ClientRef<'_, T> {
        self.inner.client_ref()
    }

    #[inline]
    fn weak_client(&self) -> WeakClient<T> {
        self.inner.weak_client()
    }

    #[inline]
    async fn new_pending_transaction(&self, tx_hash: B256) -> TransportResult<PendingTransaction> {
        RootProvider::new_pending_transaction(self, tx_hash).await
    }
}

// Internal implementation for [`chain_stream_poller`].
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<N: Network, T: Transport + Clone> Provider<N, T> for RootProviderInner<N, T> {
    #[inline]
    fn client(&self) -> ClientRef<'_, T> {
        self.client_ref()
    }

    #[inline]
    fn weak_client(&self) -> WeakClient<T> {
        self.weak_client()
    }

    #[inline]
    async fn new_pending_transaction(&self, _tx_hash: B256) -> TransportResult<PendingTransaction> {
        unreachable!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::address;
    use alloy_rpc_types::request::{TransactionInput, TransactionRequest};
    use alloy_transport_http::Http;
    use reqwest::Client;

    struct _ObjectSafe<N: Network>(dyn Provider<N>);

    #[derive(Clone)]
    struct TxLegacy(alloy_consensus::TxLegacy);
    impl serde::Serialize for TxLegacy {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let tx = &self.0;
            TransactionRequest {
                from: None,
                to: tx.to().to(),
                gas_price: tx.gas_price(),
                max_fee_per_gas: None,
                max_priority_fee_per_gas: None,
                max_fee_per_blob_gas: None,
                gas: Some(U256::from(tx.gas_limit())),
                value: Some(tx.value()),
                input: TransactionInput::new(tx.input().to_vec().into()),
                nonce: Some(U64::from(tx.nonce())),
                chain_id: tx.chain_id().map(U64::from),
                access_list: None,
                transaction_type: None,
                blob_versioned_hashes: None,
                sidecar: None,
                other: Default::default(),
            }
            .serialize(serializer)
        }
    }
    impl<'de> serde::Deserialize<'de> for TxLegacy {
        fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            unimplemented!()
        }
    }
    #[allow(unused)]
    impl alloy_network::Transaction for TxLegacy {
        type Signature = ();

        fn encode_for_signing(&self, out: &mut dyn alloy_rlp::BufMut) {
            todo!()
        }

        fn payload_len_for_signature(&self) -> usize {
            todo!()
        }

        fn into_signed(
            self,
            signature: alloy_primitives::Signature,
        ) -> alloy_network::Signed<Self, Self::Signature>
        where
            Self: Sized,
        {
            todo!()
        }

        fn encode_signed(
            &self,
            signature: &alloy_primitives::Signature,
            out: &mut dyn alloy_primitives::bytes::BufMut,
        ) {
            todo!()
        }

        fn decode_signed(buf: &mut &[u8]) -> alloy_rlp::Result<alloy_network::Signed<Self>>
        where
            Self: Sized,
        {
            todo!()
        }

        fn input(&self) -> &[u8] {
            todo!()
        }

        fn input_mut(&mut self) -> &mut alloy_primitives::Bytes {
            todo!()
        }

        fn set_input(&mut self, data: alloy_primitives::Bytes) {
            todo!()
        }

        fn to(&self) -> alloy_network::TxKind {
            todo!()
        }

        fn set_to(&mut self, to: alloy_network::TxKind) {
            todo!()
        }

        fn value(&self) -> U256 {
            todo!()
        }

        fn set_value(&mut self, value: U256) {
            todo!()
        }

        fn chain_id(&self) -> Option<alloy_primitives::ChainId> {
            todo!()
        }

        fn set_chain_id(&mut self, chain_id: alloy_primitives::ChainId) {
            todo!()
        }

        fn nonce(&self) -> u64 {
            todo!()
        }

        fn set_nonce(&mut self, nonce: u64) {
            todo!()
        }

        fn gas_limit(&self) -> u64 {
            todo!()
        }

        fn set_gas_limit(&mut self, limit: u64) {
            todo!()
        }

        fn gas_price(&self) -> Option<U256> {
            todo!()
        }

        fn set_gas_price(&mut self, price: U256) {
            todo!()
        }
    }

    struct TmpNetwork;
    impl Network for TmpNetwork {
        type TxEnvelope = alloy_consensus::TxEnvelope;
        type ReceiptEnvelope = alloy_consensus::ReceiptEnvelope;
        type Header = ();
        type TransactionRequest = TxLegacy;
        type TransactionResponse = ();
        type ReceiptResponse = ();
        type HeaderResponse = ();
    }

    fn init_tracing() {
        let _ = tracing_subscriber::fmt::try_init();
    }

    #[tokio::test]
    async fn test_send_tx() {
        init_tracing();

        let anvil = alloy_node_bindings::Anvil::new().block_time(1u64).spawn();
        let url = anvil.endpoint().parse().unwrap();
        let http = Http::<Client>::new(url);
        let provider = RootProvider::<TmpNetwork, _>::new(RpcClient::new(http, true));

        let tx = alloy_consensus::TxLegacy {
            value: U256::from(100),
            to: address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").into(),
            gas_price: 20e9 as u128,
            gas_limit: 21000,
            ..Default::default()
        };
        let pending_tx = provider.send_transaction(&TxLegacy(tx)).await.expect("failed to send tx");
        eprintln!("{pending_tx:?}");
        let () = pending_tx.await.expect("failed to await pending tx");
    }
}
