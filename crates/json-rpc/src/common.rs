use serde::{Deserialize, Serialize};

/// A JSON-RPC 2.0 ID object. This may be a number, a string, or null.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum Id {
    Number(u64),
    String(String),
    None,
}

impl PartialOrd for Id {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // numbers < strings
        // strings < null
        // null == null
        match (self, other) {
            (Id::Number(a), Id::Number(b)) => a.partial_cmp(b),
            (Id::Number(_), _) => Some(std::cmp::Ordering::Less),

            (Id::String(_), Id::Number(_)) => Some(std::cmp::Ordering::Greater),
            (Id::String(a), Id::String(b)) => a.partial_cmp(b),
            (Id::String(_), Id::None) => Some(std::cmp::Ordering::Less),

            (Id::None, Id::None) => Some(std::cmp::Ordering::Equal),
            (Id::None, _) => Some(std::cmp::Ordering::Greater),
        }
    }
}
impl Ord for Id {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
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
