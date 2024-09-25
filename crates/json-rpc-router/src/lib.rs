//! JSON-RPC router inspired by axum's `Router`.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use alloy_json_rpc::{
    PartiallySerializedRequest, Request, RequestMeta, Response, ResponsePayload, RpcObject,
};
use core::fmt;
use serde_json::value::RawValue;
use std::{
    borrow::Cow,
    collections::BTreeMap,
    convert::Infallible,
    future::Future,
    marker::PhantomData,
    ops::{Add, AddAssign},
    pin::Pin,
    sync::Arc,
    task,
};
use tower::{util::BoxCloneService, Service};

/// A JSON-RPC handler for a specific method.
pub type Route<E = Infallible> = BoxCloneService<Box<RawValue>, ResponsePayload, E>;

/// A JSON-RPC router.
#[derive(Clone)]
pub struct Router<S> {
    inner: Arc<RouterInner<S>>,
}

impl<S> Router<S> {
    /// Call a method on the router.
    pub async fn call_with_state(&self, req: PartiallySerializedRequest, state: S) -> Response {
        let Request { meta: RequestMeta { method, id, .. }, params } = req;

        let payload = self.inner.call_with_state(method, params, state).await;
        Response { id, payload }
    }
}

impl Router<()> {
    /// Call a method on the router.
    pub async fn call(&self, req: PartiallySerializedRequest) -> Response {
        self.call_with_state(req, ()).await
    }
}

impl<S> fmt::Debug for Router<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Router").finish_non_exhaustive()
    }
}

/// A unique internal identifier for a method.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct MethodId(usize);

impl From<usize> for MethodId {
    fn from(id: usize) -> Self {
        Self(id)
    }
}

impl Add for MethodId {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Add<usize> for MethodId {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl AddAssign for MethodId {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl AddAssign<usize> for MethodId {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

/// A boxed, erased type that can be converted into a Handler. Similar to
/// axum's `ErasedIntoRoute`
///
/// Currently this is a placeholder to enable future convenience functions
pub trait ErasedIntoRoute<S>: Send {
    /// Take a reference to this type, clone it, box it, and type erase it.
    ///
    /// This allows it to be stored in a collection of `dyn
    /// ErasedIntoRoute<S>`.
    fn clone_box(&self) -> Box<dyn ErasedIntoRoute<S>>;

    /// Convert this type into a handler.
    fn into_route(self: Box<Self>, state: S) -> Route;

    /// Call this handler with the given state.
    #[allow(dead_code)]
    fn call_with_state(
        self: Box<Self>,
        params: Box<RawValue>,
        state: S,
    ) -> <Route as Service<Box<RawValue>>>::Future;
}

/// A boxed, erased type that can be converted into a handler.
///
/// Similar to axum's `BoxedIntoRoute`
struct BoxedIntoHandler<S>(Box<dyn ErasedIntoRoute<S>>);

impl<S> Clone for BoxedIntoHandler<S> {
    fn clone(&self) -> Self {
        Self(self.0.clone_box())
    }
}

/// A method, which may be ready to handle requests or may need to be
/// initialized with some state.
///
/// Analagous to axum's `MethodEndpoint`
enum Method<S> {
    /// A method that needs to be initialized with some state.
    Needs(BoxedIntoHandler<S>),
    /// A method that is ready to handle requests.
    Ready(Route),
}

/// The inner state of a [`Router`]. Maps methods to their handlers.
#[derive(Default)]
pub struct RouterInner<S> {
    routes: BTreeMap<MethodId, Method<S>>,

    last_id: MethodId,
    name_to_id: BTreeMap<Cow<'static, str>, MethodId>,
    id_to_name: BTreeMap<MethodId, Cow<'static, str>>,
}

impl<S> fmt::Debug for RouterInner<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouterInner").finish_non_exhaustive()
    }
}

impl<S> RouterInner<S> {
    /// Create a new, empty router.
    pub fn new() -> Self {
        Self {
            routes: BTreeMap::new(),
            last_id: Default::default(),
            name_to_id: BTreeMap::new(),
            id_to_name: BTreeMap::new(),
        }
    }

    /// Get the next available ID.
    fn get_id(&mut self) -> MethodId {
        self.last_id += 1;
        self.last_id
    }

    /// Get a method by its name.
    fn method_by_name(&self, name: &str) -> Option<&Method<S>> {
        self.name_to_id.get(name).and_then(|id| self.routes.get(id))
    }

    fn enroll_method_name(&mut self, method: Cow<'static, str>) -> MethodId {
        if self.name_to_id.contains_key(&method) {
            panic!("Method name already exists in the router.");
        }

        let id = self.get_id();
        self.name_to_id.insert(method.clone(), id);
        self.id_to_name.insert(id, method.clone());
        id
    }

    /// Add a method to the router. This method may be missing state `S`.
    pub fn add_into_route<H>(mut self, method: impl Into<Cow<'static, str>>, handler: H) -> Self
    where
        H: ErasedIntoRoute<S>,
    {
        let method = method.into();
        let handler = handler.clone_box();

        add_method_inner(&mut self, method, handler);

        fn add_method_inner<S>(
            this: &mut RouterInner<S>,
            method: Cow<'static, str>,
            handler: Box<dyn ErasedIntoRoute<S>>,
        ) {
            let id = this.enroll_method_name(method);

            this.routes.insert(id, Method::Needs(BoxedIntoHandler(handler)));
        }

        self
    }

    /// Add a handler to the router. This method is complete and ready to call.
    pub fn add_route(mut self, method: impl Into<Cow<'static, str>>, handler: Route) -> Self {
        let method = method.into();
        let id = self.get_id();

        self.name_to_id.insert(method.clone(), id);
        self.id_to_name.insert(id, method.clone());
        self.routes.insert(id, Method::Ready(handler));

        self
    }

    /// Add a service to the router.
    pub fn route_service<T>(self, method: impl Into<Cow<'static, str>>, service: T) -> Self
    where
        T: Service<
                Box<RawValue>,
                Response = ResponsePayload,
                Error = Infallible,
                Future: Send + 'static,
            > + Clone
            + Send
            + 'static,
    {
        self.add_route(method, BoxCloneService::new(service))
    }

    /// Call a method on the router, with the provided state.
    fn call_with_state(
        &self,
        method: impl Into<Cow<'static, str>>,
        params: Box<RawValue>,
        state: S,
    ) -> impl Future<Output = ResponsePayload> + Captures<'_> {
        let method = method.into();
        let method =
            self.method_by_name(method.as_ref()).ok_or_else(ResponsePayload::method_not_found);

        async move {
            match method {
                Err(err) => return err,
                Ok(method) => match method {
                    Method::Needs(handler) => {
                        let h = handler.clone();
                        h.0.into_route(state).call(params)
                    }
                    Method::Ready(handler) => handler.clone().call(params),
                },
            }
            .await
            .unwrap()
        }
    }
}

trait Captures<'a> {}
impl<'a, T: ?Sized> Captures<'a> for T {}

/// A handler for a JSON-RPC method.
pub trait Handler<T, S>: Clone + Send + Sized + 'static {
    /// The future returned by the handler.
    type Future: Future<Output = ResponsePayload> + Send + 'static;

    /// Call the handler with the given request and state.
    fn call(self, req: Box<RawValue>, state: S) -> Self::Future;

    /// Create a new handler that wraps this handler and has some state.
    fn with_state(self, state: S) -> HandlerService<Self, T, S> {
        HandlerService::new(self, state)
    }
}

/// A handler with some state.
#[derive(Debug)]
pub struct HandlerService<H, T, S> {
    handler: H,
    state: S,
    _marker: std::marker::PhantomData<T>,
}

impl<H, T, S> Clone for HandlerService<H, T, S>
where
    H: Clone,
    S: Clone,
{
    fn clone(&self) -> Self {
        Self { handler: self.handler.clone(), state: self.state.clone(), _marker: PhantomData }
    }
}

impl<H, T, S> HandlerService<H, T, S> {
    /// Create a new handler service.
    pub const fn new(handler: H, state: S) -> Self {
        Self { handler, state, _marker: PhantomData }
    }
}

impl<H, T, S> tower::Service<Box<RawValue>> for HandlerService<H, T, S>
where
    Self: Clone,
    H: Handler<T, S>,
    T: Send + 'static,
    S: Clone + Send + 'static,
{
    type Response = ResponsePayload;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<ResponsePayload, Infallible>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: Box<RawValue>) -> Self::Future {
        let this = self.clone();
        Box::pin(async move { Ok(this.handler.call(request, this.state.clone()).await) })
    }
}

impl<F, Fut, Params, Payload, ErrData, S> Handler<(Params,), S> for F
where
    F: FnOnce(Params) -> Fut + Clone + Send + 'static,
    Fut: Future<Output = ResponsePayload<Payload, ErrData>> + Send + 'static,
    Params: RpcObject,
    Payload: RpcObject,
    ErrData: RpcObject,
{
    type Future = Pin<Box<dyn Future<Output = ResponsePayload> + Send>>;

    fn call(self, req: Box<RawValue>, _state: S) -> Self::Future {
        Box::pin(async move {
            let Ok(params) = serde_json::from_str(req.get()) else {
                return ResponsePayload::invalid_params();
            };

            self(params)
                .await
                .serialize_payload()
                .ok()
                .unwrap_or_else(ResponsePayload::internal_error)
        })
    }
}

impl<F, Fut, Params, Payload, ErrData, S> Handler<(Params, S), S> for F
where
    F: FnOnce(Params, S) -> Fut + Clone + Send + 'static,
    Fut: Future<Output = ResponsePayload<Payload, ErrData>> + Send + 'static,
    Params: RpcObject,
    Payload: RpcObject,
    ErrData: RpcObject,
    S: Send + Sync + 'static,
{
    type Future = Pin<Box<dyn Future<Output = ResponsePayload> + Send>>;

    fn call(self, req: Box<RawValue>, state: S) -> Self::Future {
        Box::pin(async move {
            let Ok(params) = serde_json::from_str(req.get()) else {
                return ResponsePayload::invalid_params();
            };

            self(params, state).await.serialize_payload().ok().unwrap_or_else(|| {
                ResponsePayload::internal_error_message("Failed to serialize response".into())
            })
        })
    }
}

#[cfg(test)]
mod test {
    use alloy_json_rpc::ErrorPayload;

    // more of an example really
    use super::*;

    #[tokio::test]
    async fn example() {
        let router: RouterInner<()> = RouterInner::new().route_service(
            "hello_world",
            tower::service_fn(|_: Box<RawValue>| async {
                Ok(ResponsePayload::<(), u8>::internal_error_with_message_and_obj(
                    Cow::Borrowed("Hello, world!"),
                    30u8,
                )
                .serialize_payload()
                .unwrap())
            }),
        );

        let res = router
            .call_with_state("hello_world", Default::default(), ())
            .await
            .deserialize_error()
            .unwrap();

        assert!(matches!(
            res,
            ResponsePayload::Failure(ErrorPayload {
                code: -32603,
                message: Cow::Borrowed("Hello, world!"),
                data: Some(30u8)
            })
        ),);
    }
}
