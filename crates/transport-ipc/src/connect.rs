use std::{
    ffi::{CString, OsString},
    path::PathBuf,
};

/// An IPC Connection object.
#[derive(Clone, Debug)]
pub struct IpcConnect<T> {
    inner: T,
}

impl<T> IpcConnect<T> {
    /// Create a new IPC connection object for any type T that can be converted into
    /// `IpcConnect<T>`.
    pub const fn new(inner: T) -> Self
    where
        Self: alloy_pubsub::PubSubConnect,
    {
        Self { inner }
    }
}

macro_rules! impl_connect {
    ($target:ty) => {
        impl From<$target> for IpcConnect<$target> {
            fn from(inner: $target) -> Self {
                Self { inner }
            }
        }

        impl From<IpcConnect<$target>> for $target {
            fn from(this: IpcConnect<$target>) -> $target {
                this.inner
            }
        }

        impl alloy_pubsub::PubSubConnect for IpcConnect<$target> {
            fn is_local(&self) -> bool {
                true
            }

            async fn connect(
                &self,
            ) -> Result<alloy_pubsub::ConnectionHandle, alloy_transport::TransportError> {
                crate::IpcBackend::connect(&self.inner)
                    .await
                    .map_err(alloy_transport::TransportErrorKind::custom)
            }
        }
    };
}

impl_connect!(OsString);
impl_connect!(CString);
impl_connect!(PathBuf);
impl_connect!(String);
