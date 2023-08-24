//! Alloy JSON-RPC data types.
//!
//! This crate provides data types for use with the JSON-RPC 2.0 protocol. It
//! does not provide any functionality for actually sending or receiving
//! JSON-RPC data.
//!
//! This crate is aimed at simplifying client implementations. It is not
//! well-suited to in-server applications. We do not support borrowing data from
//! deserializers, for example. This choice prevents complex lifetime
//! propagation in user code, at the expense of copying data

use serde::{de::DeserializeOwned, Serialize};

mod request;
pub use request::JsonRpcRequest;

mod response;
pub use response::{ErrorPayload, JsonRpcResponse, ResponsePayload};

mod common;
pub use common::Id;

mod result;
pub use result::RpcResult;

/// An object that can be used as a JSON-RPC parameter.
pub trait RpcParam: Serialize + Clone + Send + Sync + Unpin {}
impl<T> RpcParam for T where T: Serialize + Clone + Send + Sync + Unpin {}

/// An object that can be used as a JSON-RPC return value.
// Note: we add `'static` here to indicate that the Resp is wholly owned. It
// may not borrow.
pub trait RpcReturn: DeserializeOwned + Send + Sync + Unpin + 'static {}
impl<T> RpcReturn for T where T: DeserializeOwned + Send + Sync + Unpin + 'static {}

/// An object that can be used as a JSON-RPC parameter and return value.
pub trait RpcObject: RpcParam + RpcReturn {}
impl<T> RpcObject for T where T: RpcParam + RpcReturn {}
