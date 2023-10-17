//! Alloy JSON-RPC data types.
//!
//! This crate provides data types for use with the JSON-RPC 2.0 protocol. It
//! does not provide any functionality for actually sending or receiving
//! JSON-RPC data.

mod notification;
pub use notification::{EthNotification, PubSubItem};

mod packet;
pub use packet::{BorrowedResponsePacket, RequestPacket, ResponsePacket};

mod request;
pub use request::Request;

mod response;
pub use response::{
    BorrowedErrorPayload, BorrowedResponse, BorrowedResponsePayload, ErrorPayload, Response,
    ResponsePayload,
};

mod common;
pub use common::Id;

mod result;
pub use result::RpcResult;

use serde::{de::DeserializeOwned, Serialize};

/// An object that can be used as a JSON-RPC parameter.
///
/// This marker trait is blanket-implemented for every qualifying type. It is
/// used to indicate that a type can be used as a JSON-RPC parameter.
pub trait RpcParam: Serialize + Clone + Send + Sync + Unpin {}
impl<T> RpcParam for T where T: Serialize + Clone + Send + Sync + Unpin {}

/// An object that can be used as a JSON-RPC return value.
///
/// This marker trait is blanket-implemented for every qualifying type. It is
/// used to indicate that a type can be used as a JSON-RPC return value.
///
/// # Note
///
/// We add the `'static` lifetime bound to indicate that the type can't borrow.
/// This is a simplification that makes it easier to use the types in client
/// code. It is not suitable for use in server code.
pub trait RpcReturn: DeserializeOwned + Send + Sync + Unpin + 'static {}
impl<T> RpcReturn for T where T: DeserializeOwned + Send + Sync + Unpin + 'static {}

/// An object that can be used as a JSON-RPC parameter and return value.
///
/// This marker trait is blanket-implemented for every qualifying type. It is
/// used to indicate that a type can be used as both a JSON-RPC parameter and
/// return value.
pub trait RpcObject: RpcParam + RpcReturn {}
impl<T> RpcObject for T where T: RpcParam + RpcReturn {}
