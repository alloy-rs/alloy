use alloy_eips::eip712::TypedDataRequest;
use alloy_primitives::Bytes;
use alloy_serde::WithOtherFields;
use serde::{Deserialize, Serialize};

use crate::TransactionRequest;

/// Either normal raw tx or typed data with signature
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum SeismicRawTxRequest {
    /// A raw seismic tx
    Bytes(Bytes),
    /// An EIP-712 typed data request with a signature
    TypedData(TypedDataRequest),
}

impl Into<SeismicRawTxRequest> for Bytes {
    fn into(self) -> SeismicRawTxRequest {
        SeismicRawTxRequest::Bytes(self)
    }
}

impl Into<SeismicRawTxRequest> for TypedDataRequest {
    fn into(self) -> SeismicRawTxRequest {
        SeismicRawTxRequest::TypedData(self)
    }
}

/// Either a normal ETH call, raw tx, or typed data with signature
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum SeismicCallRequest {
    /// EIP-712 signed typed message with signature
    TypedData(TypedDataRequest),
    /// normal call request
    TransactionRequest(WithOtherFields<TransactionRequest>),
    /// signed raw seismic tx
    Bytes(Bytes),
}

impl Into<SeismicCallRequest> for TypedDataRequest {
    fn into(self) -> SeismicCallRequest {
        SeismicCallRequest::TypedData(self)
    }
}

impl Into<SeismicCallRequest> for WithOtherFields<TransactionRequest> {
    fn into(self) -> SeismicCallRequest {
        SeismicCallRequest::TransactionRequest(self)
    }
}

impl Into<SeismicCallRequest> for TransactionRequest {
    fn into(self) -> SeismicCallRequest {
        SeismicCallRequest::TransactionRequest(WithOtherFields::new(self))
    }
}

impl Into<SeismicCallRequest> for Bytes {
    fn into(self) -> SeismicCallRequest {
        SeismicCallRequest::Bytes(self)
    }
}
