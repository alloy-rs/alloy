use crate::Provider;
use alloy_network::Network;
use alloy_primitives::Address;
use alloy_rpc_types_eth::eip4337::{SendUserOperationResponse, UserOperation};
use alloy_transport::{Transport, TransportResult};

/// EIP-4337 Account Abstraction API
/// This module provides support for the `eth_sendUserOperation` RPC method
/// as defined in EIP-4337.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait Eip4337Api<N, T>: Send + Sync {
    /// Sends a UserOperation to the bundler.
    async fn eth_send_user_operation(
        &self,
        user_op: UserOperation,
        entry_point: Address,
    ) -> TransportResult<SendUserOperationResponse>;
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<N, T, P> Eip4337Api<N, T> for P
where
    N: Network,
    T: Transport + Clone,
    P: Provider<T, N>,
{
    async fn eth_send_user_operation(
        &self,
        user_op: UserOperation,
        entry_point: Address,
    ) -> TransportResult<SendUserOperationResponse> {
        self.client().request("eth_sendUserOperation", (user_op, entry_point)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProviderBuilder;
    use alloy_node_bindings::Geth;
    use alloy_primitives::{Address, Bytes, U256};

    #[tokio::test]
    async fn test_eth_send_user_operation() {
        let temp_dir = tempfile::TempDir::with_prefix("geth-test-").unwrap();
        let geth = Geth::new().disable_discovery().data_dir(temp_dir.path()).spawn();
        let provider = ProviderBuilder::new().on_http(geth.endpoint_url());

        let user_op = UserOperation {
            sender: Address::random(),
            nonce: U256::from(0),
            init_code: Bytes::default(),
            call_data: Bytes::default(),
            call_gas_limit: U256::from(1000000),
            verification_gas_limit: U256::from(1000000),
            pre_verification_gas: U256::from(1000000),
            max_fee_per_gas: U256::from(1000000000),
            max_priority_fee_per_gas: U256::from(1000000000),
            paymaster_and_data: Bytes::default(),
            signature: Bytes::default(),
        };

        let entry_point = Address::random();

        let result = provider.eth_send_user_operation(user_op, entry_point).await;

        // Note: This is a filler test and will fail, need to come up with a better mocking/approach.
        assert!(result.is_ok());
    }
}
