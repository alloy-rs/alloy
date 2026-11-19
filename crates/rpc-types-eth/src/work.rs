use alloy_primitives::B256;

/// The result of an `eth_getWork` request
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Work {
    /// The proof-of-work hash.
    pub pow_hash: B256,
    /// The seed hash.
    pub seed_hash: B256,
    /// The target.
    pub target: B256,
    /// The block number: this isn't always stored.
    pub number: Option<u64>,
}

#[cfg(feature = "serde")]
impl serde::Serialize for Work {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.number.map(alloy_primitives::U64::from) {
            Some(num) => (&self.pow_hash, &self.seed_hash, &self.target, num).serialize(s),
            None => (&self.pow_hash, &self.seed_hash, &self.target).serialize(s),
        }
    }
}

#[cfg(feature = "serde")]
impl<'a> serde::Deserialize<'a> for Work {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        struct WorkVisitor;

        impl<'a> serde::de::Visitor<'a> for WorkVisitor {
            type Value = Work;

            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(formatter, "Work object")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'a>,
            {
                use serde::de::Error;
                let pow_hash = seq
                    .next_element::<B256>()?
                    .ok_or_else(|| A::Error::custom("missing pow hash"))?;
                let seed_hash = seq
                    .next_element::<B256>()?
                    .ok_or_else(|| A::Error::custom("missing seed hash"))?;
                let target = seq
                    .next_element::<B256>()?
                    .ok_or_else(|| A::Error::custom("missing target"))?;
                let number = seq.next_element::<alloy_primitives::U64>()?.map(|n| n.to());
                Ok(Work { pow_hash, seed_hash, target, number })
            }
        }

        deserializer.deserialize_any(WorkVisitor)
    }
}

#[cfg(all(test, feature = "serde"))]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    const fn work(number: Option<u64>) -> Work {
        Work {
            pow_hash: B256::repeat_byte(1),
            seed_hash: B256::repeat_byte(2),
            target: B256::repeat_byte(3),
            number,
        }
    }

    fn parse(number: Option<Value>) -> Work {
        let w = work(None);
        let mut elements = vec![json!(w.pow_hash), json!(w.seed_hash), json!(w.target)];
        elements.extend(number);
        serde_json::from_value(Value::Array(elements)).unwrap()
    }

    #[test]
    fn serde_roundtrip() {
        for work in [work(None), work(Some(0x1234))] {
            let json = serde_json::to_string(&work).unwrap();
            assert_eq!(serde_json::from_str::<Work>(&json).unwrap(), work);
        }
    }

    #[test]
    fn deserialize_number_forms() {
        assert_eq!(parse(None), work(None));
        assert_eq!(parse(Some(json!("0x1234"))), work(Some(0x1234)));
        assert_eq!(parse(Some(json!(0x1234))), work(Some(0x1234)));
    }
}
