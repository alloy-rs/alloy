use std::{
    ffi::{CString, OsString},
    path::PathBuf,
};

#[derive(Debug, Clone)]
/// An IPC Connection object.
pub struct IpcConnect<T> {
    ///
    inner: T,
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

            fn connect<'a: 'b, 'b>(
                &'a self,
            ) -> alloy_transport::Pbf<
                'b,
                alloy_pubsub::ConnectionHandle,
                alloy_transport::TransportError,
            > {
                Box::pin(async move {
                    crate::IpcBackend::connect(&self.inner)
                        .await
                        .map_err(alloy_transport::TransportErrorKind::custom)
                })
            }
        }
    };
}

impl_connect!(OsString);
impl_connect!(CString);
impl_connect!(PathBuf);
impl_connect!(String);
