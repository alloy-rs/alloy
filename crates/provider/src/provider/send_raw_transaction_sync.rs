use alloy_network::Network;
use alloy_rpc_client::{ClientRef, RpcCall};
use alloy_transport::TransportResult;
use serde::ser::{Error as _, SerializeSeq};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use tokio::time::{timeout as timeout_future, Timeout};

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
use wasmtimer::tokio::{timeout as timeout_future, Timeout};

/// Parameters for an `eth_sendRawTransactionSync` request.
#[derive(Clone, Debug)]
struct SendRawTransactionSyncParams {
    encoded_tx: String,
    server_timeout: Option<Duration>,
}

impl serde::Serialize for SendRawTransactionSyncParams {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq =
            serializer.serialize_seq(Some(1 + usize::from(self.server_timeout.is_some())))?;
        seq.serialize_element(&self.encoded_tx)?;

        if let Some(server_timeout) = self.server_timeout {
            let timeout_ms = u64::try_from(server_timeout.as_millis())
                .map_err(|_| S::Error::custom("server timeout exceeds u64 milliseconds"))?;
            if timeout_ms == 0 {
                return Err(S::Error::custom("server timeout must be at least one millisecond"));
            }
            seq.serialize_element(&timeout_ms)?;
        }

        seq.end()
    }
}

/// Future for an `eth_sendRawTransactionSync` request.
///
/// This type is returned by [`Provider::send_raw_transaction_sync`]. Use [`Self::timeout`] to
/// limit how long the client waits for the response, and [`Self::server_timeout`] to configure the
/// optional server-side timeout parameter defined by [EIP-7966].
///
/// [EIP-7966]: https://eips.ethereum.org/EIPS/eip-7966
/// [`Provider::send_raw_transaction_sync`]: crate::Provider::send_raw_transaction_sync
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project::pin_project]
pub struct SendRawTransactionSync<N: Network> {
    #[pin]
    inner: RpcCall<SendRawTransactionSyncParams, N::ReceiptResponse>,
}

impl<N: Network> SendRawTransactionSync<N> {
    pub(crate) fn new(client: ClientRef<'_>, encoded_tx: &[u8]) -> Self {
        let params = SendRawTransactionSyncParams {
            encoded_tx: alloy_primitives::hex::encode_prefixed(encoded_tx),
            server_timeout: None,
        };
        Self { inner: client.request("eth_sendRawTransactionSync", params) }
    }

    /// Wraps this future in a client-side timeout.
    ///
    /// The timeout only stops waiting for the response. The transaction may still have been
    /// submitted to the network, so it may be unsafe to retry it without checking its status.
    /// Awaiting the returned future produces a timeout result around the existing transport result,
    /// so the two error cases can be handled separately.
    ///
    /// # Panics
    ///
    /// On Tokio-backed targets, including WASI, the returned future panics when polled if there
    /// is no current Tokio timer, for example when polled outside of a Tokio runtime.
    pub fn timeout(self, duration: Duration) -> Timeout<Self> {
        timeout_future(duration, self)
    }

    /// Sets the optional server-side timeout for transaction inclusion.
    ///
    /// The timeout is serialized in milliseconds as the optional second
    /// `eth_sendRawTransactionSync` parameter defined by [EIP-7966]. Unlike [`Self::timeout`], this
    /// is enforced by the RPC server and is returned as a regular RPC error if it elapses.
    ///
    /// Durations shorter than one millisecond or longer than [`u64::MAX`] milliseconds fail to
    /// serialize and are returned as transport serialization errors.
    ///
    /// [EIP-7966]: https://eips.ethereum.org/EIPS/eip-7966
    pub fn server_timeout(mut self, server_timeout: Option<Duration>) -> Self {
        self.inner.params().server_timeout = server_timeout;
        self
    }
}

impl<N: Network> std::fmt::Debug for SendRawTransactionSync<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SendRawTransactionSync").field("inner", &self.inner).finish()
    }
}

impl<N: Network> Future for SendRawTransactionSync<N> {
    type Output = TransportResult<N::ReceiptResponse>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().inner.poll(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_json_rpc::ResponsePacket;
    use alloy_network::Ethereum;
    use alloy_rpc_client::RpcClient;
    use alloy_transport::TransportFut;
    use serde_json::json;

    #[test]
    fn serializes_optional_server_timeout() {
        let params =
            SendRawTransactionSyncParams { encoded_tx: "0x1234".into(), server_timeout: None };
        assert_eq!(serde_json::to_value(&params).unwrap(), json!(["0x1234"]));

        let params =
            SendRawTransactionSyncParams { server_timeout: Some(Duration::from_secs(5)), ..params };
        assert_eq!(serde_json::to_value(&params).unwrap(), json!(["0x1234", 5000]));
    }

    #[test]
    fn rejects_invalid_server_timeout() {
        let params = SendRawTransactionSyncParams {
            encoded_tx: "0x1234".into(),
            server_timeout: Some(Duration::from_nanos(1)),
        };
        assert!(serde_json::to_value(&params).is_err());

        let params = SendRawTransactionSyncParams { server_timeout: Some(Duration::MAX), ..params };
        assert!(serde_json::to_value(&params).is_err());
    }

    #[tokio::test]
    async fn client_timeout() {
        let transport = tower::service_fn(|_| -> TransportFut<'static, ResponsePacket> {
            Box::pin(std::future::pending())
        });
        let client = RpcClient::builder().transport(transport, true);
        let call = SendRawTransactionSync::<Ethereum>::new(client.inner().as_ref(), &[0x12, 0x34]);

        assert!(call.timeout(Duration::ZERO).await.is_err());
    }

    #[test]
    fn helper_updates_server_timeout() {
        let transport = tower::service_fn(|_| -> TransportFut<'static, ResponsePacket> {
            Box::pin(std::future::pending())
        });
        let client = RpcClient::builder().transport(transport, true);
        let call = SendRawTransactionSync::<Ethereum>::new(client.inner().as_ref(), &[0x12, 0x34])
            .server_timeout(Some(Duration::from_secs(5)));

        assert_eq!(
            serde_json::to_value(&call.inner.request().params).unwrap(),
            json!(["0x1234", 5000])
        );
    }
}
