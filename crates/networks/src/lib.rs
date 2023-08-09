use alloy_json_rpc::RpcObject;

pub trait Network {
    /// The JSON body of a transaction request.
    type TransactionRequest: Transaction;

    /// The JSON body of a transaction receipt.
    type Receipt: Receipt;

    /// The JSON body of a transaction response.
    type TransactionResponse: Transaction;
}

/// Captures getters and setters common across transactions across all networks
pub trait Transaction:
    alloy_rlp::Encodable + alloy_rlp::Decodable + RpcObject + Sized + 'static
{
}

/// Captures getters and setters common across EIP-1559 transactions across all networks
pub trait Eip1559Transaction: Transaction {}

/// Captures getters and setters common across receipts across all networks
pub trait Receipt: RpcObject + 'static {}
