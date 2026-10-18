//! Sharing errors between independent consumers of one operation.

use crate::{TransportError, TransportErrorKind};
use std::{error::Error, fmt, sync::Arc};

/// Shares a transport error between consumers of the same failed operation.
///
/// JSON-RPC payloads, HTTP metadata, retry classification, and RPC error variants are preserved.
/// Arbitrary boxed causes remain available through the returned error's [`Error::source`] chain,
/// rather than through a direct downcast of the shared wrapper.
///
/// Since [`serde_json::Error`] cannot be cloned, serialization and deserialization errors use an
/// IO proxy. The proxy preserves the message, but its category is `Io` and its line and column are
/// zero. The original serde error, including its category and location, remains in the source
/// chain.
#[derive(Clone, Debug)]
pub struct SharedTransportError(Arc<TransportError>);

impl SharedTransportError {
    /// Shares an error without discarding its source.
    pub fn new(error: TransportError) -> Self {
        Self(Arc::new(error))
    }

    /// Returns an owned error for one consumer.
    ///
    /// See the type documentation for how non-cloneable causes are shared.
    pub fn to_error(&self) -> TransportError {
        match self.0.as_ref() {
            TransportError::ErrorResp(error) => TransportError::ErrorResp(error.clone()),
            TransportError::NullResp => TransportError::NullResp,
            TransportError::UnsupportedFeature(feature) => {
                TransportError::UnsupportedFeature(feature)
            }
            TransportError::LocalUsageError(_) => {
                TransportError::LocalUsageError(Box::new(SharedSource(self.0.clone())))
            }
            // serde_json errors are not cloneable. Keep their classification and retain the
            // original error, including its location and category, in the source chain.
            TransportError::SerError(_) => TransportError::SerError(serde_json::Error::io(
                std::io::Error::other(SharedSource(self.0.clone())),
            )),
            TransportError::DeserError { text, .. } => TransportError::DeserError {
                err: serde_json::Error::io(std::io::Error::other(SharedSource(self.0.clone()))),
                text: text.clone(),
            },
            TransportError::Transport(error) => TransportError::Transport(match error {
                TransportErrorKind::MissingBatchResponse(id) => {
                    TransportErrorKind::MissingBatchResponse(id.clone())
                }
                TransportErrorKind::BackendGone => TransportErrorKind::BackendGone,
                TransportErrorKind::PubsubUnavailable => TransportErrorKind::PubsubUnavailable,
                TransportErrorKind::HttpError(error) => {
                    TransportErrorKind::HttpError(crate::HttpError {
                        status: error.status,
                        body: error.body.clone(),
                    })
                }
                TransportErrorKind::HttpErrorWithRetryAfter { error, retry_after } => {
                    TransportErrorKind::HttpErrorWithRetryAfter {
                        error: crate::HttpError { status: error.status, body: error.body.clone() },
                        retry_after: *retry_after,
                    }
                }
                TransportErrorKind::NonRetryable(_) => {
                    TransportErrorKind::NonRetryable(Box::new(SharedSource(self.0.clone())))
                }
                // Arbitrary transport causes cannot be cloned, so share their source chain.
                TransportErrorKind::Custom(_) => {
                    TransportErrorKind::Custom(Box::new(SharedSource(self.0.clone())))
                }
            }),
        }
    }
}

#[derive(Debug)]
struct SharedSource(Arc<TransportError>);

impl fmt::Display for SharedSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.as_ref() {
            TransportError::LocalUsageError(error) => error.fmt(f),
            TransportError::SerError(error) | TransportError::DeserError { err: error, .. } => {
                error.fmt(f)
            }
            error => error.fmt(f),
        }
    }
}

impl Error for SharedSource {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.0.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[derive(Debug, thiserror::Error)]
    #[error("typed cause")]
    struct TypedCause;

    #[test]
    fn preserves_http_details_and_retry_classification() {
        let shared = SharedTransportError::new(TransportErrorKind::http_error_with_retry_after(
            429,
            "rate limited".to_owned(),
            Some(Duration::from_secs(5)),
        ));
        for _ in 0..2 {
            let error = shared.to_error();
            let kind = error.as_transport_err().unwrap();
            assert_eq!(kind.as_http_error().unwrap().status, 429);
            assert_eq!(kind.as_http_error().unwrap().body, "rate limited");
            assert_eq!(kind.retry_after(), Some(Duration::from_secs(5)));
            assert!(kind.is_retry_err());
        }
    }

    #[test]
    fn preserves_custom_typed_source() {
        let shared = SharedTransportError::new(TransportErrorKind::custom(TypedCause));
        let error = shared.to_error();
        assert!(error.as_transport_err().unwrap().as_custom().is_some());
        let mut source: &(dyn Error + 'static) = &error;
        while !source.is::<TypedCause>() {
            source = source.source().expect("typed cause retained in source chain");
        }
    }

    #[test]
    fn preserves_nonretryable_typed_source() {
        let shared = SharedTransportError::new(TransportErrorKind::non_retryable(TypedCause));
        let error = shared.to_error();
        assert!(error.as_transport_err().unwrap().is_non_retryable());
        let mut source: &(dyn Error + 'static) = &error;
        while !source.is::<TypedCause>() {
            source = source.source().expect("typed cause retained in source chain");
        }
    }

    #[test]
    fn preserves_local_error_display_and_source() {
        let shared = SharedTransportError::new(TransportError::local_usage(TypedCause));
        let error = shared.to_error();
        assert!(error.is_local_usage_error());
        assert_eq!(error.to_string(), "local usage error: typed cause");
    }

    #[test]
    fn preserves_deserialization_error_text_and_source() {
        let original = serde_json::from_str::<u64>("invalid").unwrap_err();
        let message = original.to_string();
        let category = original.classify();
        let location = (original.line(), original.column());
        let shared = SharedTransportError::new(TransportError::DeserError {
            err: original,
            text: "invalid".to_owned(),
        });
        let error = shared.to_error();
        let TransportError::DeserError { err, text } = &error else {
            panic!("expected deserialization error");
        };
        assert_eq!(text, "invalid");
        assert_eq!(err.to_string(), message);
        assert!(err.is_io());
        assert_eq!((err.line(), err.column()), (0, 0));

        let mut source: &(dyn Error + 'static) = &error;
        loop {
            if let Some(original) =
                source.downcast_ref::<serde_json::Error>().filter(|err| !err.is_io())
            {
                assert_eq!(original.classify(), category);
                assert_eq!((original.line(), original.column()), location);
                break;
            }
            source = source.source().expect("original serde error retained in source chain");
        }
    }
}
