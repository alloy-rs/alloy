//! Builder for [EIP-4361] messages.
//!
//! [EIP-4361]: https://eips.ethereum.org/EIPS/eip-4361

use crate::{Message, TimeStamp, Version};
use alloc::{string::String, vec::Vec};
use alloy_primitives::Address;
use http::uri::Authority;
use iri_string::types::UriString;

/// Optional fields that don't participate in the typestate.
#[derive(Clone, Debug, Default)]
struct Options {
    scheme: Option<String>,
    statement: Option<String>,
    version: Version,
    expiration_time: Option<TimeStamp>,
    not_before: Option<TimeStamp>,
    request_id: Option<String>,
    resources: Vec<UriString>,
}

/// Builder for constructing [EIP-4361] messages.
///
/// Uses a typestate pattern to ensure all required fields are set at compile time.
/// The type parameters hold either `()` (not set) or the actual value type (set).
///
/// [EIP-4361]: https://eips.ethereum.org/EIPS/eip-4361
#[derive(Clone, Debug)]
pub struct MessageBuilder<D, A, U, C, N, I> {
    options: Options,
    domain: D,
    address: A,
    uri: U,
    chain_id: C,
    nonce: N,
    issued_at: I,
}

impl Default for MessageBuilder<(), (), (), (), (), ()> {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageBuilder<(), (), (), (), (), ()> {
    /// Creates a new builder with default values.
    ///
    /// Defaults to [`Version::V1`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            options: Options::default(),
            domain: (),
            address: (),
            uri: (),
            chain_id: (),
            nonce: (),
            issued_at: (),
        }
    }
}

impl<D, A, U, C, N, I> MessageBuilder<D, A, U, C, N, I> {
    /// Sets the URI scheme (e.g., "https").
    #[must_use]
    pub fn scheme(mut self, scheme: impl Into<String>) -> Self {
        self.options.scheme = Some(scheme.into());
        self
    }

    /// Sets the human-readable statement.
    #[must_use]
    pub fn statement(mut self, statement: impl Into<String>) -> Self {
        self.options.statement = Some(statement.into());
        self
    }

    /// Sets the message version (defaults to [`Version::V1`]).
    #[must_use]
    pub const fn version(mut self, version: Version) -> Self {
        self.options.version = version;
        self
    }

    /// Sets when the message expires.
    #[must_use]
    pub fn expiration_time(mut self, expiration_time: TimeStamp) -> Self {
        self.options.expiration_time = Some(expiration_time);
        self
    }

    /// Sets when the message becomes valid.
    #[must_use]
    pub fn not_before(mut self, not_before: TimeStamp) -> Self {
        self.options.not_before = Some(not_before);
        self
    }

    /// Sets the request identifier.
    #[must_use]
    pub fn request_id(mut self, request_id: impl Into<String>) -> Self {
        self.options.request_id = Some(request_id.into());
        self
    }

    /// Adds a resource URI.
    #[must_use]
    pub fn resource(mut self, resource: UriString) -> Self {
        self.options.resources.push(resource);
        self
    }

    /// Adds multiple resource URIs.
    #[must_use]
    pub fn resources(mut self, resources: impl IntoIterator<Item = UriString>) -> Self {
        self.options.resources.extend(resources);
        self
    }

    /// Sets the domain requesting the signing.
    #[must_use]
    pub fn domain(self, domain: Authority) -> MessageBuilder<Authority, A, U, C, N, I> {
        MessageBuilder {
            options: self.options,
            domain,
            address: self.address,
            uri: self.uri,
            chain_id: self.chain_id,
            nonce: self.nonce,
            issued_at: self.issued_at,
        }
    }

    /// Sets the Ethereum address performing the signing.
    #[must_use]
    pub fn address(self, address: Address) -> MessageBuilder<D, Address, U, C, N, I> {
        MessageBuilder {
            options: self.options,
            domain: self.domain,
            address,
            uri: self.uri,
            chain_id: self.chain_id,
            nonce: self.nonce,
            issued_at: self.issued_at,
        }
    }

    /// Sets the URI of the resource.
    #[must_use]
    pub fn uri(self, uri: UriString) -> MessageBuilder<D, A, UriString, C, N, I> {
        MessageBuilder {
            options: self.options,
            domain: self.domain,
            address: self.address,
            uri,
            chain_id: self.chain_id,
            nonce: self.nonce,
            issued_at: self.issued_at,
        }
    }

    /// Sets the chain ID.
    #[must_use]
    pub fn chain_id(self, chain_id: u64) -> MessageBuilder<D, A, U, u64, N, I> {
        MessageBuilder {
            options: self.options,
            domain: self.domain,
            address: self.address,
            uri: self.uri,
            chain_id,
            nonce: self.nonce,
            issued_at: self.issued_at,
        }
    }

    /// Sets the nonce for replay protection.
    #[must_use]
    pub fn nonce(self, nonce: impl Into<String>) -> MessageBuilder<D, A, U, C, String, I> {
        MessageBuilder {
            options: self.options,
            domain: self.domain,
            address: self.address,
            uri: self.uri,
            chain_id: self.chain_id,
            nonce: nonce.into(),
            issued_at: self.issued_at,
        }
    }

    /// Sets when the message was created.
    #[must_use]
    pub fn issued_at(self, issued_at: TimeStamp) -> MessageBuilder<D, A, U, C, N, TimeStamp> {
        MessageBuilder {
            options: self.options,
            domain: self.domain,
            address: self.address,
            uri: self.uri,
            chain_id: self.chain_id,
            nonce: self.nonce,
            issued_at,
        }
    }
}

impl MessageBuilder<Authority, Address, UriString, u64, String, TimeStamp> {
    /// Builds the [`Message`].
    ///
    /// This method is only available when all required fields have been set.
    #[must_use]
    pub fn build(self) -> Message {
        Message {
            scheme: self.options.scheme,
            domain: self.domain,
            address: self.address,
            statement: self.options.statement,
            uri: self.uri,
            version: self.options.version,
            chain_id: self.chain_id,
            nonce: self.nonce,
            issued_at: self.issued_at,
            expiration_time: self.options.expiration_time,
            not_before: self.options.not_before,
            request_id: self.options.request_id,
            resources: self.options.resources,
        }
    }
}
