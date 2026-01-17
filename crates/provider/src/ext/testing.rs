//! Testing namespace for building a block in a single call.
//!
//! This follows the `testing_buildBlockV1` specification.

use crate::Provider;
use alloy_json_rpc::RpcRecv;
use alloy_network::Network;
use alloy_rpc_types_engine::{ExecutionPayloadEnvelopeV5, TestingBuildBlockRequestV1};
use alloy_transport::TransportResult;

/// Extension trait that gives access to Testing API RPC methods.
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
pub trait TestingApi<N>: Send + Sync {
    /// Builds a block using the provided method and request, returning a generic response type.
    async fn build_block<R: RpcRecv>(
        &self,
        method: &'static str,
        request: TestingBuildBlockRequestV1,
    ) -> TransportResult<R>;

    /// Builds a block using the provided parent, payload attributes, and transactions.
    async fn build_block_v1(
        &self,
        request: TestingBuildBlockRequestV1,
    ) -> TransportResult<ExecutionPayloadEnvelopeV5>;
}

#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
impl<N, P> TestingApi<N> for P
where
    N: Network,
    P: Provider<N>,
{
    async fn build_block<R: RpcRecv>(
        &self,
        method: &'static str,
        request: TestingBuildBlockRequestV1,
    ) -> TransportResult<R> {
        self.client().request(method, (request,)).await
    }

    async fn build_block_v1(
        &self,
        request: TestingBuildBlockRequestV1,
    ) -> TransportResult<ExecutionPayloadEnvelopeV5> {
        self.build_block("testing_buildBlockV1", request).await
    }
}
