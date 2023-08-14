use alloy_json_rpc::RpcObject;

/// Captures type info for network-specific RPC requests/responses
pub trait Network: Sized + Send + Sync + 'static {
    #[doc(hidden)]
    /// Asserts that this trait can only be implemented on a ZST.
    const __ASSERT_ZST: bool = {
        assert!(std::mem::size_of::<Self>() == 0, "Network must be a ZST");
        true
    };

    /// The JSON body of a transaction request.
    type TransactionRequest: Transaction;

    /// The JSON body of a transaction receipt.
    type Receipt: Receipt;

    /// The JSON body of a transaction response.
    type TransactionResponse: Transaction;
}

/// Captures getters and setters common across transactions and
/// transaction-like objects across all networks.
pub trait Transaction:
    alloy_rlp::Encodable + alloy_rlp::Decodable + RpcObject + Clone + Sized + 'static
{
    fn set_gas(&mut self, gas: alloy_primitives::U256);
}

/// Captures getters and setters common across EIP-1559 transactions across all networks
pub trait Eip1559Transaction: Transaction {}

/// Captures getters and setters common across receipts across all networks
pub trait Receipt: RpcObject + 'static {}
