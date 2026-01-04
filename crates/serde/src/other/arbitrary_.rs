use crate::OtherFields;
use alloc::{collections::BTreeMap, string::String, vec::Vec};

impl arbitrary::Arbitrary<'_> for OtherFields {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let mut inner = BTreeMap::new();
        for _ in 0usize..u.int_in_range(0usize..=15)? {
            inner.insert(
                u.arbitrary()?,
                ArbitraryValue::arbitrary_with_depth(u, 5)?.into_json_value(),
            );
        }
        Ok(Self { inner })
    }
}

/// Redefinition of `serde_json::Value` for the purpose of implementing `Arbitrary`.
///
/// This enum supports generating arbitrary JSON values with depth control to prevent
/// excessive recursion. Supports unsigned integers, signed integers, and floating-point numbers.
#[derive(Clone, Debug)]
enum ArbitraryValue {
    Null,
    Bool(bool),
    Number(NumberVariant),
    String(String),
    Array(Vec<Self>),
    Object(BTreeMap<String, Self>),
}

/// Variants for different JSON number types.
#[derive(Clone, Debug)]
enum NumberVariant {
    /// Unsigned integer (u64)
    U64(u64),
    /// Signed integer (i64)
    I64(i64),
    /// Floating point number (f64)
    F64(f64),
}

impl ArbitraryValue {
    /// Generate an arbitrary value with depth control.
    fn arbitrary_with_depth(
        u: &mut arbitrary::Unstructured<'_>,
        depth: usize,
    ) -> arbitrary::Result<Self> {
        if depth == 0 {
            // At max depth, only generate primitive values
            match u.int_in_range(0..=3)? {
                0 => Ok(Self::Null),
                1 => Ok(Self::Bool(u.arbitrary()?)),
                2 => Ok(Self::Number(NumberVariant::arbitrary(u)?)),
                3 => Ok(Self::String(u.arbitrary()?)),
                _ => unreachable!(),
            }
        } else {
            match u.int_in_range(0..=5)? {
                0 => Ok(Self::Null),
                1 => Ok(Self::Bool(u.arbitrary()?)),
                2 => Ok(Self::Number(NumberVariant::arbitrary(u)?)),
                3 => Ok(Self::String(u.arbitrary()?)),
                4 => {
                    let len = u.int_in_range(0..=5)?;
                    let mut vec = Vec::new();
                    for _ in 0..len {
                        vec.push(Self::arbitrary_with_depth(u, depth - 1)?);
                    }
                    Ok(Self::Array(vec))
                }
                5 => {
                    let len = u.int_in_range(0..=5)?;
                    let mut map = BTreeMap::new();
                    for _ in 0..len {
                        map.insert(u.arbitrary()?, Self::arbitrary_with_depth(u, depth - 1)?);
                    }
                    Ok(Self::Object(map))
                }
                _ => unreachable!(),
            }
        }
    }

    fn into_json_value(self) -> serde_json::Value {
        match self {
            Self::Null => serde_json::Value::Null,
            Self::Bool(b) => serde_json::Value::Bool(b),
            Self::Number(n) => n.into_json_number(),
            Self::String(s) => serde_json::Value::String(s),
            Self::Array(a) => {
                serde_json::Value::Array(a.into_iter().map(Self::into_json_value).collect())
            }
            Self::Object(o) => serde_json::Value::Object(
                o.into_iter().map(|(k, v)| (k, v.into_json_value())).collect(),
            ),
        }
    }
}

impl NumberVariant {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        match u.int_in_range(0..=2)? {
            0 => Ok(Self::U64(u.arbitrary()?)),
            1 => Ok(Self::I64(u.arbitrary()?)),
            2 => Ok(Self::F64(u.arbitrary()?)),
            _ => unreachable!(),
        }
    }

    fn into_json_number(self) -> serde_json::Value {
        match self {
            Self::U64(n) => serde_json::Value::Number(n.into()),
            Self::I64(n) => {
                // serde_json::Number doesn't support i64 directly, convert via f64 for negatives
                if n >= 0 {
                    serde_json::Value::Number((n as u64).into())
                } else {
                    serde_json::Value::Number(
                        serde_json::Number::from_f64(n as f64)
                            .unwrap_or_else(|| serde_json::Number::from(0)),
                    )
                }
            }
            Self::F64(n) => serde_json::Value::Number(
                serde_json::Number::from_f64(n).unwrap_or_else(|| serde_json::Number::from(0)),
            ),
        }
    }
}
