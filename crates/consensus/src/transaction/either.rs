/// A type that represents exactly one of two possible variants: `Left(L)` or `Right(R)`.
///
/// `Either` is completely neutral with respect to the meaning of each variants.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Either<L, R> {
    /// The left variant.
    Left(L),
    /// The right variant.
    Right(R),
}

impl<L, R> Either<L, R> {
    /// Returns `true` if this is a `Left` variant.
    #[inline]
    pub fn is_left(&self) -> bool {
        matches!(self, Self::Left(_))
    }

    /// Returns `true` if this is a `Right` variant.
    #[inline]
    pub fn is_right(&self) -> bool {
        matches!(self, Self::Right(_))
    }

    /// Converts from `Either<L, R>` to `Option<L>`.
    ///
    /// Returns `Some` if this is a `Left` variant, otherwise returns `None`.
    #[inline]
    pub fn left(self) -> Option<L> {
        match self {
            Self::Left(l) => Some(l),
            Self::Right(_) => None,
        }
    }

    /// Converts from `Either<L, R>` to `Option<R>`.
    ///
    /// Returns `Some` if this is a `Right` variant, otherwise returns `None`.
    #[inline]
    pub fn right(self) -> Option<R> {
        match self {
            Self::Left(_) => None,
            Self::Right(r) => Some(r),
        }
    }

    /// Returns a reference to the left value if this is a `Left` variant.
    #[inline]
    pub fn as_left(&self) -> Option<&L> {
        match self {
            Self::Left(l) => Some(l),
            Self::Right(_) => None,
        }
    }

    /// Returns a reference to the right value if this is a `Right` variant.
    #[inline]
    pub fn as_right(&self) -> Option<&R> {
        match self {
            Self::Left(_) => None,
            Self::Right(r) => Some(r),
        }
    }

    /// Returns a mutable reference to the left value if this is a `Left` variant.
    #[inline]
    pub fn as_left_mut(&mut self) -> Option<&mut L> {
        match self {
            Self::Left(l) => Some(l),
            Self::Right(_) => None,
        }
    }

    /// Returns a mutable reference to the right value if this is a `Right` variant.
    #[inline]
    pub fn as_right_mut(&mut self) -> Option<&mut R> {
        match self {
            Self::Left(_) => None,
            Self::Right(r) => Some(r),
        }
    }

    /// Returns the contained left value or a default.
    ///
    /// Consumes the `self` value and returns the contained value if left,
    /// or returns the provided default if right.
    #[inline]
    pub fn left_or(self, default: L) -> L {
        match self {
            Self::Left(l) => l,
            Self::Right(_) => default,
        }
    }

    /// Returns the contained right value or a default.
    ///
    /// Consumes the `self` value and returns the contained value if right,
    /// or returns the provided default if left.
    #[inline]
    pub fn right_or(self, default: R) -> R {
        match self {
            Self::Left(_) => default,
            Self::Right(r) => r,
        }
    }

    /// Maps an `Either<L, R>` to `Either<T, R>` by applying a function
    /// to the left value.
    #[inline]
    pub fn map_left<T, F>(self, f: F) -> Either<T, R>
    where
        F: FnOnce(L) -> T,
    {
        match self {
            Self::Left(l) => Either::Left(f(l)),
            Self::Right(r) => Either::Right(r),
        }
    }

    /// Maps an `Either<L, R>` to `Either<L, T>` by applying
    /// a function to the right value.
    #[inline]
    pub fn map_right<T, F>(self, f: F) -> Either<L, T>
    where
        F: FnOnce(R) -> T,
    {
        match self {
            Self::Left(l) => Either::Left(l),
            Self::Right(r) => Either::Right(f(r)),
        }
    }

    /// Maps an `Either<L, R>` to `Either<T, U>` by applying one of two functions.
    #[inline]
    pub fn map_either<T, U, F, G>(self, f: F, g: G) -> Either<T, U>
    where
        F: FnOnce(L) -> T,
        G: FnOnce(R) -> U,
    {
        match self {
            Self::Left(l) => Either::Left(f(l)),
            Self::Right(r) => Either::Right(g(r)),
        }
    }

    /// Applies a function to the contained value, and returns the result for
    /// both variants.
    #[inline]
    pub fn either<T, F, G>(self, f: F, g: G) -> T
    where
        F: FnOnce(L) -> T,
        G: FnOnce(R) -> T,
    {
        match self {
            Self::Left(l) => f(l),
            Self::Right(r) => g(r),
        }
    }
}

impl<T> Either<T, T> {
    /// Converts from `Either<T, T>` to `T`.
    #[inline]
    pub fn into_inner(self) -> T {
        match self {
            Self::Left(t) => t,
            Self::Right(t) => t,
        }
    }
}

impl<L, R> From<Either<L, R>> for Result<L, R> {
    fn from(either: Either<L, R>) -> Self {
        match either {
            Either::Left(l) => Ok(l),
            Either::Right(r) => Err(r),
        }
    }
}

#[cfg(test)]
mod test_either {
    use super::*;

    #[test]
    fn test_either_variants() {
        let left: Either<i32, &str> = Either::Left(42);
        let right: Either<i32, &str> = Either::Right("hello");

        assert!(!left.is_right());
        assert!(!right.is_left());
    }

    #[test]
    fn test_either_mapping() {
        let left: Either<i32, &str> = Either::Left(42);
        let mapped = left.map_left(|x| x.to_string());
        assert_eq!(mapped.left(), Some("42".to_string()));

        let right: Either<i32, &str> = Either::Right("hello");
        let mapped = right.map_right(|x| x.len());
        assert_eq!(mapped.right(), Some(5));
    }

    #[test]
    fn test_either_same_types() {
        let left: Either<i32, i32> = Either::Left(42);
        assert_eq!(left.into_inner(), 42);

        let right: Either<i32, i32> = Either::Right(24);
        assert_eq!(right.into_inner(), 24);
    }
}
