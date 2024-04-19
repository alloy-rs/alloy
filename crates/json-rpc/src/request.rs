use crate::{common::Id, RpcParam};
use alloy_primitives::{keccak256, B256};
use serde::{de::DeserializeOwned, ser::SerializeMap, Deserialize, Serialize};
use serde_json::value::RawValue;
use std::borrow::Cow;

/// `RequestMeta` contains the [`Id`] and method name of a request.
#[derive(Clone, Debug)]
pub struct RequestMeta {
    /// The method name.
    pub method: Cow<'static, str>,
    /// The request ID.
    pub id: Id,
    /// Whether the request is a subscription, other than `eth_subscribe`.
    is_subscription: bool,
}

impl RequestMeta {
    /// Create a new `RequestMeta`.
    pub const fn new(method: Cow<'static, str>, id: Id) -> Self {
        Self { method, id, is_subscription: false }
    }

    /// Returns `true` if the request is a subscription.
    pub fn is_subscription(&self) -> bool {
        self.is_subscription || self.method == "eth_subscribe"
    }

    /// Indicates that the request is a non-standard subscription (i.e. not
    /// "eth_subscribe").
    pub fn set_is_subscription(&mut self) {
        self.set_subscription_status(true);
    }

    /// Setter for `is_subscription`. Indicates to RPC clients that the request
    /// triggers a stream of notifications.
    pub fn set_subscription_status(&mut self, sub: bool) {
        self.is_subscription = sub;
    }
}

/// A JSON-RPC 2.0 request object.
///
/// This is a generic type that can be used to represent any JSON-RPC request.
/// The `Params` type parameter is used to represent the parameters of the
/// request, and the `method` field is used to represent the method name.
///
/// ### Note
///
/// The value of `method` should be known at compile time.
#[derive(Clone, Debug)]
pub struct Request<Params> {
    /// The request metadata (ID and method).
    pub meta: RequestMeta,
    /// The request parameters.
    pub params: Params,
}

impl<Params> Request<Params> {
    /// Create a new `Request`.
    pub fn new(method: impl Into<Cow<'static, str>>, id: Id, params: Params) -> Self {
        Self { meta: RequestMeta::new(method.into(), id), params }
    }

    /// Returns `true` if the request is a subscription.
    pub fn is_subscription(&self) -> bool {
        self.meta.is_subscription()
    }

    /// Indicates that the request is a non-standard subscription (i.e. not
    /// "eth_subscribe").
    pub fn set_is_subscription(&mut self) {
        self.meta.set_is_subscription()
    }

    /// Setter for `is_subscription`. Indicates to RPC clients that the request
    /// triggers a stream of notifications.
    pub fn set_subscription_status(&mut self, sub: bool) {
        self.meta.set_subscription_status(sub);
    }
}

/// A [`Request`] that has been partially serialized. The request parameters
/// have been serialized, and are represented as a boxed [`RawValue`]. This is
/// useful for collections containing many requests, as it erases the `Param`
/// type. It can be created with [`Request::box_params()`].
///
/// See the [top-level docs] for more info.
///
/// [top-level docs]: crate
pub type PartiallySerializedRequest = Request<Box<RawValue>>;

impl<Params> Request<Params>
where
    Params: RpcParam,
{
    /// Serialize the request parameters as a boxed [`RawValue`].
    ///
    /// # Panics
    ///
    /// If serialization of the params fails.
    pub fn box_params(self) -> PartiallySerializedRequest {
        Request { meta: self.meta, params: serde_json::value::to_raw_value(&self.params).unwrap() }
    }

    /// Serialize the request, including the request parameters.
    pub fn serialize(self) -> serde_json::Result<SerializedRequest> {
        let request = serde_json::value::to_raw_value(&self)?;
        Ok(SerializedRequest { meta: self.meta, request })
    }
}

impl<Params> Request<&Params>
where
    Params: Clone,
{
    /// Clone the request, including the request parameters.
    pub fn into_owned_params(self) -> Request<Params> {
        Request { meta: self.meta, params: self.params.clone() }
    }
}

impl<'a, Params> Request<Params>
where
    Params: AsRef<RawValue> + 'a,
{
    /// Attempt to deserialize the params.
    ///
    /// To borrow from the params via the deserializer, use
    /// [`Request::try_borrow_params_as`].
    ///
    /// # Returns
    /// - `Ok(T)` if the params can be deserialized as `T`
    /// - `Err(e)` if the params cannot be deserialized as `T`
    pub fn try_params_as<T: DeserializeOwned>(&self) -> serde_json::Result<T> {
        serde_json::from_str(self.params.as_ref().get())
    }

    /// Attempt to deserialize the params, borrowing from the params
    ///
    /// # Returns
    /// - `Ok(T)` if the params can be deserialized as `T`
    /// - `Err(e)` if the params cannot be deserialized as `T`
    pub fn try_borrow_params_as<T: Deserialize<'a>>(&'a self) -> serde_json::Result<T> {
        serde_json::from_str(self.params.as_ref().get())
    }
}

// manually implemented to avoid adding a type for the protocol-required
// `jsonrpc` field
impl<Params> Serialize for Request<Params>
where
    Params: RpcParam,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let sized_params = std::mem::size_of::<Params>() != 0;

        let mut map = serializer.serialize_map(Some(3 + sized_params as usize))?;
        map.serialize_entry("method", &self.meta.method[..])?;

        // Params may be omitted if it is 0-sized
        if sized_params {
            map.serialize_entry("params", &self.params)?;
        }

        map.serialize_entry("id", &self.meta.id)?;
        map.serialize_entry("jsonrpc", "2.0")?;
        map.end()
    }
}

/// A JSON-RPC 2.0 request object that has been serialized, with its [`Id`] and
/// method preserved.
///
/// This struct is used to represent a request that has been serialized, but
/// not yet sent. It is used by RPC clients to build batch requests and manage
/// in-flight requests.
#[derive(Clone, Debug)]
pub struct SerializedRequest {
    meta: RequestMeta,
    request: Box<RawValue>,
}

impl<Params> std::convert::TryFrom<Request<Params>> for SerializedRequest
where
    Params: RpcParam,
{
    type Error = serde_json::Error;

    fn try_from(value: Request<Params>) -> Result<Self, Self::Error> {
        value.serialize()
    }
}

impl SerializedRequest {
    /// Returns the request metadata (ID and Method).
    pub const fn meta(&self) -> &RequestMeta {
        &self.meta
    }

    /// Returns the request ID.
    pub const fn id(&self) -> &Id {
        &self.meta.id
    }

    /// Returns the request method.
    pub fn method(&self) -> &str {
        &self.meta.method
    }

    /// Mark the request as a non-standard subscription (i.e. not
    /// `eth_subscribe`)
    pub fn set_is_subscription(&mut self) {
        self.meta.set_is_subscription();
    }

    /// Returns `true` if the request is a subscription.
    pub fn is_subscription(&self) -> bool {
        self.meta.is_subscription()
    }

    /// Returns the serialized request.
    pub const fn serialized(&self) -> &RawValue {
        &self.request
    }

    /// Consume the serialized request, returning the underlying [`RawValue`].
    #[allow(clippy::missing_const_for_fn)] // erroneous lint
    pub fn into_serialized(self) -> Box<RawValue> {
        self.request
    }

    /// Consumes the serialized request, returning the underlying
    /// [`RequestMeta`] and the [`RawValue`].
    #[allow(clippy::missing_const_for_fn)] // erroneous lint
    pub fn decompose(self) -> (RequestMeta, Box<RawValue>) {
        (self.meta, self.request)
    }

    /// Take the serialized request, consuming the [`SerializedRequest`].
    #[allow(clippy::missing_const_for_fn)] // erroneous lint
    pub fn take_request(self) -> Box<RawValue> {
        self.request
    }

    /// Get a reference to the serialized request's params.
    ///
    /// This partially deserializes the request, and should be avoided if
    /// possible.
    pub fn params(&self) -> Option<&RawValue> {
        #[derive(Deserialize)]
        struct Req<'a> {
            #[serde(borrow)]
            params: Option<&'a RawValue>,
        }

        let req: Req<'_> = serde_json::from_str(self.request.get()).unwrap();
        req.params
    }

    /// Get the hash of the serialized request's params.
    ///
    /// This partially deserializes the request, and should be avoided if
    /// possible.
    pub fn params_hash(&self) -> B256 {
        if let Some(params) = self.params() {
            keccak256(params.get())
        } else {
            keccak256("")
        }
    }
}

impl Serialize for SerializedRequest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.request.serialize(serializer)
    }
}
