use base64::{engine::general_purpose, Engine};
use std::fmt;

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
        if url.has_authority() {
            let username = url.username();
            let pass = url.password().unwrap_or_default();
            Some(Authorization::basic(username, pass))
        } else {
            None
        }
    }

    /// Instantiate a new basic auth.
    pub fn basic(username: impl AsRef<str>, password: impl AsRef<str>) -> Self {
        let username = username.as_ref();
        let password = password.as_ref();
        let auth_secret = general_purpose::STANDARD.encode(format!("{username}:{password}"));
        Self::Basic(auth_secret)
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
