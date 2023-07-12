#![warn(
    missing_debug_implementations,
    // missing_docs,
    unreachable_pub,
    // unused_crate_dependencies
)]
#![deny(unused_must_use, rust_2018_idioms)]
#![doc(test(
    no_crate_inject,
    attr(deny(warnings, rust_2018_idioms), allow(dead_code, unused_variables))
))]

pub mod common;
pub(crate) mod utils;

mod error;
pub use error::{RpcResult, TransportError};

mod call;
pub use call::RpcCall;

mod connection;
pub use connection::{Connection, PubSubConnection, RpcParam, RpcResp};

pub mod transports;
pub use transports::Http;
