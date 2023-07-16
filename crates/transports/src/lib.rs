mod call;
pub use call::RpcCall;

mod common;
pub use common::Authorization;

pub mod connection;
pub use connection::RpcClient;

mod error;
pub use error::TransportError;

pub(crate) mod utils;

mod batch;
pub use batch::BatchRequest;

mod transports;
pub use transports::Http;

pub use alloy_json_rpc::RpcResult;
