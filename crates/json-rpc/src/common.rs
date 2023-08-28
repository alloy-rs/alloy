use serde::{Deserialize, Serialize};

/// A JSON-RPC 2.0 ID object. This may be a number, a string, or null.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum Id {
    Number(u64),
    String(String),
    None,
}

impl Id {
    /// Returns `true` if the ID is a number.
    pub fn is_number(&self) -> bool {
        matches!(self, Id::Number(_))
    }

    /// Returns `true` if the ID is a string.
    pub fn is_string(&self) -> bool {
        matches!(self, Id::String(_))
    }

    /// Returns `true` if the ID is `None`.
    pub fn is_none(&self) -> bool {
        matches!(self, Id::None)
    }

    /// Returns the ID as a number, if it is one.
    pub fn as_number(&self) -> Option<u64> {
        match self {
            Id::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Returns the ID as a string, if it is one.
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Id::String(s) => Some(s),
            _ => None,
        }
    }
}
