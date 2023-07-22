use alloy_json_rpc::RpcObject;

pub trait Network {
    type Transaction: Transaction;
    type Receipt: RpcObject;
}

pub trait Transaction: alloy_rlp::Encodable + alloy_rlp::Decodable + RpcObject {}

pub trait Eip1559Transaction: Transaction {}

pub trait Middleware<N> {
    type Inner: Middleware<N>;
}
