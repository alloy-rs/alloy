use interprocess::local_socket as ls;
use std::io;

pub(crate) fn to_name(path: &std::ffi::OsStr) -> io::Result<ls::Name<'_>> {
    if cfg!(windows) && !path.as_encoded_bytes().starts_with(br"\\.\pipe\") {
        ls::ToNsName::to_ns_name::<ls::GenericNamespaced>(path)
    } else {
        ls::ToFsName::to_fs_name::<ls::GenericFilePath>(path)
    }
}

/// An IPC Connection object.
///
/// By default, messages from the server are limited to [`DEFAULT_MAX_RESPONSE_SIZE`] bytes.
///
/// [`DEFAULT_MAX_RESPONSE_SIZE`]: alloy_transport::DEFAULT_MAX_RESPONSE_SIZE
#[derive(Clone, Debug)]
pub struct IpcConnect<T> {
    inner: T,
    max_response_size: Option<usize>,
}

impl<T> IpcConnect<T> {
    /// Create a new IPC connection object for any type T that can be converted into
    /// `IpcConnect<T>`.
    pub const fn new(inner: T) -> Self
    where
        Self: alloy_pubsub::PubSubConnect,
    {
        Self { inner, max_response_size: Some(alloy_transport::DEFAULT_MAX_RESPONSE_SIZE) }
    }

    /// Set the maximum size of a single message from the server in bytes, or `None` for no limit.
    ///
    /// A larger message closes the connection with a non-retryable error, failing all pending
    /// requests, as retrying the request would likely produce the same message. Defaults to
    /// [`DEFAULT_MAX_RESPONSE_SIZE`].
    ///
    /// [`DEFAULT_MAX_RESPONSE_SIZE`]: alloy_transport::DEFAULT_MAX_RESPONSE_SIZE
    pub const fn with_max_response_size(mut self, max_response_size: Option<usize>) -> Self {
        self.max_response_size = max_response_size;
        self
    }

    /// Get the maximum size of a single message from the server in bytes, if any.
    pub const fn max_response_size(&self) -> Option<usize> {
        self.max_response_size
    }
}

macro_rules! impl_connect {
    ($target:ty => | $inner:ident | $map:expr) => {
        impl From<$target> for IpcConnect<$target> {
            fn from(inner: $target) -> Self {
                Self::new(inner)
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
                let $inner = &self.inner;
                let inner = $map;
                let name = to_name(inner).map_err(alloy_transport::TransportErrorKind::custom)?;
                crate::IpcBackend::connect(name, self.max_response_size)
                    .await
                    .map_err(alloy_transport::TransportErrorKind::custom)
            }
        }
    };
}

impl_connect!(std::ffi::OsString => |s| s.as_os_str());
impl_connect!(std::path::PathBuf => |s| s.as_os_str());
impl_connect!(String => |s| s.as_ref());

#[cfg(unix)]
impl_connect!(std::ffi::CString => |s| {
    use std::os::unix::ffi::OsStrExt;
    std::ffi::OsStr::from_bytes(s.to_bytes())
});
