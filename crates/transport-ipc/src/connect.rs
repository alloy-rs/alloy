use interprocess::local_socket as ls;
use std::io;

#[cfg(unix)]
pub(crate) fn to_name<'a, S>(path: impl ls::ToFsName<'a, S>) -> io::Result<ls::Name<'a>>
where
    S: ToOwned + ?Sized,
    ls::GenericFilePath: ls::PathNameType<S>,
{
    path.to_fs_name::<ls::GenericFilePath>()
}

#[cfg(windows)]
pub(crate) fn to_name<'a, S>(path: impl ls::ToNsName<'a, S>) -> io::Result<ls::Name<'a>>
where
    S: ToOwned + ?Sized,
    ls::GenericNamespaced: ls::NamespacedNameType<S>,
{
    path.to_ns_name::<ls::GenericNamespaced>()
}

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
    ($target:ty => $($map:tt)*) => {
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
                let name = to_name(self.inner $($map)*)
                    .map_err(alloy_transport::TransportErrorKind::custom)?;
                crate::IpcBackend::connect(name)
                    .await
                    .map_err(alloy_transport::TransportErrorKind::custom)
            }
        }
    };
}

impl_connect!(std::ffi::OsString => .as_os_str());
#[cfg(unix)]
impl_connect!(std::ffi::CString => .as_c_str());
impl_connect!(std::path::PathBuf => .as_os_str());
impl_connect!(String => .as_str());
