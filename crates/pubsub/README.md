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
identified locally by a `B256`. This value is a params-derived alias: it is the
Keccak-256 hash of the serialized parameters and does not include the method.
Requests with identical parameters therefore share an alias, including custom
subscription methods with different names. The service uses this ID to manage
the subscription lifecycle and dispatch notifications to receivers.

### Server & Local IDs

When a user issues a subscription request, the **frontend** sends a
subscription request to the **service**. The **service** dispatches it to the
RPC server via the **backend**. The **service** then intercepts the RPC server
response containing the server ID and assigns a stable `B256` `local_id` to
the subscription. Server IDs are represented by [`alloy_json_rpc::SubId`] and
may be numeric or strings. The `local_id` identifies the subscription in the
**service** and consumer tasks, while the current `server_id` associates
notifications with the server-side subscription.

This allows the service to keep the consumer-facing `local_id` stable while the
server ID changes across reconnections.

### Reconnection, replay, and cancellation

After a retryable established-backend failure, the service attempts reconnection under its
configured retry policy; non-retryable failures terminate it immediately. Following a successful
reconnect, it re-sends every
request still awaiting a response and re-creates active subscriptions. If the
server processed a request but its response was lost, that request may execute
more than once. Do not rely on at-most-once execution for non-idempotent methods.

Dropping a request future stops waiting locally, but does not cancel a request
already accepted by the service. Likewise, dropping a [`RawSubscription`] or
[`Subscription`] only drops that receiver; it does not send
`eth_unsubscribe`. Call [`PubSubFrontend::unsubscribe`] with the local ID when
the server-side subscription is no longer needed. `unsubscribe` only queues
the instruction and does not wait for the server's response. It is best effort:
an unsubscribe processed while reconnection is re-creating the subscription can
race with the replacement response, so it is not confirmation of server-side
teardown.

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

### Subscription Lifecycle

Regular Request Lifecycle

1. The user issues a request to the **frontend**.
1. The **frontend** sends the request to the **service**, with a oneshot channel
   to receive the response.
1. The **service** stores the oneshot channel in its `RequestManager`.
1. The **service** sends the request to the **backend**.
1. The **backend** sends the request to the RPC server.
1. The RPC server responds with a JSON RPC response.
1. The **backend** sends the response to the **service**.
1. The **service** sends the response to the waiting task via the oneshot.

Subscription Request Lifecycle:

1. The user issues a subscription request to the **frontend**.
1. The **frontend** sends the request to the **service**, with a oneshot channel
   to receive the response.
1. The **service** stores the oneshot channel in its `RequestManager`.
1. The **service** sends the request to the **backend**.
1. The **backend** sends the request to the RPC server.
1. The RPC server responds with a numeric or string `server_id`.
1. The **backend** sends the response to the **service**.
1. The **service** assigns a `local_id` to the subscription, creates a
   subscription broadcast channel, and stores the relevant information in its
   `SubscriptionManager`.
1. The **service** overwrites the JSON RPC response with the `local_id`.
1. The **service** sends the response to the waiting task via the oneshot.

Subscription Notification Lifecycle

1. The RPC server sends a notification to the **backend**.
1. The **backend** sends the notification to the **service**.
1. The **service** maps the notification's `server_id` to its stable
   `local_id`.
1. If present, the **service** sends the notification to the relevant channel.
   1. Otherwise, the **service** ignores the notification.
