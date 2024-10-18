/// Serializes and Deserializes [`Sealed`], flattening the struct.
///
/// [`Sealed`]: alloy_primitives::Sealed
pub mod flat {
    use alloy_primitives::{Sealed, B256};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize)]
    struct FlatSer<'a, T> {
        seal: B256,
        #[serde(flatten)]
        t: &'a T,
    }

    #[derive(Deserialize)]
    struct FlatDeser<T> {
        seal: B256,
        #[serde(flatten)]
        t: T,
    }

    /// Serializes a [`Sealed`] with the given serializer.
    pub fn serialize<S, T>(sealed: &Sealed<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        FlatSer { seal: sealed.seal(), t: sealed.inner() }.serialize(serializer)
    }

    /// Deserializes a [`Sealed`] with the given deserializer.
    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<Sealed<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        let FlatDeser { seal, t } = FlatDeser::deserialize(deserializer)?;
        Ok(Sealed::new_unchecked(t, seal))
    }
}

/// Serializes and Deserializes [`Sealed`], flattening the struct and renaming
/// the `seal` key to `hash`.
///
/// [`Sealed`]: alloy_primitives::Sealed
pub mod flat_hash {
    use alloy_primitives::{Sealed, B256};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize)]
    struct FlatSer<'a, T> {
        hash: B256,
        #[serde(flatten)]
        t: &'a T,
    }

    #[derive(Deserialize)]
    struct FlatDeser<T> {
        hash: B256,
        #[serde(flatten)]
        t: T,
    }

    /// Serializes a [`Sealed`] with the given serializer.
    pub fn serialize<S, T>(sealed: &Sealed<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize,
    {
        FlatSer { hash: sealed.seal(), t: sealed.inner() }.serialize(serializer)
    }

    /// Deserializes a [`Sealed`] with the given deserializer.
    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<Sealed<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        let FlatDeser { hash, t } = FlatDeser::deserialize(deserializer)?;
        Ok(Sealed::new_unchecked(t, hash))
    }
}
