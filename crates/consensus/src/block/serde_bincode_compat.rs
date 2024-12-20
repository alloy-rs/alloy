//! Helper for managing bincode serialization and deserialization of types.

use core::fmt::Debug;

pub use super::header::{serde_bincode_compat::*};
use serde::{de::DeserializeOwned, Serialize};

/// Trait for types that can be serialized and deserialized using bincode.
pub trait SerdeBincodeCompat: Sized + 'static {
    /// Serde representation of the type for bincode serialization.
    type BincodeRepr<'a>: Debug + Serialize + DeserializeOwned + From<&'a Self> + Into<Self>;
}

impl SerdeBincodeCompat for crate::Header {
    type BincodeRepr<'a> = crate::serde_bincode_compat::Header<'a>;
}
