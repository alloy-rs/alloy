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

//! # alloy-pubsub.
//!
//! ### Overview
//!
//! PubSub services, unlike regular RPC services, are long-lived and
//! bidirectional. They are used to subscribe to events on the server, and
//! receive notifications when those events occur.
//!
//! The PubSub system here consists of 3 logical parts:
//! - The **frontend** is the part of the system that the user interacts with. It exposes a simple
//!   API that allows the user to issue requests and manage subscriptions.
//! - The **service** is an intermediate layer that manages request/response mappings, subscription
//!   aliasing, and backend lifecycle events. Running [`PubSubConnect::into_service`] will spawn a
//!   long-lived service task.
//! - The **backend** is an actively running connection to the server. Users should NEVER
//!   instantiate a backend directly. Instead, they should use [`PubSubConnect::into_service`] for
//!   some connection object.
//!
//! This module provides the following:
//!
//! - [PubSubConnect]: A trait for instantiating a PubSub service by connecting to some **backend**.
//!   Implementors of this trait are responsible for the precise connection details, and for
//!   spawning the **backend** task. Users should ALWAYS call [`PubSubConnect::into_service`] to get
//!   a running service with a running backend.
//! - [`ConnectionHandle`]: A handle to a running **backend**. This type is returned by
//!   [PubSubConnect::connect], and owned by the **service**. Dropping the handle will shut down the
//!   **backend**.
//! - [`ConnectionInterface`]: The reciprocal of [ConnectionHandle]. This type is owned by the
//!   **backend**, and is used to communicate with the **service**. Dropping the interface will
//!   notify the **service** of a terminal error.
//! - [`PubSubFrontend`]: The **frontend**. A handle to a running PubSub **service**. It is used to
//!   issue requests and subscription lifecycle instructions to the **service**.

mod connect;
pub use connect::PubSubConnect;

mod frontend;
pub use frontend::PubSubFrontend;

mod ix;

mod handle;
pub use handle::{ConnectionHandle, ConnectionInterface};

mod managers;

mod service;
