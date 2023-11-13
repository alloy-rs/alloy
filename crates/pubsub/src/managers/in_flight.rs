use alloy_json_rpc::{Response, ResponsePayload, SerializedRequest};
use alloy_primitives::U256;
use alloy_transport::TransportError;
use tokio::sync::oneshot;

/// An in-flight JSON-RPC request.
///
/// This struct contains the request that was sent, as well as a channel to
/// receive the response on.
pub(crate) struct InFlight {
    /// The request
    pub(crate) request: SerializedRequest,

    /// The channel to send the response on.
    pub(crate) tx: oneshot::Sender<Result<Response, TransportError>>,
}

impl std::fmt::Debug for InFlight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let channel_desc =
            format!("Channel status: {}", if self.tx.is_closed() { "closed" } else { "ok" });

        f.debug_struct("InFlight").field("req", &self.request).field("tx", &channel_desc).finish()
    }
}

impl InFlight {
    /// Create a new in-flight request.
    pub(crate) fn new(
        request: SerializedRequest,
    ) -> (Self, oneshot::Receiver<Result<Response, TransportError>>) {
        let (tx, rx) = oneshot::channel();

        (Self { request, tx }, rx)
    }

    /// Get the method
    pub(crate) const fn method(&self) -> &'static str {
        self.request.method()
    }

    /// Get a reference to the serialized request.
    ///
    /// This is used to (re-)send the request over the transport.
    pub(crate) const fn request(&self) -> &SerializedRequest {
        &self.request
    }

    /// Fulfill the request with a response. This consumes the in-flight
    /// request. If the request is a subscription and the response is not an
    /// error, the subscription ID and the in-flight request are returned.
    pub(crate) fn fulfill(self, resp: Response) -> Option<(U256, Self)> {
        if self.method() == "eth_subscribe" {
            if let ResponsePayload::Success(val) = resp.payload {
                let sub_id: serde_json::Result<U256> = serde_json::from_str(val.get());
                match sub_id {
                    Ok(alias) => return Some((alias, self)),
                    Err(e) => {
                        let _ = self.tx.send(Err(TransportError::deser_err(e, val.get())));
                        return None;
                    }
                }
            }
        }

        let _ = self.tx.send(Ok(resp));
        None
    }
}
