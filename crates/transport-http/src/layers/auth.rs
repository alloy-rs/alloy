use alloy_json_rpc::RequestPacket;
use alloy_rpc_types_engine::{Claims, JwtSecret};
use alloy_transport::{TransportError, TransportErrorKind};
use http::header::{HeaderValue, AUTHORIZATION};
#[cfg(feature = "hyper")]
use http::{Request, Response};
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
/// The layer can wrap a service of [`RequestPacket`]s, such as [`Http`](crate::Http) with a
/// `reqwest` client, or, with the `hyper` feature, a service of [`http::Request`]s such as
/// [`HyperClient`](crate::HyperClient).
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

    /// Returns the [`AUTHORIZATION`] header value for the next request, reusing the most recent
    /// claim while it is still within the latency buffer.
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

        Ok(HeaderValue::from_str(&token).unwrap())
    }
}

/// Sets the token on the packet's HTTP headers, which the HTTP transports send with the request.
///
/// A batch is sent as a single HTTP request with the headers of all its requests combined, so the
/// token is set on the last request and any other [`AUTHORIZATION`] header in the packet is
/// removed.
impl<S> Service<RequestPacket> for AuthService<S>
where
    S: Service<RequestPacket, Error = TransportError>,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = TransportError;
    type Future = Pin<Box<dyn Future<Output = Result<S::Response, TransportError>> + Send>>;

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

        let requests = req.requests_mut();
        for request in requests.iter_mut() {
            if let Some(headers) = request.extensions_mut().get_mut::<http::HeaderMap>() {
                headers.remove(AUTHORIZATION);
            }
        }
        if let Some(last) = requests.last_mut() {
            last.headers_mut().insert(AUTHORIZATION, token);
        }

        Box::pin(self.inner.call(req))
    }
}

#[cfg(feature = "hyper")]
impl<S, B, ResBody> Service<Request<B>> for AuthService<S>
where
    S: Service<hyper::Request<B>, Response = Response<ResBody>> + Clone + Send + Sync + 'static,
    S::Future: Send,
    S::Error: std::error::Error + Send + Sync + 'static,
    B: From<Vec<u8>> + Send + 'static + Clone + Sync,
    ResBody: hyper::body::Body + Send + 'static,
    ResBody::Error: std::error::Error + Send + Sync + 'static,
    ResBody::Data: Send,
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
