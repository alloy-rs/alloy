# alloy-pubsub

Ethereum JSON-RPC [publish-subscribe] tower service and type definitions.

[publish-subscribe]: https://en.wikipedia.org/wiki/Publish%E2%80%93subscribe_pattern

## Overview

PubSub services, unlike regular RPC services, are long-lived and
bidirectional. They are used to subscribe to events on the server, and
receive notifications when those events occur.

The PubSub system here consists of 3 logical parts:

- The **frontend** is the part of the system that the user interacts with.
  It exposes a simple API that allows the user to issue requests and manage
  subscriptions.
- The **service** is an intermediate layer that manages request/response
  mappings, subscription aliasing, and backend lifecycle events. Running
  [`PubSubConnect::into_service`] will spawn a long-lived service task. The
  service exists to manage the lifecycle of requests and subscriptions over
  reconnections, and to serve any number of **frontends**.
- The **backend** is an actively running connection to the server. Users
  should NEVER instantiate a backend directly. Instead, they should use
  [`PubSubConnect::into_service`] for some connection object. Backends
  are responsible for managing the connection to the server, accepting user
  requests from the **service** and forwarding server responses to the
  **service**.

This crate provides the following:

- [`PubSubConnect`]: A trait for instantiating a PubSub service by connecting
  to some **backend**. Implementers of this trait are responsible for
  the precise connection details, and for spawning the **backend** task.
  Users should ALWAYS call [`PubSubConnect::into_service`] to get a running
  service with a running backend.
- [`ConnectionHandle`]: A handle to a running **backend**. This type is
  returned by [PubSubConnect::connect], and owned by the **service**.
  Dropping the handle will shut down the **backend**.
- [`ConnectionInterface`]: The reciprocal of [ConnectionHandle]. This type
  is owned by the **backend**, and is used to communicate with the
  **service**. Dropping the interface will notify the **service** of a
  terminal error.
- [`PubSubFrontend`]: The **frontend**. A handle to a running PubSub
  **service**. It is used to issue requests and subscription lifecycle
  instructions to the **service**.
- [`RawSubscription`]: A handle to a subscription. This type is yielded by
  the **service** when a user issues a `get_subscription()` request. It is a
  `tokio::broadcast` channel which receives notifications from the **service**
  when the server sends a notification for the subscription.
- [`Subscription`]: A handle to a subscription expecting a specific response
  type. A wrapper around [`RawSubscription`] that deserializes notifications
  into the expected type, and allows the user to accept or discard unexpected
  responses.
- [`SubscriptionItem`]: An item in a typed [`Subscription`]. This type is
  yielded by the subscription via the `recv_any()` API. It contains the
  deserialized item, or the raw JSON value if deserialization fails.

## On Handling Subscriptions

For a normal request, the user sends a request to the **frontend**, and
later receives a response via a tokio oneshot channel. This is straightforward
and easy to reason about. Subscriptions, however, are side-effects of other
requests, and are long-lived. They are managed by the **service** and
identified locally by a `B256`. Each subscription request is answered with its
own local ID, which is never reused. Identical subscription requests, with the
same method and serialized parameters, share a single server-side subscription:
a request for a subscription that exists, or is being created, is answered
without another request to the server. The service uses these IDs to manage
the subscription lifecycle and dispatch notifications to receivers.

### Server & Local IDs

When a user issues a subscription request, the **frontend** sends a
subscription request to the **service**. The **service** dispatches it to the
RPC server via the **backend**. The **service** then intercepts the RPC server
response containing the server ID and answers the caller with a stable `B256`
`local_id`. Server IDs are represented by [`alloy_json_rpc::SubId`] and
may be numeric or strings. The `local_id` identifies the subscription in the
**service** and consumer tasks, while the current `server_id` associates
notifications with the server-side subscription.

This allows the service to keep the consumer-facing `local_id` stable while the
server ID changes across reconnections.

### Reconnection, replay, and cancellation

After a retryable established-backend failure, the service attempts reconnection under its
configured retry policy; non-retryable failures terminate it immediately. A backend that dies
with requests pending, without answering any of them since it connected, is unhealthy even if
connecting succeeded; notifications don't count as answers. From the second consecutive
unhealthy backend on, the service backs off before reconnecting, and after `max_retries` of
them it shuts down. A request that was pending on four consecutive dead backends fails instead
of being re-sent again, and a subscription whose resubscription four consecutive backends did
not acknowledge, although they answered other requests, is dropped, which closes its receivers.
A subscription whose resubscription the server rejects, or answers with something other than a
subscription ID, is dropped the same way; an identical subscribe request creates it afresh.

Following a successful reconnect, it re-sends every
request still awaiting a response and re-creates active subscriptions that still
have consumers: a receiver, or a caller that was handed a local ID but has not
fetched a receiver yet. If the
server processed a request but its response was lost, that request may execute
more than once. Do not rely on at-most-once execution for non-idempotent methods.

Dropping a request future stops waiting locally, but does not cancel a request
already accepted by the service. Likewise, dropping a [`RawSubscription`] or
[`Subscription`] only drops that receiver; it does not cancel the server-side
subscription. Call [`PubSubFrontend::unsubscribe`] with the local ID when the
subscription is no longer needed. This closes the receivers fetched with that
local ID. Identical subscriptions share the server-side subscription, so it is
cancelled once no other local ID of it has a receiver or has yet to fetch one.
Cancelling it sends
`{namespace}_unsubscribe`, with the namespace of the subscribe method, e.g.
`eth_unsubscribe`, `{namespace}_unsubscribe{Name}` for methods named
`{namespace}_subscribe{Name}`, and `{method}_unsubscribe` for other methods,
e.g. `admin_peerEvents_unsubscribe`. `unsubscribe` only queues the
instruction and does not wait for the server's response.

The service also cancels server-side subscriptions it would not route: those
acknowledged after their caller stopped waiting, or after an unsubscribe while
reconnection was re-creating them, and duplicates of a subscription that is
already routed.

### What is a subscription request?

The **service** uses the `is_subscription()` method in the request to determine
whether a given RPC request is a subscription. In general, subscription requests
use the `eth_subscribe` method. However, other methods may also be used to
create subscriptions, such as `admin_peerEvents`. To allow custom subscriptions
on unknown methods, the `Request`, `SerializedRequest` and `RpcCall` expose
`set_is_subscription()`, which can be used to mark any given request as a
subscription.

When marking a request as a subscription, the **service** will intercept the
RPC response, which must deserialize as an [`alloy_json_rpc::SubId`] (a numeric
or string ID). Other response types will fail deserialization.
