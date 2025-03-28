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

    pub type AnyFillProvider = FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider<AnyNetwork>,
        AnyNetwork,
    >;
    pub struct Provider;

    impl Provider {
        pub async fn connect(s: &str) -> AnyFillProvider {
            ProviderBuilder::new().network::<AnyNetwork>().connect(s).await.unwrap()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::ProviderTrait;
        use alloy_node_bindings::Anvil;

        #[tokio::test]
        async fn test_provider() {
            let anvil = Anvil::new().spawn();
            let provider = Provider::connect(&anvil.endpoint()).await;
            let block = provider.get_block_number().await.unwrap();
            println!("block: {:?}", block);
        }
    }
}

pub use recommended::{AnyFillProvider, Provider};
