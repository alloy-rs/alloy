use alloy_network::Network;

/// A transaction that can be sent. This is either a builder or an envelope.
///
/// This type is used to allow for fillers to convert a builder into an envelope
/// without changing the user-facing API.
///
/// Users should NOT use this type directly. It should only be used as an
/// implementation detail of [`Provider::send_transaction_internal`](crate::Provider).
#[doc(alias = "SendableTransaction")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SendableTx<N: Network> {
    /// A transaction that is not yet signed.
    Builder(N::TransactionRequest),
    /// A transaction that is signed and fully constructed.
    Envelope(N::TxEnvelope),
    /// A canonical envelope with separately encoded network sidecar data.
    EnvelopeWithSidecar {
        /// Canonical transaction, excluding sidecar data.
        envelope: N::TxEnvelope,
        /// Raw submission bytes including the network sidecar.
        encoded: alloy_primitives::Bytes,
    },
}

impl<N: Network> SendableTx<N> {
    /// Fallible cast to an unbuilt transaction request.
    pub const fn as_mut_builder(&mut self) -> Option<&mut N::TransactionRequest> {
        match self {
            Self::Builder(tx) => Some(tx),
            _ => None,
        }
    }

    /// Fallible cast to an unbuilt transaction request.
    pub const fn as_builder(&self) -> Option<&N::TransactionRequest> {
        match self {
            Self::Builder(tx) => Some(tx),
            _ => None,
        }
    }

    /// Checks if the transaction is a builder.
    pub const fn is_builder(&self) -> bool {
        matches!(self, Self::Builder(_))
    }

    /// Check if the transaction is an envelope.
    pub const fn is_envelope(&self) -> bool {
        matches!(self, Self::Envelope(_) | Self::EnvelopeWithSidecar { .. })
    }

    /// Fallible cast to a built transaction envelope.
    pub const fn as_envelope(&self) -> Option<&N::TxEnvelope> {
        match self {
            Self::Envelope(tx) | Self::EnvelopeWithSidecar { envelope: tx, .. } => Some(tx),
            _ => None,
        }
    }

    /// Returns the canonical envelope for either built variant, discarding network-only sidecar
    /// bytes.
    ///
    /// Returns a [`SendableTxErr`] with the request object otherwise.
    pub fn try_into_envelope(self) -> Result<N::TxEnvelope, SendableTxErr<N::TransactionRequest>> {
        match self {
            Self::Builder(req) => Err(SendableTxErr::new(req)),
            Self::Envelope(env) | Self::EnvelopeWithSidecar { envelope: env, .. } => Ok(env),
        }
    }

    /// Returns the request if this variant is an [`SendableTx::Builder`].
    ///
    /// Returns a [`SendableTxErr`] with the envelope object otherwise.
    pub fn try_into_request(self) -> Result<N::TransactionRequest, SendableTxErr<N::TxEnvelope>> {
        match self {
            Self::Builder(req) => Ok(req),
            Self::Envelope(env) | Self::EnvelopeWithSidecar { envelope: env, .. } => {
                Err(SendableTxErr::new(env))
            }
        }
    }
}

/// Error when converting a [`SendableTx`].
#[derive(Debug, Clone, thiserror::Error)]
#[error("Unexpected variant: {0:?}")]
pub struct SendableTxErr<T>(pub T);

impl<T> SendableTxErr<T> {
    /// Create a new error.
    pub const fn new(inner: T) -> Self {
        Self(inner)
    }

    /// Unwrap the error and return the original value.
    pub fn into_inner(self) -> T {
        self.0
    }
}
