use crate::hyper::{
    header::{HeaderMap, AUTHORIZATION},
    Request, Response,
};
use alloy_rpc_types_engine::{Claims, JwtSecret};
use alloy_transport::{TransportError, TransportErrorKind};
use hyper::header::HeaderValue;
use jsonwebtoken::{decode, get_current_timestamp, DecodingKey, Validation};
use std::{
    future::Future,
    pin::Pin,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tower::{Layer, Service};

/// The [`AuthLayer`] is a that validates whether the bearer token in the request is valid or not.
/// If invalid, it generates a valid token from the provided [`JwtSecret`] and inserts it into the
/// request.
///
/// This layer also inserts the [`AUTHORIZATION`] header into the request with a valid token if its
/// not already in the request.
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

    /// We use this buffer to perfom an extra check on the `iat` field to prevent sending any
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

/// A service that checks the jwt token in a request is valid. If invalid, it refreshes the token
/// using the provided [`JwtSecret`] and inserts it into the request.
#[derive(Clone, Debug)]
pub struct AuthService<S> {
    inner: S,
    secret: JwtSecret,
    /// In milliseconds.
    latency_buffer: u64,
}

impl<S> AuthService<S> {
    /// Create a new [`AuthService`] with the given inner service.
    pub const fn new(inner: S, secret: JwtSecret, latency_buffer: u64) -> Self {
        Self { inner, secret, latency_buffer }
    }

    /// Validate the token in the request headers.
    ///
    /// Returns `true` if the token is valid and `iat` is beyond the grace buffer.
    pub fn validate(&self, headers: &HeaderMap) -> bool {
        get_bearer_token(headers).map_or(false, |token| {
            let is_valid = self.secret.validate(&token).ok().and_then(|_| {
                let validation = Validation::new(jsonwebtoken::Algorithm::HS256);
                let decoding_key = DecodingKey::from_secret(self.secret.as_bytes());
                decode::<Claims>(token.as_str(), &decoding_key, &validation).ok().and_then(|data| {
                    let curr_secs = get_current_timestamp();
                    if data.claims.iat.abs_diff(curr_secs) <= self.latency_buffer {
                        None
                    } else {
                        Some(())
                    }
                })
            });
            is_valid.is_some()
        })
    }
}

impl<S, B, ResBody> Service<Request<B>> for AuthService<S>
where
    S: Service<hyper::Request<B>, Response = Response<ResBody>, Error = TransportError>
        + Clone
        + Send
        + Sync
        + 'static,
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
        Pin<Box<dyn Future<Output = Result<Response<ResBody>, TransportError>> + Send + 'static>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let headers = req.headers();

        match self.validate(headers) {
            true => Box::pin(self.inner.call(req)),
            false => {
                // Generate a fresh token from the secret and insert it into the request.
                match create_token_from_secret(&self.secret) {
                    Ok(token) => {
                        let mut req = req.clone();

                        req.headers_mut()
                            .insert(AUTHORIZATION, HeaderValue::from_str(&token).unwrap());
                        Box::pin(self.inner.call(req))
                    }
                    Err(e) => {
                        let e = TransportErrorKind::custom(e);

                        Box::pin(async move { Err(e) })
                    }
                }
            }
        }
    }
}

fn get_bearer_token(headers: &HeaderMap) -> Option<String> {
    let header = headers.get(AUTHORIZATION)?;
    let auth: &str = header.to_str().ok()?;
    let prefix = "Bearer ";
    let index = auth.find(prefix)?;
    let token: &str = &auth[index + prefix.len()..];
    Some(token.into())
}

fn create_token_from_secret(secret: &JwtSecret) -> Result<String, jsonwebtoken::errors::Error> {
    let token = secret.encode(&Claims {
        iat: (SystemTime::now().duration_since(UNIX_EPOCH).unwrap() + Duration::from_secs(60))
            .as_secs(),
        exp: None,
    })?;

    Ok(format!("Bearer {}", token))
}
