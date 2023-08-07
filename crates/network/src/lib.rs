use std::marker::PhantomData;

use alloy_json_rpc::{RpcObject, RpcParam, RpcReturn};
use alloy_transports::{RpcCall, RpcClient, Transport};

pub trait Network {
    type Transaction: Transaction;
    type Receipt: RpcObject;
}

pub trait Transaction: alloy_rlp::Encodable + alloy_rlp::Decodable + RpcObject {}

pub trait Eip1559Transaction: Transaction {}

pub trait Middleware<N: Network, T: Transport> {
    fn client(&self) -> &RpcClient<T>;

    fn inner(&self) -> &dyn Middleware<N, T>;

    fn send_transaction(&self, tx: N::Transaction) -> MwareCall<T, N, N::Transaction, N::Receipt> {
        self.inner().send_transaction(tx)
    }
}

impl<N: Network, T: Transport> Middleware<N, T> for RpcClient<T> {
    fn client(&self) -> &RpcClient<T> {
        self
    }

    fn inner(&self) -> &dyn Middleware<N, T> {
        panic!("called inner on <RpcClient as Middleware>")
    }

    fn send_transaction(&self, tx: N::Transaction) -> MwareCall<T, N, N::Transaction, N::Receipt> {
        self.prepare("eth_sendTransaction", tx).into()
    }
}

pub struct MwareCall<T, N, Params, Resp>
where
    T: Transport,
    N: Network,
    Params: RpcParam,
    Resp: RpcReturn,
{
    pub(crate) inner: RpcCall<T, Params, Resp>,
    pub(crate) pre: Option<Box<dyn FnOnce(Params) -> Params>>,
    pub(crate) post: Option<Box<dyn FnOnce(Resp) -> Resp>>,
    _pd: PhantomData<fn() -> N>,
}

impl<T, N, Params, Resp> From<RpcCall<T, Params, Resp>> for MwareCall<T, N, Params, Resp>
where
    T: Transport,
    N: Network,
    Params: RpcParam,
    Resp: RpcReturn,
{
    fn from(value: RpcCall<T, Params, Resp>) -> Self {
        Self {
            inner: value,
            pre: None,
            post: None,
            _pd: PhantomData,
        }
    }
}
