//! RFC 3339 timestamp handling.

use core::{
    cmp::Ordering,
    fmt::{self, Display, Formatter},
    str::FromStr,
};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

/// Wrapper for [`OffsetDateTime`] that preserves the original string representation.
///
/// This ensures that parsing and displaying a timestamp yields the exact same string,
/// which is important for signature verification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeStamp(String, OffsetDateTime);

impl TimeStamp {
    /// Returns the underlying [`OffsetDateTime`].
    #[must_use]
    pub const fn as_datetime(&self) -> &OffsetDateTime {
        &self.1
    }

    /// Returns the original string representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<OffsetDateTime> for TimeStamp {
    fn from(t: OffsetDateTime) -> Self {
        Self(t.format(&Rfc3339).expect("Rfc3339 formatting works"), t)
    }
}

impl AsRef<OffsetDateTime> for TimeStamp {
    fn as_ref(&self) -> &OffsetDateTime {
        &self.1
    }
}

impl Display for TimeStamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.0)
    }
}

impl FromStr for TimeStamp {
    type Err = time::error::Parse;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.into(), OffsetDateTime::parse(s, &Rfc3339)?))
    }
}

impl PartialEq<OffsetDateTime> for TimeStamp {
    fn eq(&self, other: &OffsetDateTime) -> bool {
        &self.1 == other
    }
}

impl PartialOrd<OffsetDateTime> for TimeStamp {
    fn partial_cmp(&self, other: &OffsetDateTime) -> Option<Ordering> {
        self.1.partial_cmp(other)
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use super::*;
    use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

    impl Serialize for TimeStamp {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(&self.0)
        }
    }

    impl<'de> Deserialize<'de> for TimeStamp {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let s = String::deserialize(deserializer)?;
            s.parse().map_err(de::Error::custom)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timestamp() {
        let ts: TimeStamp = "2021-12-07T18:28:18.807Z".parse().unwrap();
        assert_eq!(ts.as_str(), "2021-12-07T18:28:18.807Z");
    }

    #[test]
    fn test_roundtrip() {
        let original = "2021-12-07T18:28:18.807Z";
        let ts: TimeStamp = original.parse().unwrap();
        assert_eq!(ts.to_string(), original);
    }
}
