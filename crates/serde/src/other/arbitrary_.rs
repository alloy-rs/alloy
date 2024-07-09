use crate::OtherFields;
use alloc::collections::BTreeMap;
use proptest::{
    arbitrary::any,
    prop_oneof,
    strategy::{BoxedStrategy, Just, Strategy},
};

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

impl arbitrary::Arbitrary<'_> for OtherFields {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let mut inner = BTreeMap::new();
        for _ in 0usize..u.int_in_range(0usize..=15)? {
            inner.insert(u.arbitrary()?, u.arbitrary::<ArbitraryValue>()?.into_json_value());
        }
        Ok(Self { inner })
    }
}

impl proptest::arbitrary::Arbitrary for OtherFields {
    type Parameters = ();
    type Strategy = proptest::strategy::Map<
        proptest::collection::VecStrategy<(
            <String as proptest::arbitrary::Arbitrary>::Strategy,
            <ArbitraryValue as proptest::arbitrary::Arbitrary>::Strategy,
        )>,
        fn(Vec<(String, ArbitraryValue)>) -> Self,
    >;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        proptest::collection::vec(any::<(String, ArbitraryValue)>(), 0..16)
            .prop_map(|map| map.into_iter().map(|(k, v)| (k, v.into_json_value())).collect())
    }
}

/// Redefinition of `serde_json::Value` for the purpose of implementing `Arbitrary`.
#[derive(Clone, Debug, arbitrary::Arbitrary)]
#[allow(unnameable_types)]
pub enum ArbitraryValue {
    Null,
    Bool(bool),
    Number(u64),
    String(String),
    Array(Vec<ArbitraryValue>),
    Object(BTreeMap<String, ArbitraryValue>),
}

impl proptest::arbitrary::Arbitrary for ArbitraryValue {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        prop_oneof![
            Just(Self::Null),
            any::<bool>().prop_map(Self::Bool),
            any::<u64>().prop_map(Self::Number),
            any::<String>().prop_map(Self::String),
        ]
        .prop_recursive(4, 64, 16, |this| {
            prop_oneof![
                1 => proptest::collection::vec(this.clone(), 0..16).prop_map(Self::Array),
                1 => proptest::collection::btree_map(any::<String>(), this, 0..16).prop_map(Self::Object),
            ]
        })
        .boxed()
    }
}

impl ArbitraryValue {
    fn into_json_value(self) -> serde_json::Value {
        match self {
            Self::Null => serde_json::Value::Null,
            Self::Bool(b) => serde_json::Value::Bool(b),
            Self::Number(n) => serde_json::Value::Number(n.into()),
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

#[cfg(test)]
mod tests {
    use super::*;

    proptest::proptest!(
        #[test]
        fn test_arbitrary_value(value in any::<ArbitraryValue>()) {
            let _json_value = value.into_json_value();
        }
    );
}
