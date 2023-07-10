use base64::{engine::general_purpose, Engine};
use serde_json::value::RawValue;
use std::{borrow::Cow, fmt, future::Future, pin::Pin};

pub use jsonrpsee_types::{ErrorObject, ErrorResponse, Id, RequestSer as Request, Response};

use crate::TransportError;

#[cfg(target_arch = "wasm32")]
pub(crate) type DynFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) type DynFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub type JsonRpcResult<'a> = Result<Cow<'a, RawValue>, ErrorObject<'a>>;
pub type JsonRpcResultOwned = JsonRpcResult<'static>;

pub type RpcOutcome = Result<JsonRpcResultOwned, TransportError>;
pub type BatchRpcOutcome = Result<Vec<JsonRpcResultOwned>, TransportError>;

pub type RpcFuture = DynFuture<'static, RpcOutcome>;
pub type BatchRpcFuture = DynFuture<'static, BatchRpcOutcome>;

/// Basic or bearer authentication in http or websocket transport
///
/// Use to inject username and password or an auth token into requests
#[derive(Clone, Debug)]
pub enum Authorization {
    /// HTTP Basic Auth
    Basic(String),
    /// Bearer Auth
    Bearer(String),
}

impl Authorization {
    /// Make a new basic auth
    pub fn basic(username: impl AsRef<str>, password: impl AsRef<str>) -> Self {
        let username = username.as_ref();
        let password = password.as_ref();
        let auth_secret = general_purpose::STANDARD.encode(format!("{username}:{password}"));
        Self::Basic(auth_secret)
    }

    /// Make a new bearer auth
    pub fn bearer(token: impl Into<String>) -> Self {
        Self::Bearer(token.into())
    }
}

impl fmt::Display for Authorization {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Authorization::Basic(auth_secret) => write!(f, "Basic {auth_secret}"),
            Authorization::Bearer(token) => write!(f, "Bearer {token}"),
        }
    }
}
