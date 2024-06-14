use crate::OtherFields;
use alloc::collections::BTreeMap;
use proptest::{
    arbitrary::any,
    strategy::{BoxedStrategy, Strategy},
};

impl arbitrary::Arbitrary<'_> for OtherFields {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let mut inner = BTreeMap::new();
        for _ in 0usize..u.int_in_range(0usize..=10)? {
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
        proptest::collection::vec(any::<(String, ArbitraryValue)>(), 0..=10)
            .prop_map(|map| map.into_iter().map(|(k, v)| (k, v.into_json_value())).collect())
    }
}

/// Redefinition of `serde_json::Value` for the purpose of implementing `Arbitrary`.
#[derive(Debug, arbitrary::Arbitrary)]
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
        todo!()
    }
}

impl ArbitraryValue {
    fn into_json_value(self) -> serde_json::Value {
        match self {
            Self::Null => serde_json::Value::Null,
            Self::Bool(b) => serde_json::Value::Bool(b),
            Self::Number(n) => serde_json::Value::Number(n.into()),
            Self::String(s) => serde_json::Value::String(s),
            Self::Array(a) => serde_json::Value::Array(
                a.into_iter().map(ArbitraryValue::into_json_value).collect(),
            ),
            Self::Object(o) => serde_json::Value::Object(
                o.into_iter().map(|(k, v)| (k, v.into_json_value())).collect(),
            ),
        }
    }
}
