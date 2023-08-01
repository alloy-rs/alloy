mod call;
pub use call::RpcCall;

mod common;
pub use common::Authorization;

pub mod client;
pub use client::RpcClient;

mod error;
pub use error::TransportError;

pub(crate) mod utils;

mod batch;
pub use batch::BatchRequest;

mod transports;
pub use transports::{Http, Transport};

pub use alloy_json_rpc::RpcResult;

#[cfg(test)]
mod test {
    use tower::util::BoxCloneService;

    use super::*;

    fn box_clone_transport() -> BoxCloneService<
        Box<serde_json::value::RawValue>,
        Box<serde_json::value::RawValue>,
        TransportError,
    > {
        BoxCloneService::new(Http::new("http://localhost:8545".parse().unwrap()))
    }
}
