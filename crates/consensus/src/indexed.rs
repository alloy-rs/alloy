use alloc::borrow::Cow;

/// A value tagged with its index in a containing sequence.
///
/// This is useful when an item must be processed independently while retaining its original
/// position, for example transactions or receipts within a block.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    derive_more::Deref,
    derive_more::DerefMut,
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "borsh", derive(borsh::BorshSerialize, borsh::BorshDeserialize))]
#[doc(alias = "IndexedReceipt", alias = "IndexedTransaction")]
pub struct Indexed<T> {
    /// The index of the value in its containing sequence.
    pub index: usize,
    /// The indexed value.
    #[deref]
    #[deref_mut]
    pub value: T,
}

impl<T> Indexed<T> {
    /// Creates a new indexed value.
    #[inline]
    pub const fn new(index: usize, value: T) -> Self {
        Self { index, value }
    }

    /// Returns the index of the value.
    #[inline]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns a reference to the indexed value.
    #[inline]
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// Returns a mutable reference to the indexed value.
    #[inline]
    pub const fn value_mut(&mut self) -> &mut T {
        &mut self.value
    }

    /// Returns the indexed value, consuming `self`.
    #[inline]
    pub fn into_value(self) -> T {
        self.value
    }

    /// Splits the indexed value into its components.
    #[inline]
    #[doc(alias = "split")]
    pub fn into_parts(self) -> (usize, T) {
        (self.index, self.value)
    }

    /// Converts from `&Indexed<T>` to `Indexed<&T>`.
    #[inline]
    pub const fn as_ref(&self) -> Indexed<&T> {
        Indexed { index: self.index, value: &self.value }
    }

    /// Converts from `&mut Indexed<T>` to `Indexed<&mut T>`.
    #[inline]
    pub const fn as_mut(&mut self) -> Indexed<&mut T> {
        Indexed { index: self.index, value: &mut self.value }
    }

    /// Converts the indexed value to the given alternative that is `From<T>`.
    #[inline]
    pub fn convert<U>(self) -> Indexed<U>
    where
        U: From<T>,
    {
        self.map(U::from)
    }

    /// Converts the indexed value to the given alternative that is `TryFrom<T>`.
    #[inline]
    pub fn try_convert<U>(self) -> Result<Indexed<U>, U::Error>
    where
        U: TryFrom<T>,
    {
        self.try_map(U::try_from)
    }

    /// Applies the given closure to the indexed value.
    #[inline]
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Indexed<U> {
        Indexed { index: self.index, value: f(self.value) }
    }

    /// Applies the given fallible closure to the indexed value.
    #[inline]
    pub fn try_map<U, E>(self, f: impl FnOnce(T) -> Result<U, E>) -> Result<Indexed<U>, E> {
        Ok(Indexed { index: self.index, value: f(self.value)? })
    }
}

impl<T> Indexed<&T> {
    /// Maps an `Indexed<&T>` to an `Indexed<T>` by copying the indexed value.
    #[inline]
    pub fn copied(self) -> Indexed<T>
    where
        T: Copy,
    {
        self.map(|value| *value)
    }

    /// Maps an `Indexed<&T>` to an `Indexed<T>` by cloning the indexed value.
    #[inline]
    pub fn cloned(self) -> Indexed<T>
    where
        T: Clone,
    {
        self.map(T::clone)
    }
}

impl<T> Indexed<&mut T> {
    /// Maps an `Indexed<&mut T>` to an `Indexed<T>` by copying the indexed value.
    #[inline]
    pub fn copied(self) -> Indexed<T>
    where
        T: Copy,
    {
        self.map(|value| *value)
    }

    /// Maps an `Indexed<&mut T>` to an `Indexed<T>` by cloning the indexed value.
    #[inline]
    pub fn cloned(self) -> Indexed<T>
    where
        T: Clone,
    {
        self.map(|value| value.clone())
    }
}

impl<T> Indexed<Cow<'_, T>>
where
    T: Clone,
{
    /// Converts an indexed [`Cow`] into an indexed owned value by cloning if necessary.
    #[inline]
    pub fn into_owned(self) -> Indexed<T> {
        self.map(Cow::into_owned)
    }
}

impl<T> From<(usize, T)> for Indexed<T> {
    #[inline]
    fn from((index, value): (usize, T)) -> Self {
        Self::new(index, value)
    }
}

impl<T> From<Indexed<T>> for (usize, T) {
    #[inline]
    fn from(indexed: Indexed<T>) -> Self {
        indexed.into_parts()
    }
}

#[cfg(test)]
mod tests {
    use super::Indexed;

    #[test]
    fn constructs_and_splits_indexed_value() {
        let indexed = Indexed::new(3, "receipt");

        assert_eq!(indexed.index(), 3);
        assert_eq!(indexed.value(), &"receipt");
        assert_eq!(indexed.into_parts(), (3, "receipt"));
    }

    #[test]
    fn maps_value_and_preserves_index() {
        let indexed = Indexed::new(1, 41);

        assert_eq!(indexed.map(|value| value + 1), Indexed::new(1, 42));
    }

    #[test]
    fn try_maps_value_and_preserves_index() {
        let indexed = Indexed::new(2, "42");

        let mapped: Result<Indexed<u64>, _> = indexed.try_map(str::parse);

        assert_eq!(mapped.unwrap(), Indexed::new(2, 42));
    }

    #[test]
    fn converts_to_and_from_enumerated_tuple() {
        let indexed = Indexed::from((5, "tx"));
        let tuple: (usize, &str) = indexed.into();

        assert_eq!(tuple, (5, "tx"));
    }

    #[test]
    fn creates_indexed_references() {
        let mut indexed = Indexed::new(7, 1u64);

        assert_eq!(indexed.as_ref().copied(), Indexed::new(7, 1));

        *indexed.as_mut().value += 1;

        assert_eq!(indexed, Indexed::new(7, 2));
    }
}
