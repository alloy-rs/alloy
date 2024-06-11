use alloy_network::Network;

/// A transaction that can be sent. This is either a builder or an envelope.
///
/// This type is used to allow for fillers to convert a builder into an envelope
/// without changing the user-facing API.
///
/// Users should NOT use this type directly. It should only be used as an
/// implementation detail of [`Provider::send_transaction_internal`].
#[doc(hidden, alias = "SendableTransaction")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SendableTx<N: Network> {
    /// A transaction that is not yet signed.
    Builder(N::TransactionRequest),
    /// A transaction that is signed and fully constructed.
    Envelope(N::TxEnvelope),
}

impl<N: Network> SendableTx<N> {
    /// Fallible cast to an unbuilt transaction request.
    pub fn as_mut_builder(&mut self) -> Option<&mut N::TransactionRequest> {
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
        matches!(self, Self::Envelope(_))
    }

    /// Fallible cast to a built transaction envelope.
    pub const fn as_envelope(&self) -> Option<&N::TxEnvelope> {
        match self {
            Self::Envelope(tx) => Some(tx),
            _ => None,
        }
    }
}
