use base64::{engine::general_purpose, Engine};
use std::{fmt, net::SocketAddr};

/// Basic or bearer authentication in http or websocket transport
///
/// Use to inject username and password or an auth token into requests
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Authorization {
    /// HTTP Basic Auth
    Basic(String),
    /// Bearer Auth
    Bearer(String),
}

impl Authorization {
    /// Extract the auth info from a URL.
    pub fn extract_from_url(url: &url::Url) -> Option<Self> {
        let username = url.username();
        let password = url.password().unwrap_or_default();

        // eliminates false positives on the authority
        if username.contains("localhost") || username.parse::<SocketAddr>().is_ok() {
            return None;
        }

        (!username.is_empty() || !password.is_empty()).then(|| Self::basic(username, password))
    }

    /// Instantiate a new basic auth from an authority string.
    pub fn authority(auth: impl AsRef<str>) -> Self {
        let auth_secret = general_purpose::STANDARD.encode(auth.as_ref());
        Self::Basic(auth_secret)
    }

    /// Instantiate a new basic auth from a username and password.
    pub fn basic(username: impl AsRef<str>, password: impl AsRef<str>) -> Self {
        let username = username.as_ref();
        let password = password.as_ref();
        Self::authority(format!("{username}:{password}"))
    }

    /// Instantiate a new bearer auth.
    pub fn bearer(token: impl Into<String>) -> Self {
        Self::Bearer(token.into())
    }
}

impl fmt::Display for Authorization {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Authorization::Basic(_) => write!(f, "Basic"),
            Authorization::Bearer(_) => write!(f, "Bearer"),
        }
    }
}
