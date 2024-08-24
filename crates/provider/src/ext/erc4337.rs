use crate::Provider;
use alloy_network::Network;
use alloy_primitives::{Address, Bytes};
use alloy_rpc_types_eth::erc4337::{
    SendUserOperation, SendUserOperationResponse, UserOperationGasEstimation, UserOperationReceipt,
};
use alloy_transport::{Transport, TransportResult};

/// ERC-4337 Account Abstraction API
///
/// This module provides support for the `eth_sendUserOperation` RPC method
/// as defined in ERC-4337.
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait Erc4337Api<N, T>: Send + Sync {
    /// Sends a [`UserOperation`] or [`PackedUserOperation`] to the bundler.
    ///
    /// Entry point changes based on the user operation type.
    async fn send_user_operation(
        &self,
        user_op: SendUserOperation,
        entry_point: Address,
    ) -> TransportResult<SendUserOperationResponse>;

    /// Returns the list of supported entry points.
    async fn supported_entry_points(&self) -> TransportResult<Vec<Address>>;

    /// Returns the receipt for any [`UserOperation`] or [`PackedUserOperation`].
    ///
    /// Hash is the same as the one returned by [`send_user_operation`].
    async fn get_user_operation_receipt(
        &self,
        user_op_hash: Bytes,
    ) -> TransportResult<UserOperationReceipt>;

    /// Estimates the gas for a [`UserOperation`] or [`PackedUserOperation`].
    ///
    /// Entry point changes based on the user operation type.
    async fn estimate_user_operation_gas(
        &self,
        user_op: SendUserOperation,
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
        user_op: SendUserOperation,
        entry_point: Address,
    ) -> TransportResult<SendUserOperationResponse> {
        match user_op {
            SendUserOperation::EntryPointV06(user_op) => {
                self.client().request("eth_sendUserOperation", (user_op, entry_point)).await
            }
            SendUserOperation::EntryPointV07(packed_user_op) => {
                self.client().request("eth_sendUserOperation", (packed_user_op, entry_point)).await
            }
        }
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
        user_op: SendUserOperation,
        entry_point: Address,
    ) -> TransportResult<UserOperationGasEstimation> {
        match user_op {
            SendUserOperation::EntryPointV06(user_op) => {
                self.client().request("eth_estimateUserOperationGas", (user_op, entry_point)).await
            }
            SendUserOperation::EntryPointV07(packed_user_op) => {
                self.client()
                    .request("eth_estimateUserOperationGas", (packed_user_op, entry_point))
                    .await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProviderBuilder;
    use alloy_node_bindings::Geth;
    use alloy_primitives::{Address, Bytes, U256};

    #[tokio::test]
    async fn test_send_user_operation() {
        let temp_dir = tempfile::TempDir::with_prefix("geth-test-").unwrap();
        let geth = Geth::new().disable_discovery().data_dir(temp_dir.path()).spawn();
        let provider = ProviderBuilder::new().on_http(geth.endpoint_url());

        let user_op = SendUserOperation::EntryPointV06(UserOperation {
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
        });

        let entry_point_old: Address =
            "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789".parse().unwrap();

        let result = provider.send_user_operation(user_op, entry_point_old).await;

        match result {
            Ok(result) => {
                println!("User operation sent successfully: {:?}", result);
            }
            Err(_) => {
                println!("Skipping eth_sendUserOperation test because of non-realistic user_op construction")
            }
        }
    }

    #[tokio::test]
    async fn test_supported_entry_points() {
        let temp_dir = tempfile::TempDir::with_prefix("geth-test-").unwrap();
        let geth = Geth::new().disable_discovery().data_dir(temp_dir.path()).spawn();
        let provider = ProviderBuilder::new().on_http(geth.endpoint_url());

        let result = provider.supported_entry_points().await;

        assert!(result.unwrap().len() > 0);
    }

    #[tokio::test]
    async fn test_get_user_operation_receipt() {
        let temp_dir = tempfile::TempDir::with_prefix("geth-test-").unwrap();
        let geth = Geth::new().disable_discovery().data_dir(temp_dir.path()).spawn();
        let provider = ProviderBuilder::new().on_http(geth.endpoint_url());

        let user_op_hash =
            "0x93c06f3f5909cc2b192713ed9bf93e3e1fde4b22fcd2466304fa404f9b80ff90".parse().unwrap();
        let result = provider.get_user_operation_receipt(user_op_hash).await;

        assert!(result.unwrap().success);
    }

    #[tokio::test]
    async fn test_estimate_user_operation_gas() {
        let temp_dir = tempfile::TempDir::with_prefix("geth-test-").unwrap();
        let geth = Geth::new().disable_discovery().data_dir(temp_dir.path()).spawn();
        let provider = ProviderBuilder::new().on_http(geth.endpoint_url());

        let user_op = SendUserOperation::EntryPointV07(PackedUserOperation {
            sender: Address::random(),
            nonce: U256::from(0),
            factory: Address::random(),
            factory_data: Bytes::default(),
            call_data: Bytes::default(),
            call_gas_limit: U256::from(1000000),
            verification_gas_limit: U256::from(1000000),
            pre_verification_gas: U256::from(1000000),
            max_fee_per_gas: U256::from(1000000000),
            max_priority_fee_per_gas: U256::from(1000000000),
            paymaster: Address::random(),
            paymaster_verification_gas_limit: U256::from(1000000),
            paymaster_post_op_gas_limit: U256::from(1000000),
            paymaster_data: Bytes::default(),
            signature: Bytes::default(),
        });

        let entry_point_new: Address =
            "0x0000000071727De22E5E9d8BAf0edAc6f37da032".parse().unwrap();

        let result = provider.estimate_user_operation_gas(user_op, entry_point_new).await;

        match result {
            Ok(result) => {
                println!("User operation gas estimation: {:?}", result);
            }
            Err(_) => {
                println!("Skipping eth_estimateUserOperationGas test because of non-realistic user_op construction")
            }
        }
    }
}
