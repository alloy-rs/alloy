#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]

use alloy_consensus::{BlockHeader, Transaction, TxReceipt};
use alloy_eips::eip2718::{Eip2718Envelope, Eip2718Error};
use alloy_json_rpc::RpcObject;
use alloy_network_primitives::HeaderResponse;
use core::fmt::{Debug, Display};

mod transaction;
pub use transaction::{
    BuildResult, FullSigner, FullSignerSync, NetworkWallet, TransactionBuilder,
    TransactionBuilder4844, TransactionBuilder7594, TransactionBuilder7702,
    TransactionBuilderError, TxSigner, TxSignerSync, UnbuiltTransactionError,
};

mod ethereum;
pub use ethereum::{Ethereum, EthereumWallet, IntoWallet};

/// Types for handling unknown network types.
pub mod any;
pub use any::{
    AnyHeader, AnyNetwork, AnyReceiptEnvelope, AnyRpcBlock, AnyRpcHeader, AnyRpcTransaction,
    AnyTransactionReceipt, AnyTxEnvelope, AnyTxType, AnyTypedTransaction, UnknownTxEnvelope,
    UnknownTypedTransaction,
};

pub use alloy_eips::eip2718;
use alloy_eips::Typed2718;
pub use alloy_network_primitives::{
    self as primitives, BlockResponse, ReceiptResponse, TransactionResponse,
};

/// Captures type info for network-specific RPC requests/responses.
///
/// Networks are only containers for types, so it is recommended to use ZSTs for their definition.
pub trait Network: Debug + Clone + Copy + Sized + Send + Sync + 'static {
    // -- Consensus types --

    /// The network transaction type enum.
    ///
    /// This should be a simple `#[repr(u8)]` enum, and as such has strict type
    /// bounds for better use in error messages, assertions etc.
    #[doc(alias = "TransactionType")]
    type TxType: Typed2718
        + Into<u8>
        + PartialEq
        + Eq
        + TryFrom<u8, Error = Eip2718Error>
        + Debug
        + Display
        + Clone
        + Copy
        + Send
        + Sync
        + 'static;

    /// The network transaction envelope type.
    #[doc(alias = "TransactionEnvelope")]
    type TxEnvelope: Eip2718Envelope + Transaction + Debug;

    /// An enum over the various transaction types.
    #[doc(alias = "UnsignedTransaction")]
    type UnsignedTx: From<Self::TxEnvelope>;

    /// The network receipt envelope type.
    #[doc(alias = "TransactionReceiptEnvelope", alias = "TxReceiptEnvelope")]
    type ReceiptEnvelope: Eip2718Envelope + TxReceipt;

    /// The network header type.
    type Header: BlockHeader;

    // -- JSON RPC types --

    /// The JSON body of a transaction request.
    #[doc(alias = "TxRequest")]
    type TransactionRequest: RpcObject
        + TransactionBuilder<Self>
        + Debug
        + From<Self::TxEnvelope>
        + From<Self::UnsignedTx>;

    /// The JSON body of a transaction response.
    #[doc(alias = "TxResponse")]
    type TransactionResponse: RpcObject + TransactionResponse + AsRef<Self::TxEnvelope>;

    /// The JSON body of a transaction receipt.
    #[doc(alias = "TransactionReceiptResponse", alias = "TxReceiptResponse")]
    type ReceiptResponse: RpcObject + ReceiptResponse;

    /// The JSON body of a header response.
    type HeaderResponse: RpcObject + HeaderResponse + AsRef<Self::Header>;

    /// The JSON body of a block response.
    type BlockResponse: RpcObject
        + BlockResponse<Transaction = Self::TransactionResponse, Header = Self::HeaderResponse>;
}

/// Utility to implement IntoWallet for signer over the specified network.
#[macro_export]
macro_rules! impl_into_wallet {
    ($(@[$($generics:tt)*])? $signer:ty) => {
        impl $(<$($generics)*>)? $crate::IntoWallet for $signer {
            type NetworkWallet = $crate::EthereumWallet;
            fn into_wallet(self) -> Self::NetworkWallet {
                $crate::EthereumWallet::from(self)
            }
        }

        impl $(<$($generics)*>)? $crate::IntoWallet<$crate::AnyNetwork> for $signer {
            type NetworkWallet = $crate::EthereumWallet;
            fn into_wallet(self) -> Self::NetworkWallet {
                $crate::EthereumWallet::from(self)
            }
        }
    };
}
