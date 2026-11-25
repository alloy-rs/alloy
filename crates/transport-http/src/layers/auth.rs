use alloy_json_rpc::{RequestPacket, ResponsePacket};
use alloy_rpc_types_engine::{Claims, JwtSecret};
use alloy_transport::{Transport, TransportError, TransportErrorKind, TransportFut};
use http::{
    header::{HeaderValue, AUTHORIZATION},
    HeaderMap, Request, Response,
};
use jsonwebtoken::get_current_timestamp;
use std::{
    future::Future,
    pin::Pin,
    time::{SystemTime, UNIX_EPOCH},
};
use tower::{Layer, Service};

/// The [`AuthLayer`] uses the provided [`JwtSecret`] to generate and validate the jwt token
/// in the requests.
///
/// The generated token is inserted into the [`AUTHORIZATION`] header of the request.
///
/// The layer can wrap either a [`Transport`], such as [`Http`](crate::Http), or a service of
/// [`http::Request`]s, such as the inner service of a `HyperClient`.
#[derive(Clone, Debug)]
pub struct AuthLayer {
    secret: JwtSecret,
    latency_buffer: u64,
}

impl AuthLayer {
    /// Create a new [`AuthLayer`].
    pub const fn new(secret: JwtSecret) -> Self {
        Self { secret, latency_buffer: 5000 }
    }

    /// We use this buffer to perform an extra check on the `iat` field to prevent sending any
    /// requests with tokens that are valid now but may not be upon reaching the server.
    ///
    /// In milliseconds. Default is 5s.
    pub const fn with_latency_buffer(self, latency_buffer: u64) -> Self {
        Self { latency_buffer, ..self }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthService::new(inner, self.secret, self.latency_buffer)
    }
}

/// A service that generates and validates the jwt token in the requests using the provided secret.
#[derive(Clone, Debug)]
pub struct AuthService<S> {
    inner: S,
    secret: JwtSecret,
    /// In milliseconds.
    latency_buffer: u64,
    most_recent_claim: Option<Claims>,
}

impl<S> AuthService<S> {
    /// Create a new [`AuthService`] with the given inner service.
    pub const fn new(inner: S, secret: JwtSecret, latency_buffer: u64) -> Self {
        Self { inner, secret, latency_buffer, most_recent_claim: None }
    }

    /// Validate the token in the request headers.
    ///
    /// Returns `true` if the token is still valid and `iat` is within the grace buffer.
    /// A token is considered valid if the time difference between the current time
    /// and the issued-at time is within the configured latency buffer.
    fn validate(&self) -> bool {
        if let Some(claim) = self.most_recent_claim.as_ref() {
            let curr_secs = get_current_timestamp();
            // Check if the token is not too old (within latency buffer)
            // Convert seconds to milliseconds for comparison with latency_buffer
            if claim.iat.abs_diff(curr_secs) * 1000 <= self.latency_buffer {
                return true;
            }
        }

        false
    }

    /// Create a new token from the secret.
    ///
    /// Updates the most_recent_claim with the new claim.
    /// The issued-at time is set to the current timestamp to ensure proper validation.
    fn create_token_from_secret(&mut self) -> Result<String, jsonwebtoken::errors::Error> {
        let claims = Claims {
            // Set iat to current time (not future time) for proper validation
            iat: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            exp: None,
        };

        self.most_recent_claim = Some(claims);

        let token = self.secret.encode(&claims)?;

        Ok(format!("Bearer {token}"))
    }

    /// Returns the [`AUTHORIZATION`] value for the next request.
    fn authorization(&mut self) -> Result<HeaderValue, TransportError> {
        let token = if self.validate() {
            // Encodes the most recent claim into a token.
            self.secret
                .encode(self.most_recent_claim.as_ref().unwrap())
                .map(|token| format!("Bearer {token}"))
        } else {
            // Creates a new Claim and encodes it into a token.
            self.create_token_from_secret()
        }
        .map_err(TransportErrorKind::custom)?;

        let mut value = HeaderValue::try_from(token).map_err(TransportErrorKind::custom)?;
        value.set_sensitive(true);
        Ok(value)
    }
}

impl<S, B, ResBody> Service<Request<B>> for AuthService<S>
where
    S: Service<Request<B>, Response = Response<ResBody>> + Clone + Send + Sync + 'static,
    S::Future: Send,
    S::Error: std::error::Error + Send + Sync + 'static,
    B: From<Vec<u8>> + Send + 'static + Clone + Sync,
    ResBody: Send + 'static,
{
    type Response = Response<ResBody>;
    type Error = TransportError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Response<ResBody>, Self::Error>> + Send + 'static>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(TransportErrorKind::custom)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let mut req = req;
        match self.authorization() {
            Ok(token) => {
                req.headers_mut().insert(AUTHORIZATION, token);

                let mut this = self.clone();

                Box::pin(
                    async move { this.inner.call(req).await.map_err(TransportErrorKind::custom) },
                )
            }
            Err(e) => Box::pin(async move { Err(e) }),
        }
    }
}

impl<S: Transport> Service<RequestPacket> for AuthService<S> {
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: RequestPacket) -> Self::Future {
        let token = match self.authorization() {
            Ok(token) => token,
            Err(e) => return Box::pin(async move { Err(e) }),
        };

        // A batch is one HTTP request that appends the headers of all its calls, so it must carry
        // the token exactly once.
        if let Some((last, rest)) = req.requests_mut().split_last_mut() {
            for request in rest {
                if let Some(headers) = request.extensions_mut().get_mut::<HeaderMap>() {
                    headers.remove(AUTHORIZATION);
                }
            }
            last.headers_mut().insert(AUTHORIZATION, token);
        }

        self.inner.call(req)
    }
}

#[cfg(all(test, feature = "reqwest"))]
mod tests {
    use super::*;
    use crate::Http;
    use alloy_json_rpc::{Id, Request as RpcRequest, SerializedRequest};
    use alloy_transport::BoxTransport;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    fn request(id: u64) -> SerializedRequest {
        RpcRequest::new("eth_chainId", Id::Number(id), ()).serialize().unwrap()
    }

    #[tokio::test]
    async fn reqwest_batch_sends_one_bearer_token() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap()).parse().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut received = Vec::new();
            let mut buf = [0; 1024];
            // The request body is a JSON array.
            while !received.ends_with(b"]") {
                let n = stream.read(&mut buf).await.unwrap();
                assert_ne!(n, 0, "connection closed before the request ended");
                received.extend_from_slice(&buf[..n]);
            }
            let body = r#"[{"jsonrpc":"2.0","id":1,"result":"0x1"},{"jsonrpc":"2.0","id":2,"result":"0x1"}]"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\
                 connection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            String::from_utf8(received).unwrap()
        });

        let secret = JwtSecret::random();
        let mut stale = request(1);
        stale.headers_mut().insert(AUTHORIZATION, HeaderValue::from_static("Bearer stale"));
        let packet = RequestPacket::Batch(vec![stale, request(2)]);

        let mut transport = BoxTransport::new(AuthLayer::new(secret).layer(Http::new(url)));
        transport.call(packet).await.unwrap();

        let received = server.await.unwrap();
        let values = received
            .lines()
            .filter_map(|line| line.split_once(':'))
            .filter(|(name, _)| name.eq_ignore_ascii_case(AUTHORIZATION.as_str()))
            .map(|(_, value)| value.trim())
            .collect::<Vec<_>>();
        let [value] = values.as_slice() else { panic!("{values:?}") };
        secret.validate(value.strip_prefix("Bearer ").unwrap()).unwrap();
    }
}
