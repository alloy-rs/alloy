use std::marker::PhantomData;

use alloy_json_rpc::{RpcObject, RpcParam, RpcReturn};
use alloy_transports::{BoxTransport, RpcCall, RpcClient, Transport, TransportError};

pub trait Network {
    type Transaction: Transaction;
    type Receipt: RpcObject;
}

pub trait Transaction: alloy_rlp::Encodable + alloy_rlp::Decodable + RpcObject + Sized {}

pub trait Eip1559Transaction: Transaction {}
