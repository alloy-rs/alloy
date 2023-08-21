mod call;
pub use call::RpcCall;

mod common;
pub use common::Authorization;

/// [`RpcClient`] and [`ClientBuilder`].
pub mod client;
pub use client::{ClientBuilder, RpcClient};

mod error;
pub use error::TransportError;

mod batch;
pub use batch::BatchRequest;

mod transports;
pub use transports::{BoxTransport, Http, Transport};

pub use alloy_json_rpc::RpcResult;

pub(crate) mod utils;
