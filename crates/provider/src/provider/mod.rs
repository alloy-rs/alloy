mod eth_call;
pub use eth_call::{Caller, EthCall, EthCallMany, EthCallManyParams, EthCallParams};

mod get_block;
#[cfg(feature = "pubsub")]
pub use get_block::SubFullBlocks;
pub use get_block::{EthGetBlock, EthGetBlockParams};

mod prov_call;
pub use prov_call::{BoxedFut, ProviderCall};

mod root;
pub use root::{builder, RootProvider};

mod sendable;
pub use sendable::{SendableTx, SendableTxErr};

mod r#trait;
pub use r#trait::{FilterPollerBuilder, Provider as ProviderTrait};

mod wallet;
pub use wallet::WalletProvider;

mod with_block;
pub use with_block::{ParamsWithBlock, RpcWithBlock};

mod multicall;
pub use multicall::*;

mod erased;
pub use erased::DynProvider;

#[cfg(feature = "pubsub")]
mod subscription;
#[cfg(feature = "pubsub")]
pub use subscription::GetSubscription;

mod web3_signer;
pub use web3_signer::Web3Signer;
mod recommended {
    use crate::{
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
        Identity, ProviderBuilder, RootProvider,
    };
    use alloy_network::AnyNetwork;

    /// A [`FillProvider`] with recommended fillers connected to [`AnyNetwork`].
    pub type AnyFillProvider = FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider<AnyNetwork>,
        AnyNetwork,
    >;

    /// An [`AnyFillProvider`] initializer.
    ///
    /// Helper type to instantiate an [`AnyFillProvider`] using [`Provider::connect`].
    ///
    /// One can connect a provider in a synchronous way using the [`Provider::connect_http`] method.
    ///
    /// Note:
    ///
    /// - Connecting using this provider enables the recommended fillers.
    /// - The resulting provider leverages the catch-all [`AnyNetwork`] type so that it can be used
    ///   across networks.
    ///
    /// If you wish to have more customizability over the resulting provider, use
    /// the [`ProviderBuilder`] directly.
    #[derive(Debug, Clone)]
    pub struct Provider;

    impl Provider {
        /// Instantiates a new [`AnyFillProvider`] using the given RPC endpoint.
        pub async fn connect(s: &str) -> alloy_transport::TransportResult<AnyFillProvider> {
            ProviderBuilder::new().network::<AnyNetwork>().connect(s).await
        }

        /// Instantiates a new [`AnyFillProvider`] using the given HTTP RPC endpoint.
        ///
        /// For connecting using WS endpoint or IPC endpoint, enable the "ws" or "ipc" feature and
        /// use [`Provider::connect`].
        #[cfg(all(not(feature = "hyper"), feature = "reqwest"))]
        pub fn connect_http(s: &str) -> alloy_transport::TransportResult<AnyFillProvider> {
            use alloy_rpc_client::BuiltInConnectionString;

            let conn = BuiltInConnectionString::try_as_http(s)?;
            Ok(ProviderBuilder::new().network::<AnyNetwork>().on_http(conn.url().unwrap()))
        }

        /// Instantiates a new [`AnyFillProvider`] using the given HTTP RPC endpoint.
        #[cfg(all(not(feature = "reqwest"), feature = "hyper"))]
        pub fn connect_http(s: &str) -> alloy_transport::TransportResult<AnyFillProvider> {
            let conn = BuiltInConnectionString::try_as_http(s)?;
            Ok(ProviderBuilder::new().network::<AnyNetwork>().on_hyper_http(conn.url().unwrap()))
        }
    }
}

pub use recommended::{AnyFillProvider, Provider};
