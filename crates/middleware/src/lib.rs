use alloy_networks::Network;
use alloy_transports::{BoxTransport, RpcClient, Transport};

pub trait Middleware<N: Network, T: Transport = BoxTransport> {
    fn client(&self) -> &RpcClient<T>;

    fn inner(&self) -> &dyn Middleware<N, T>;

    // fn send_transaction(&self, tx: N::Transaction) -> MwareCall<T, N, N::Transaction, N::Receipt> {
    //     self.inner().send_transaction(tx)
    // }
}

impl<N: Network, T: Transport> Middleware<N, T> for RpcClient<T> {
    fn client(&self) -> &RpcClient<T> {
        self
    }

    fn inner(&self) -> &dyn Middleware<N, T> {
        panic!("called inner on <RpcClient as Middleware>")
    }

    // fn send_transaction(&self, tx: N::Transaction) -> MwareCall<T, N, N::Transaction, N::Receipt> {
    //     self.prepare("eth_sendTransaction", tx).into()
    // }
}
// pub struct MwareCall<T, N, Params, Resp>
// where
//     T: Transport,
//     N: Network,
//     Params: RpcParam,
//     Resp: RpcReturn,
// {
//     pub(crate) inner: RpcCall<T, Params, Resp>,
//     pub(crate) pre: Option<
//         Box<
//             dyn FnOnce(
//                 Params,
//             )
//                 -> Box<dyn std::future::Future<Output = Result<Params, TransportError>>>,
//         >,
//     >,
//     pub(crate) post: Option<
//         Box<
//             dyn FnOnce(Resp) -> Box<dyn std::future::Future<Output = Result<Resp, TransportError>>>,
//         >,
//     >,
//     _pd: PhantomData<fn() -> N>,
// }

// impl<T, N, Params, Resp> From<RpcCall<T, Params, Resp>> for MwareCall<T, N, Params, Resp>
// where
//     T: Transport,
//     N: Network,
//     Params: RpcParam,
//     Resp: RpcReturn,
// {
//     fn from(value: RpcCall<T, Params, Resp>) -> Self {
//         Self {
//             inner: value,
//             pre: None,
//             post: None,
//             _pd: PhantomData,
//         }
//     }
// }

// #[cfg(test)]
// mod test {
//     use super::Middleware;

//     fn _compile_check<N>() -> Box<dyn Middleware<N>> {
//         todo!()
//     }
// }
