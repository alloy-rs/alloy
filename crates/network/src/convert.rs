//! Conversion traits between consensus and RPC types.
//!
//! This module provides traits for converting between consensus-layer types and
//! RPC-layer types, enabling generic implementations that work across different
//! network types.

use crate::{Network, TxSigner};
use alloy_consensus::{
    error::ValueError, transaction::Recovered, EthereumTxEnvelope, SignableTransaction, TxEip4844,
    Transaction,
};
use alloy_primitives::{Address, Signature};
use alloy_rpc_types_eth::{TransactionInfo, TransactionRequest};
use core::{error, fmt::Debug, future::Future};
use std::convert::Infallible;

/// Converts `T` into `self`. It is reciprocal of [`IntoRpcTx`].
///
/// Should create an RPC transaction response object based on a consensus transaction, its signer
/// [`Address`] and an additional context [`FromConsensusTx::TxInfo`].
///
/// Prefer implementing [`FromConsensusTx`] over [`IntoRpcTx`] because it automatically provides an
/// implementation of [`IntoRpcTx`] thanks to the blanket implementation in this crate.
///
/// Prefer using [`IntoRpcTx`] over using [`FromConsensusTx`] when specifying trait bounds on a
/// generic function. This way, types that directly implement [`IntoRpcTx`] can be used as arguments
/// as well.
pub trait FromConsensusTx<T>: Sized {
    /// An additional context, usually [`TransactionInfo`] in a wrapper that carries some
    /// implementation specific extra information.
    type TxInfo;
    /// An associated RPC conversion error.
    type Err: error::Error;

    /// Performs the conversion consuming `tx` with `signer` and `tx_info`. See [`FromConsensusTx`]
    /// for details.
    fn from_consensus_tx(tx: T, signer: Address, tx_info: Self::TxInfo) -> Result<Self, Self::Err>;
}

impl<TxIn: Transaction, T: Transaction + From<TxIn>> FromConsensusTx<TxIn>
    for alloy_rpc_types_eth::Transaction<T>
{
    type TxInfo = TransactionInfo;
    type Err = Infallible;

    fn from_consensus_tx(
        tx: TxIn,
        signer: Address,
        tx_info: Self::TxInfo,
    ) -> Result<Self, Self::Err> {
        Ok(Self::from_transaction(Recovered::new_unchecked(tx.into(), signer), tx_info))
    }
}

/// Converts `self` into `T`. The opposite of [`FromConsensusTx`].
///
/// Should create an RPC transaction response object based on a consensus transaction, its signer
/// [`Address`] and an additional context [`IntoRpcTx::TxInfo`].
///
/// Avoid implementing [`IntoRpcTx`] and use [`FromConsensusTx`] instead. Implementing it
/// automatically provides an implementation of [`IntoRpcTx`] thanks to the blanket implementation
/// in this crate.
///
/// Prefer using [`IntoRpcTx`] over [`FromConsensusTx`] when specifying trait bounds on a generic
/// function to ensure that types that only implement [`IntoRpcTx`] can be used as well.
pub trait IntoRpcTx<T> {
    /// An additional context, usually [`TransactionInfo`] in a wrapper that carries some
    /// implementation specific extra information.
    type TxInfo;
    /// An associated RPC conversion error.
    type Err: error::Error;

    /// Performs the conversion consuming `self` with `signer` and `tx_info`. See [`IntoRpcTx`]
    /// for details.
    fn into_rpc_tx(self, signer: Address, tx_info: Self::TxInfo) -> Result<T, Self::Err>;
}

impl<ConsensusTx, RpcTx> IntoRpcTx<RpcTx> for ConsensusTx
where
    ConsensusTx: Transaction,
    RpcTx: FromConsensusTx<Self>,
    <RpcTx as FromConsensusTx<ConsensusTx>>::Err: Debug,
{
    type TxInfo = RpcTx::TxInfo;
    type Err = <RpcTx as FromConsensusTx<ConsensusTx>>::Err;

    fn into_rpc_tx(self, signer: Address, tx_info: Self::TxInfo) -> Result<RpcTx, Self::Err> {
        RpcTx::from_consensus_tx(self, signer, tx_info)
    }
}

/// Trait for converting network transaction responses to primitive transaction types.
pub trait TryFromTransactionResponse<N: Network> {
    /// The error type returned if the conversion fails.
    type Error: core::error::Error + Send + Sync + Unpin;

    /// Converts a network transaction response to a primitive transaction type.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Self)` on successful conversion, or `Err(Self::Error)` if the conversion fails.
    fn from_transaction_response(
        transaction_response: N::TransactionResponse,
    ) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

/// Trait for converting network receipt responses to primitive receipt types.
pub trait TryFromReceiptResponse<N: Network> {
    /// The error type returned if the conversion fails.
    type Error: core::error::Error + Send + Sync + Unpin;

    /// Converts a network receipt response to a primitive receipt type.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Self)` on successful conversion, or `Err(Self::Error)` if the conversion fails.
    fn from_receipt_response(receipt_response: N::ReceiptResponse) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

/// Converts `self` into `T`.
///
/// Should create a fake transaction for simulation using [`TransactionRequest`].
pub trait TryIntoSimTx<T>
where
    Self: Sized,
{
    /// Performs the conversion.
    ///
    /// Should return a signed typed transaction envelope for the [`eth_simulateV1`] endpoint with a
    /// dummy signature or an error if [required fields] are missing.
    ///
    /// [`eth_simulateV1`]: <https://github.com/ethereum/execution-apis/pull/484>
    /// [required fields]: TransactionRequest::buildable_type
    fn try_into_sim_tx(self) -> Result<T, ValueError<Self>>;
}

impl TryIntoSimTx<EthereumTxEnvelope<TxEip4844>> for TransactionRequest {
    fn try_into_sim_tx(self) -> Result<EthereumTxEnvelope<TxEip4844>, ValueError<Self>> {
        Self::build_typed_simulate_transaction(self)
    }
}

/// Error for [`SignableTxRequest`] trait.
#[derive(Debug, thiserror::Error)]
pub enum SignTxRequestError {
    /// The transaction request is invalid.
    #[error("invalid transaction request")]
    InvalidTransactionRequest,

    /// The signer is not supported.
    #[error(transparent)]
    SignerNotSupported(#[from] alloy_signer::Error),
}

/// An abstraction over transaction requests that can be signed.
pub trait SignableTxRequest<T>: Send + Sync + 'static {
    /// Attempts to build a transaction request and sign it with the given signer.
    fn try_build_and_sign(
        self,
        signer: impl TxSigner<Signature> + Send,
    ) -> impl Future<Output = Result<T, SignTxRequestError>> + Send;
}

impl SignableTxRequest<EthereumTxEnvelope<TxEip4844>> for TransactionRequest {
    async fn try_build_and_sign(
        self,
        signer: impl TxSigner<Signature> + Send,
    ) -> Result<EthereumTxEnvelope<TxEip4844>, SignTxRequestError> {
        let mut tx =
            self.build_typed_tx().map_err(|_| SignTxRequestError::InvalidTransactionRequest)?;
        let signature = signer.sign_transaction(&mut tx).await?;
        Ok(tx.into_signed(signature).into())
    }
}
