use crate::fillers::TxFiller;

/// Empty filler stack state
#[derive(Debug, Clone)]
pub struct Empty;

/// A stack of transaction fillers
#[derive(Debug)]
pub struct FillerStack<T> {
    _pd: std::marker::PhantomData<T>,
}

/// A trait for tuples that can have types pushed to them
pub trait TuplePush<T> {
    /// The resulting type after pushing T
    type Pushed;
}

// Implement TuplePush for Empty
impl<T: TxFiller> TuplePush<T> for Empty {
    type Pushed = (T,);
}

// Implement base FillerStack methods
impl FillerStack<Empty> {
    /// Create a new empty filler stack
    pub fn new() -> Self {
        Self { _pd: std::marker::PhantomData }
    }
}

// Implement methods for all FillerStack variants
impl<T> FillerStack<T> {
    /// Push a new filler onto the stack
    pub fn push<F: TxFiller>(self, filler: F) -> FillerStack<T::Pushed>
    where
        T: TuplePush<F>,
    {
        FillerStack { _pd: std::marker::PhantomData }
    }
}

// Macro to implement for tuples of different sizes
macro_rules! impl_tuple {
    ($($idx:tt => $ty:ident),+) => {
        // Implement pushing a new type onto the tuple
        impl<T: TxFiller, $($ty: TxFiller,)+> TuplePush<T> for ($($ty,)+) {
            type Pushed = ($($ty,)+ T,);
        }
    };
}

// Implement for tuples up to 3 elements (can be extended if needed)
impl_tuple!(0 => T1);
impl_tuple!(0 => T1, 1 => T2);
impl_tuple!(0 => T1, 1 => T2, 2 => T3);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fillers::{ChainIdFiller, GasFiller, NonceFiller};

    #[test]
    fn test_filler_stack() {
        let stack = FillerStack::new()
            .push(GasFiller)
            .push(NonceFiller::default())
            .push(ChainIdFiller::default());

        // Type should be FillerStack<(GasFiller, NonceFiller, ChainIdFiller)>
        let _: FillerStack<(GasFiller, NonceFiller, ChainIdFiller)> = stack;
    }
}
