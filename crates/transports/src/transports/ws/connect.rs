use tokio_tungstenite::tungstenite::{self, client::IntoClientRequest};

use crate::{pubsub::PubSubConnect, transports::ws::backend::WsBackend, TransportError};

#[derive(Debug, Clone)]
pub struct WsConnect {
    pub url: String,
    pub auth: Option<crate::Authorization>,
}

impl IntoClientRequest for WsConnect {
    fn into_client_request(self) -> tungstenite::Result<tungstenite::handshake::client::Request> {
        let mut request: http::Request<()> = self.url.into_client_request()?;
        if let Some(auth) = self.auth {
            let mut auth_value = http::HeaderValue::from_str(&auth.to_string())?;
            auth_value.set_sensitive(true);

            request
                .headers_mut()
                .insert(http::header::AUTHORIZATION, auth_value);
        }

        request.into_client_request()
    }
}

impl PubSubConnect for WsConnect {
    type Error = TransportError;

    fn connect<'a: 'b, 'b>(
        &'a self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<crate::pubsub::ConnectionHandle, Self::Error>>
                + Send
                + 'b,
        >,
    > {
        let request = self.clone().into_client_request();

        Box::pin(async move {
            let (socket, _) = tokio_tungstenite::connect_async(request?).await?;

            let (handle, interface) = crate::pubsub::ConnectionHandle::new();
            let backend = WsBackend { socket, interface };

            backend.spawn();

            Ok(handle)
        })
    }
}
