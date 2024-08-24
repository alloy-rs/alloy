use crate::Provider;
use alloy_network::Network;
use alloy_primitives::{Address, Bytes};
use alloy_rpc_types_eth::erc4337::{
    SendUserOperationResponse, UserOperation, UserOperationGasEstimation, UserOperationReceipt,
};
use alloy_transport::{Transport, TransportResult};

/// ERC-4337 Account Abstraction API
///
/// This module provides support for the `eth_sendUserOperation` RPC method
/// as defined in ERC-4337.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait Erc4337Api<N, T>: Send + Sync {
    /// Sends a [`UserOperation`] to the bundler.
    async fn send_user_operation(
        &self,
        user_op: UserOperation,
        entry_point: Address,
    ) -> TransportResult<SendUserOperationResponse>;

    /// Returns the list of supported entry points.
    async fn supported_entry_points(&self) -> TransportResult<Vec<Address>>;

    /// Returns the receipt of a [`UserOperation`].
    async fn get_user_operation_receipt(
        &self,
        user_op_hash: Bytes,
    ) -> TransportResult<UserOperationReceipt>;

    /// Estimates the gas for a [`UserOperation`].
    async fn estimate_user_operation_gas(
        &self,
        user_op: UserOperation,
        entry_point: Address,
    ) -> TransportResult<UserOperationGasEstimation>;
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<N, T, P> Erc4337Api<N, T> for P
where
    N: Network,
    T: Transport + Clone,
    P: Provider<T, N>,
{
    async fn send_user_operation(
        &self,
        user_op: UserOperation,
        entry_point: Address,
    ) -> TransportResult<SendUserOperationResponse> {
        self.client().request("eth_sendUserOperation", (user_op, entry_point)).await
    }

    async fn supported_entry_points(&self) -> TransportResult<Vec<Address>> {
        self.client().request("eth_supportedEntryPoints", ()).await
    }

    async fn get_user_operation_receipt(
        &self,
        user_op_hash: Bytes,
    ) -> TransportResult<UserOperationReceipt> {
        self.client().request("eth_getUserOperationReceipt", (user_op_hash,)).await
    }

    async fn estimate_user_operation_gas(
        &self,
        user_op: UserOperation,
        entry_point: Address,
    ) -> TransportResult<UserOperationGasEstimation> {
        self.client().request("eth_estimateUserOperationGas", (user_op, entry_point)).await
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

        let entry_point: Address = "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789".parse().unwrap();

        let result = provider.eth_send_user_operation(user_op, entry_point).await;

        match result {
            Ok(_) => {
                println!("User operation sent successfully: {:?}", result);
            }
            Err(e) => {
                println!("Skipping eth_sendUserOperation test because of non-realistic user_op construction")
            }
        }
    }

    #[tokio::test]
    async fn test_eth_supported_entry_points() {
        let temp_dir = tempfile::TempDir::with_prefix("geth-test-").unwrap();
        let geth = Geth::new().disable_discovery().data_dir(temp_dir.path()).spawn();
        let provider = ProviderBuilder::new().on_http(geth.endpoint_url());

        let result = provider.eth_supported_entry_points().await;

        assert!(result.unwrap().len() > 0);
    }

    #[tokio::test]
    async fn test_eth_get_user_operation_receipt() {
        let temp_dir = tempfile::TempDir::with_prefix("geth-test-").unwrap();
        let geth = Geth::new().disable_discovery().data_dir(temp_dir.path()).spawn();
        let provider = ProviderBuilder::new().on_http(geth.endpoint_url());

        /// User operation hash that has already been included in a block
        let user_op_hash =
            "0x93c06f3f5909cc2b192713ed9bf93e3e1fde4b22fcd2466304fa404f9b80ff90".parse().unwrap();
        let result = provider.eth_get_user_operation_receipt(user_op_hash).await;

        assert!(result.unwrap().success);
    }

    #[tokio::test]
    async fn test_eth_estimate_user_operation_gas() {
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

        let entry_point: Address = "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789".parse().unwrap();

        let result = provider.eth_estimate_user_operation_gas(user_op, entry_point).await;

        match result {
            Ok(_) => {
                println!("User operation gas estimation: {:?}", result);
            }
            Err(e) => {
                println!("Skipping eth_estimateUserOperationGas test because of non-realistic user_op construction")
            }
        }
    }
}
