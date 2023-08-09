use serde::{de::DeserializeOwned, Serialize};

mod request;
pub use request::JsonRpcRequest;

mod response;
pub use response::{ErrorPayload, JsonRpcResponse, ResponsePayload};

mod common;
pub use common::Id;

mod result;
pub use result::RpcResult;

pub trait RpcParam: Serialize + Send + Sync + Unpin {}
impl<T> RpcParam for T where T: Serialize + Send + Sync + Unpin {}

// Note: we add `'static` here to indicate that the Resp is wholly owned. it
// may not borrow.
pub trait RpcReturn: DeserializeOwned + Send + Sync + Unpin + 'static {}
impl<T> RpcReturn for T where T: DeserializeOwned + Send + Sync + Unpin + 'static {}

pub trait RpcObject: RpcParam + RpcReturn {}
impl<T> RpcObject for T where T: RpcParam + RpcReturn {}
