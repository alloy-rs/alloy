use super::{EthLogsParams, LogOptions};
use crate::ProviderCall;
use alloy_eips::BlockNumberOrTag;
use alloy_json_rpc::{RpcRecv, RpcSend};
use alloy_network::Network;
use alloy_primitives::U256;
use alloy_rpc_client::{RpcClientInner, WeakClient};
use alloy_rpc_types_eth::{Filter, FilterBlockOption, Log};
use alloy_transport::{TransportErrorKind, TransportResult};
use std::sync::Arc;

/// Trait for making log retrieval calls.
pub trait LogsCaller<N, Resp>: Send + Sync + std::fmt::Debug
where
    N: Network,
    Resp: RpcRecv,
{
    /// Get logs with the given parameters.
    fn get_logs(
        &self,
        params: EthLogsParams<N>,
    ) -> TransportResult<ProviderCall<EthLogsParams<N>, Resp>>;
}

impl<N> LogsCaller<N, Vec<Log>> for WeakClient
where
    N: Network,
{
    fn get_logs(
        &self,
        params: EthLogsParams<N>,
    ) -> TransportResult<ProviderCall<EthLogsParams<N>, Vec<Log>>> {
        // Validate options
        if params.options.batch_size.is_some_and(|x| x == 0) {
            return Err(
                TransportErrorKind::Custom("LogOptions.batch_size must be > 0".into()).into()
            );
        }
        if params.options.max_count.is_some_and(|x| x == 0) {
            return Err(
                TransportErrorKind::Custom("LogOptions.max_count must be > 0".into()).into()
            );
        }

        // If no special options are provided, use the standard RPC call
        if !params.options.has_options() {
            return provider_rpc_call(self, "eth_getLogs", params);
        }

        // Use boxed future for batch logic
        let client = self.upgrade().ok_or_else(TransportErrorKind::backend_gone)?;

        Ok(ProviderCall::BoxedFuture(Box::pin(async move {
            get_logs_in_batch(client, &params.filter, &params.options).await
        })))
    }
}

/// Returns a [`ProviderCall::RpcCall`] from the provided method and [`EthLogsParams`].
fn provider_rpc_call<Req: RpcSend, Resp: RpcRecv>(
    client: &WeakClient,
    method: &'static str,
    params: Req,
) -> TransportResult<ProviderCall<Req, Resp>> {
    let client = client.upgrade().ok_or_else(TransportErrorKind::backend_gone)?;
    let rpc_call = client.request(method, params);
    Ok(ProviderCall::RpcCall(rpc_call))
}

/// Core batch log retrieval logic.
async fn get_logs_in_batch(
    client: Arc<RpcClientInner>,
    filter: &Filter,
    options: &LogOptions,
) -> TransportResult<Vec<Log>> {
    let mut all_logs = Vec::new();

    // Check if we need to batch and have a range filter
    if let (Some(batch_size), FilterBlockOption::Range { from_block, to_block }) =
        (options.batch_size, &filter.block_option)
    {
        // Extract block range - only support Number, Earliest, and Latest
        let from_num = match from_block {
            Some(BlockNumberOrTag::Number(n)) => *n,
            Some(BlockNumberOrTag::Earliest) => 0,
            _ => {
                // Fall back to standard method
                return client.request("eth_getLogs", (filter,)).await;
            }
        };

        let to_num = match to_block {
            Some(BlockNumberOrTag::Number(n)) => *n,
            Some(BlockNumberOrTag::Latest) => {
                let block_num: U256 = client.request_noparams("eth_blockNumber").await?;
                block_num.to::<u64>()
            }
            _ => {
                // Fall back to standard method
                return client.request("eth_getLogs", (filter,)).await;
            }
        };

        let mut batch_start = from_num;
        let mut batch_filter = filter.clone();
        while batch_start <= to_num {
            let batch_end = std::cmp::min(batch_start + batch_size - 1, to_num);

            batch_filter.block_option = FilterBlockOption::Range {
                from_block: Some(BlockNumberOrTag::Number(batch_start)),
                to_block: Some(BlockNumberOrTag::Number(batch_end)),
            };

            let batch_logs: Vec<Log> = client.request("eth_getLogs", (&batch_filter,)).await?;
            all_logs.extend(batch_logs);

            // Check if we should stop early due to count limit
            if options.max_count.is_some_and(|x| all_logs.len() >= x) {
                break;
            }

            batch_start = batch_end + 1;
        }
    } else {
        // No batching, fetch all logs at once
        all_logs = client.request("eth_getLogs", (filter,)).await?;
    }

    // Apply count limit at the end
    if let Some(max_count) = options.max_count {
        if all_logs.len() > max_count {
            all_logs.truncate(max_count);
        }
    }

    Ok(all_logs)
}
