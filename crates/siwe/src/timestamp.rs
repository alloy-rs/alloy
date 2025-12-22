//! RFC 3339 timestamp handling.

use alloc::string::String;
use core::{cmp::Ordering, str::FromStr};
use derive_more::{AsRef, Display};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

/// Wrapper for [`OffsetDateTime`] that preserves the original string representation.
///
/// This ensures that parsing and displaying a timestamp yields the exact same string,
/// which is important for signature verification.
#[derive(Clone, Debug, PartialEq, Eq, Display, AsRef)]
#[display("{raw}")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "String", into = "String"))]
pub struct TimeStamp {
    /// Original string representation (for Display/signature verification).
    raw: String,
    /// Parsed datetime (for time comparisons).
    #[as_ref]
    parsed: OffsetDateTime,
}

impl TimeStamp {
    /// Returns the underlying [`OffsetDateTime`].
    #[must_use]
    pub const fn as_datetime(&self) -> &OffsetDateTime {
        &self.parsed
    }

    /// Returns the original string representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.raw
    }
}

impl From<OffsetDateTime> for TimeStamp {
    fn from(dt: OffsetDateTime) -> Self {
        Self {
            raw: dt.format(&Rfc3339).expect("Rfc3339 formatting works"),
            parsed: dt,
        }
    }
}

impl From<TimeStamp> for String {
    fn from(ts: TimeStamp) -> Self {
        ts.raw
    }
}

impl TryFrom<String> for TimeStamp {
    type Error = time::error::Parse;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Ok(Self {
            parsed: OffsetDateTime::parse(&s, &Rfc3339)?,
            raw: s,
        })
    }
}

impl FromStr for TimeStamp {
    type Err = time::error::Parse;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            raw: s.into(),
            parsed: OffsetDateTime::parse(s, &Rfc3339)?,
        })
    }
}

impl PartialEq<OffsetDateTime> for TimeStamp {
    fn eq(&self, other: &OffsetDateTime) -> bool {
        self.parsed == *other
    }
}

impl PartialOrd<OffsetDateTime> for TimeStamp {
    fn partial_cmp(&self, other: &OffsetDateTime) -> Option<Ordering> {
        self.parsed.partial_cmp(other)
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
