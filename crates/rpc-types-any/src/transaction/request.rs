use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

use alloy_consensus_any::{AnyTxEnvelope, AnyTypedTransaction, UnknownTypedTransaction};
use alloy_eips::eip7702::SignedAuthorization;
use alloy_network_primitives::{TransactionBuilder4844, TransactionBuilder7702};
use alloy_rpc_types_eth::BlobTransactionSidecar;
use alloy_rpc_types_eth::TransactionRequest;
use alloy_serde::WithOtherFields;

/// A catch-all transaction request type for handling transactions on multiple networks.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[doc(alias = "AnyTxRequest")]
pub struct AnyTransactionRequest(pub WithOtherFields<TransactionRequest>);

impl AnyTransactionRequest {
    /// Create a new transaction request.
    pub fn new(tx: TransactionRequest) -> Self {
        Self(WithOtherFields::new(tx))
    }
}

impl Deref for AnyTransactionRequest {
    type Target = TransactionRequest;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl DerefMut for AnyTransactionRequest {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

impl From<AnyTypedTransaction> for AnyTransactionRequest {
    fn from(value: AnyTypedTransaction) -> Self {
        match value {
            AnyTypedTransaction::Ethereum(tx) => Self(WithOtherFields::new(tx.into())),
            AnyTypedTransaction::Unknown(UnknownTypedTransaction { ty, mut fields, .. }) => {
                fields.insert("type".to_string(), serde_json::Value::Number(ty.0.into()));
                Self(WithOtherFields { inner: Default::default(), other: fields })
            }
        }
    }
}

impl From<AnyTxEnvelope> for AnyTransactionRequest {
    fn from(value: AnyTxEnvelope) -> Self {
        AnyTypedTransaction::from(value).into()
    }
}

impl TransactionBuilder4844 for AnyTransactionRequest {
    fn max_fee_per_blob_gas(&self) -> Option<u128> {
        self.max_fee_per_blob_gas
    }

    fn set_max_fee_per_blob_gas(&mut self, max_fee_per_blob_gas: u128) {
        self.max_fee_per_blob_gas = Some(max_fee_per_blob_gas)
    }

    fn blob_sidecar(&self) -> Option<&BlobTransactionSidecar> {
        self.sidecar.as_ref()
    }

    fn set_blob_sidecar(&mut self, sidecar: BlobTransactionSidecar) {
        self.sidecar = Some(sidecar);
        self.populate_blob_hashes();
    }
}

impl TransactionBuilder7702 for AnyTransactionRequest {
    fn authorization_list(&self) -> Option<&Vec<SignedAuthorization>> {
        self.authorization_list.as_ref()
    }

    fn set_authorization_list(&mut self, authorization_list: Vec<SignedAuthorization>) {
        self.authorization_list = Some(authorization_list);
    }
}
