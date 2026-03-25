use crate::WsConnect;
use alloy_pubsub::PubSubConnect;
use alloy_transport::{Authorization, TransportErrorKind, TransportResult};
use mpp::{client::PaymentProvider, format_authorization, PaymentChallenge};
use tokio_tungstenite::tungstenite;
use tracing::debug;

/// A wrapper around [`WsConnect`] that handles MPP 402 payment challenges
/// during the WebSocket upgrade handshake.
///
/// If the server responds with `402 Payment Required` and a
/// `WWW-Authenticate: Payment ...` header during connection, this wrapper:
/// 1. Parses the [`PaymentChallenge`]
/// 2. Calls [`PaymentProvider::pay`] to obtain a credential
/// 3. Retries the connection with an `Authorization: Payment ...` header
///
/// # Example
///
/// ```ignore
/// use alloy_transport_ws::{WsConnect, MppWsConnect};
///
/// let ws = WsConnect::new("wss://example.com/rpc");
/// let client = ClientBuilder::default()
///     .pubsub(MppWsConnect::new(ws, my_provider))
///     .await?;
/// ```
///
/// [`PaymentChallenge`]: mpp::PaymentChallenge
#[derive(Clone, Debug)]
pub struct MppWsConnect<P> {
    inner: WsConnect,
    provider: P,
}

impl<P> MppWsConnect<P> {
    /// Create a new [`MppWsConnect`] wrapping the given [`WsConnect`] and
    /// [`PaymentProvider`].
    pub const fn new(inner: WsConnect, provider: P) -> Self {
        Self { inner, provider }
    }
}

impl<P> PubSubConnect for MppWsConnect<P>
where
    P: PaymentProvider + 'static,
{
    fn is_local(&self) -> bool {
        self.inner.is_local()
    }

    async fn connect(&self) -> TransportResult<alloy_pubsub::ConnectionHandle> {
        match self.inner.connect().await {
            Ok(handle) => Ok(handle),
            Err(err) => {
                // Extract the tungstenite HTTP error from the transport error chain.
                let challenge = extract_402_challenge(&err).ok_or(err)?;

                debug!("received 402 during WS upgrade, attempting MPP payment");

                let credential = self.provider.pay(&challenge).await.map_err(|e| {
                    TransportErrorKind::custom_str(&format!("MPP payment failed: {e}"))
                })?;

                let auth_value = format_authorization(&credential).map_err(|e| {
                    TransportErrorKind::custom_str(&format!(
                        "failed to format MPP authorization: {e}"
                    ))
                })?;

                debug!("MPP payment succeeded, retrying WS connection");

                let retry = self.inner.clone().with_auth(Authorization::raw(auth_value));
                retry.connect().await
            }
        }
    }
}

/// Try to extract a [`PaymentChallenge`] from a transport error that wraps a
/// tungstenite HTTP 402 response.
fn extract_402_challenge(err: &alloy_transport::TransportError) -> Option<PaymentChallenge> {
    use std::error::Error;

    // Walk the error source chain to find a tungstenite::Error.
    let mut source: Option<&(dyn Error + 'static)> = Some(err);
    while let Some(current) = source {
        if let Some(tungstenite::Error::Http(response)) =
            current.downcast_ref::<tungstenite::Error>()
        {
            if response.status() == http::StatusCode::PAYMENT_REQUIRED {
                let www_auth =
                    response.headers().get(http::header::WWW_AUTHENTICATE)?.to_str().ok()?;
                return PaymentChallenge::from_header(www_auth).ok();
            }
        }
        source = current.source();
    }
    None
}
