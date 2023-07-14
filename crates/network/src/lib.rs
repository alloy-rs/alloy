use alloy_transports::{RpcParam, RpcResp};

mod transaction;
pub use transaction::{Eip1559Transaction, Transaction};

pub trait Network: Sized + Send + Sync + 'static {
    #[doc(hidden)]
    const __ENFORCE_ZST: () = assert!(
        // This ensures that the network is a zero-sized type
        std::mem::size_of::<Self>() == 0,
        "Network must be a zero-sized type"
    );

    // argument for `eth_sendTransaction`
    type Transaction: Transaction + RpcParam;

    // return for `eth_getTransaction`
    type TransactionRespose: RpcResp;

    // return for `eth_getTransactionReceipt`
    type Receipt: RpcResp;
}

mod mware1;

mod mware2;
