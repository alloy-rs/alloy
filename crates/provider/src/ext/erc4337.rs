use crate::Provider;
use alloy_network::Network;
use alloy_primitives::{Address, Bytes};
use alloy_rpc_types_eth::erc4337::{
    SendUserOperation, SendUserOperationResponse, UserOperationGasEstimation, UserOperationReceipt,
};
use alloy_transport::TransportResult;

/// ERC-4337 Account Abstraction API
///
/// This module provides support for the `eth_sendUserOperation` RPC method
/// as defined in ERC-4337.
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
pub trait Erc4337Api<N>: Send + Sync {
    /// Sends a user operation to the bundler, as defined in ERC-4337.
    ///
    /// Entry point changes based on the user operation type.
    async fn send_user_operation(
        &self,
        user_op: SendUserOperation,
        entry_point: Address,
    ) -> TransportResult<SendUserOperationResponse>;

    /// Returns the list of supported entry points.
    async fn supported_entry_points(&self) -> TransportResult<Vec<Address>>;

    /// Returns the receipt for any user operation.
    ///
    /// Hash is the same returned by any user operation.
    ///
    /// Returns `None` if the user operation is unknown or not yet included in a block.
    async fn get_user_operation_receipt(
        &self,
        user_op_hash: Bytes,
    ) -> TransportResult<Option<UserOperationReceipt>>;

    /// Estimates the gas for a user operation.
    ///
    /// Entry point changes based on the user operation type.
    async fn estimate_user_operation_gas(
        &self,
        user_op: SendUserOperation,
        entry_point: Address,
    ) -> TransportResult<UserOperationGasEstimation>;
}

#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
impl<N, P> Erc4337Api<N> for P
where
    N: Network,
    P: Provider<N>,
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
    ) -> TransportResult<Option<UserOperationReceipt>> {
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
    use alloy_primitives::bytes;
    use alloy_transport::mock::Asserter;

    #[tokio::test]
    async fn get_user_operation_receipt_null() {
        let asserter = Asserter::new();
        let provider = ProviderBuilder::new().connect_mocked_client(asserter.clone());
        asserter.push_success(&serde_json::Value::Null);

        let receipt = provider
            .get_user_operation_receipt(bytes!(
                "77c0b560eb0b042902abc5e4c3fb0bfbef2f86c6d3ec9fdbc5ac0bd2d4eb1d4c"
            ))
            .await
            .unwrap();
        assert!(receipt.is_none());
    }
}
