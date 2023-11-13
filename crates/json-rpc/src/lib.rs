//! Alloy JSON-RPC data types.
//!
//! This crate provides data types for use with the JSON-RPC 2.0 protocol. It
//! does not provide any functionality for actually sending or receiving
//! JSON-RPC data.
//!
//! If you find yourself importing this crate, and you are not implementing a
//! JSON-RPC client or transport, you are likely at the wrong layer of
//! abstraction. If you want to _use_ a JSON-RPC client, consider using the
//! [`alloy-transports`] crate.
//!
//! [`alloy-transports`]: https://docs.rs/alloy-transports/latest/alloy-transports
//!
//! ## Note On (De)Serialization
//!
//! [`Request`], [`Response`], and similar types are generic over the
//! actual data being passed to and from the RPC. We can achieve partial
//! (de)serialization by making them generic over a `serde_json::RawValue`.
//!
//! - For [`Request`] - [`PartiallySerializedRequest`] is a `Request<Box<RawValue>`. It represents a
//!   `Request` whose parameters have been serialized. [`SerializedRequest`], on the other hand is a
//!   request that has been totally serialized. For client-development purposes, its [`Id`] and
//!   method have been preserved.
//! - For [`Response`] - [`BorrowedResponse`] is a `Response<&RawValue>`. It represents a Response
//!   whose [`Id`] and return status (success or failure) have been deserialized, but whose payload
//!   has not.
//!
//! Allowing partial serialization lets us include many unlike [`Request`]
//! objects in collections (e.g. in a batch request). This is useful for
//! implementing a client.
//!
//! Allowing partial deserialization lets learn request status, and associate
//! the raw response data with the corresponding client request before doing
//! full deserialization work. This is useful for implementing a client.
//!
//! In general, partially deserialized responses can be further deserialized.
//! E.g. an [`BorrowedRpcResult`] may have success responses deserialized
//! with [`RpcResult::deserialize_success::<U>`], which will transform it to an
//! [`RpcResult<U>`]. Or the caller may use [`RpcResult::try_success_as::<U>`]
//! to attempt to deserialize without transforming the [`RpcResult`].

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![warn(
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    clippy::missing_const_for_fn,
    rustdoc::all
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use serde::{de::DeserializeOwned, Serialize};

mod notification;
pub use notification::{EthNotification, PubSubItem};

mod packet;
pub use packet::{BorrowedResponsePacket, RequestPacket, ResponsePacket};

mod request;
pub use request::{PartiallySerializedRequest, Request, RequestMeta, SerializedRequest};

mod response;
pub use response::{
    BorrowedErrorPayload, BorrowedResponse, BorrowedResponsePayload, ErrorPayload, Response,
    ResponsePayload,
};

mod common;
pub use common::Id;

mod result;
pub use result::{BorrowedRpcResult, RpcResult};

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
