use serde::{de::Visitor, Deserialize, Serialize};
use std::fmt::Display;

/// A JSON-RPC 2.0 ID object. This may be a number, a string, or null.
///
/// ### Ordering
///
/// This type implements [`PartialOrd`], [`Ord`], [`PartialEq`], and [`Eq`] so
/// that it can be used as a key in a [`BTreeMap`] or an item in a
/// [`BTreeSet`]. The ordering is as follows:
///
/// 1. Numbers are less than strings.
/// 2. Strings are less than null.
/// 3. Null is equal to null.
///
/// ### Hash
///
/// This type implements [`Hash`] so that it can be used as a key in a
/// [`HashMap`] or an item in a [`HashSet`].
///
/// [`BTreeMap`]: std::collections::BTreeMap
/// [`BTreeSet`]: std::collections::BTreeSet
/// [`HashMap`]: std::collections::HashMap
/// [`HashSet`]: std::collections::HashSet
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Id {
    /// A number.
    Number(u64),
    /// A string.
    String(String),
    /// Null.
    None,
}

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Id::Number(n) => write!(f, "{n}"),
            Id::String(s) => f.write_str(s),
            Id::None => f.write_str("null"),
        }
    }
}

impl Serialize for Id {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Id::Number(n) => serializer.serialize_u64(*n),
            Id::String(s) => serializer.serialize_str(s),
            Id::None => serializer.serialize_none(),
        }
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct IdVisitor;

        impl<'de> Visitor<'de> for IdVisitor {
            type Value = Id;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "a string, a number, or null")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Id::String(v.to_owned()))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Id::Number(v))
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Id::None)
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Id::None)
            }
        }

        deserializer.deserialize_any(IdVisitor)
    }
}

impl PartialOrd for Id {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Id {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // numbers < strings
        // strings < null
        // null == null
        match (self, other) {
            (Id::Number(a), Id::Number(b)) => a.cmp(b),
            (Id::Number(_), _) => std::cmp::Ordering::Less,

            (Id::String(_), Id::Number(_)) => std::cmp::Ordering::Greater,
            (Id::String(a), Id::String(b)) => a.cmp(b),
            (Id::String(_), Id::None) => std::cmp::Ordering::Less,

            (Id::None, Id::None) => std::cmp::Ordering::Equal,
            (Id::None, _) => std::cmp::Ordering::Greater,
        }
    }
}

impl Id {
    /// Returns `true` if the ID is a number.
    pub const fn is_number(&self) -> bool {
        matches!(self, Id::Number(_))
    }

    /// Returns `true` if the ID is a string.
    pub const fn is_string(&self) -> bool {
        matches!(self, Id::String(_))
    }

    /// Returns `true` if the ID is `None`.
    pub const fn is_none(&self) -> bool {
        matches!(self, Id::None)
    }

    /// Returns the ID as a number, if it is one.
    pub const fn as_number(&self) -> Option<u64> {
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

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestCase {
        id: Id,
    }

    #[test]
    fn it_serializes_and_deserializes() {
        let cases = [
            (TestCase { id: Id::Number(1) }, r#"{"id":1}"#),
            (TestCase { id: Id::String("foo".to_string()) }, r#"{"id":"foo"}"#),
            (TestCase { id: Id::None }, r#"{"id":null}"#),
        ];
        for (case, expected) in cases {
            let serialized = serde_json::to_string(&case).unwrap();
            assert_eq!(serialized, expected);

            let deserialized: TestCase = serde_json::from_str(expected).unwrap();
            assert_eq!(deserialized, case);
        }
    }
}
