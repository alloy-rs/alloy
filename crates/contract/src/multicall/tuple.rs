use crate::{Error, MulticallError, Result};
use alloy_primitives::Bytes;
use alloy_sol_types::SolCall;

/// Sealed trait to prevent external implementations
mod private {
    pub trait Sealed {}
}
use private::Sealed;

/// A trait for tuples that can have types pushed to them
#[doc(hidden)]
pub trait TuplePush<T> {
    /// The resulting type after pushing T
    type Pushed;
}

/// A trait for tuples of SolCalls that can be decoded
#[doc(hidden)]
pub trait CallTuple: Sealed {
    /// The flattened return type
    type Returns;

    /// Decode the returns from a sequence of bytes
    fn decode_returns(data: &[Bytes]) -> Result<Self::Returns>;
}

// Empty tuple implementation
impl Sealed for () {}

impl<T: SolCall> TuplePush<T> for () {
    type Pushed = (T,);
}

impl CallTuple for () {
    type Returns = ();

    fn decode_returns(_: &[Bytes]) -> Result<Self::Returns> {
        Ok(())
    }
}

// Macro to implement for tuples of different sizes
macro_rules! impl_tuple {
    ($($idx:tt => $ty:ident),+) => {
        impl<$($ty: SolCall,)+> Sealed for ($($ty,)+) {}

        // Implement pushing a new type onto the tuple
        impl<T: SolCall, $($ty: SolCall,)+> TuplePush<T> for ($($ty,)+) {
            type Pushed = ($($ty,)+ T,);
        }

        // Implement decoding for the tuple
        impl<$($ty: SolCall,)+> CallTuple for ($($ty,)+) {
            // The Returns associated type is a tuple of each SolCall's Return type
            type Returns = ($($ty::Return,)+);

            fn decode_returns(data: &[Bytes]) -> Result<Self::Returns> {
                if data.len() != count!($($ty),+) {
                    return Err(Error::MulticallError(MulticallError::NoReturnData));
                }

                // Decode each return value in order
                Ok(($($ty::abi_decode_returns(&data[$idx], true)?,)+))
            }
        }
    };
}

// Helper macro to count number of types
macro_rules! count {
    () => (0);
    ($x:tt $(,$xs:tt)*) => (1 + count!($($xs),*));
}

// Max CALL_LIMIT is 16
impl_tuple!(0 => T1);
impl_tuple!(0 => T1, 1 => T2);
impl_tuple!(0 => T1, 1 => T2, 2 => T3);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10, 10 => T11);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10, 10 => T11, 11 => T12);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10, 10 => T11, 11 => T12, 12 => T13);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10, 10 => T11, 11 => T12, 12 => T13, 13 => T14);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10, 10 => T11, 11 => T12, 12 => T13, 13 => T14, 14 => T15);
impl_tuple!(0 => T1, 1 => T2, 2 => T3, 3 => T4, 4 => T5, 5 => T6, 6 => T7, 7 => T8, 8 => T9, 9 => T10, 10 => T11, 11 => T12, 12 => T13, 13 => T14, 14 => T15, 15 => T16);
