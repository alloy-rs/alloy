use interprocess::local_socket::{GenericFilePath, ToFsName};
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
    ($target:ty => $map:ident) => {
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
                let name = self
                    .inner
                    .$map()
                    .to_fs_name::<GenericFilePath>()
                    .map_err(alloy_transport::TransportErrorKind::custom)?;
                crate::IpcBackend::connect(name)
                    .await
                    .map_err(alloy_transport::TransportErrorKind::custom)
            }
        }
    };
}

impl_connect!(OsString => as_os_str);
impl_connect!(CString => as_c_str);
impl_connect!(PathBuf => as_path);
impl_connect!(String => as_str);
