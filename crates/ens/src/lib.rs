#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! ENS Name resolving utilities.

mod utils;
pub use utils::{dns_encode, namehash, reverse_address, try_dns_encode, DnsEncodeError};

use alloy_primitives::{address, Address};
use std::str::FromStr;

/// ENS Universal Resolver address (`0xeEeEEEeE14D718C2B47D9923Deab1335E144EeEe`).
///
/// The primary entry-point for ENS name resolution. Supports wildcard resolvers
/// and CCIP Read (ERC-3668).
pub const UNIVERSAL_RESOLVER_ADDRESS: Address =
    address!("0xeEeEEEeE14D718C2B47D9923Deab1335E144EeEe");

/// ENS registry address (`0x00000000000C2E074eC69A0dFb2997BA6C7d2e1e`).
#[deprecated(
    note = "resolution now routes through the Universal Resolver; use `UNIVERSAL_RESOLVER_ADDRESS` and the Universal Resolver helpers"
)]
pub const ENS_ADDRESS: Address = address!("0x00000000000C2E074eC69A0dFb2997BA6C7d2e1e");

/// ENS registry domain for reverse records: `addr.reverse`.
#[deprecated(
    note = "reverse resolution now routes through the Universal Resolver; use `reverse_address` or `lookup_address`"
)]
pub const ENS_REVERSE_REGISTRAR_DOMAIN: &str = "addr.reverse";

/// Helpers for ENS multichain coin types.
///
/// Non-EVM chains use their static SLIP-0044 coin type according to ENSIP-9.
/// EVM-compatible chains follow ENSIP-11: `coinType = 0x80000000 | chainId`.
/// Use [`evm_chain`][coin_type::evm_chain] to derive an EVM coin type.
pub mod coin_type {
    /// Ethereum mainnet (SLIP-0044, coin type 60).
    pub const ETH: u64 = 60;

    /// Converts an EVM chain ID to its ENSIP-11 coin type.
    ///
    /// Chain ID `1` (Ethereum mainnet) returns [`ETH`] (`60`) per SLIP-0044.
    /// All other chains use `0x80000000 | chain_id`.
    ///
    /// Returns `None` for chain IDs of `0x80000000` and above: ENSIP-11 reserves
    /// a 31-bit chain ID space, and larger IDs would collide with the coin types
    /// of smaller ones (e.g. `0` and `0x80000000`).
    pub const fn evm_chain(chain_id: u64) -> Option<u64> {
        if chain_id == 1 {
            return Some(ETH);
        }
        if chain_id >= 0x8000_0000 {
            return None;
        }
        Some(0x8000_0000 | chain_id)
    }
}

#[cfg(feature = "contract")]
pub use contract::*;

#[cfg(feature = "provider")]
pub use provider::*;

/// An ENS name or Ethereum address.
///
/// [`FromStr`] first attempts to parse an address, then treats a string containing `.` as a name.
/// This is only a routing heuristic: it rejects dotless names and does not normalize or validate
/// ENS names. In contrast, converting from [`String`] always creates [`Name`](Self::Name).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NameOrAddress {
    /// An ENS name. The value must already be ENSIP-15 normalized; its format is not checked.
    Name(String),
    /// An Ethereum Address
    Address(Address),
}

impl NameOrAddress {
    /// Resolves a name to an Ethereum address, or returns an address unchanged without an RPC call.
    #[cfg(feature = "provider")]
    pub async fn resolve<N: alloy_provider::Network, P: alloy_provider::Provider<N>>(
        &self,
        provider: &P,
    ) -> Result<Address, EnsError> {
        match self {
            Self::Name(name) => provider.resolve_name(name).await,
            Self::Address(addr) => Ok(*addr),
        }
    }
}

impl From<String> for NameOrAddress {
    fn from(name: String) -> Self {
        Self::Name(name)
    }
}

impl From<&String> for NameOrAddress {
    fn from(name: &String) -> Self {
        Self::Name(name.clone())
    }
}

impl From<Address> for NameOrAddress {
    fn from(addr: Address) -> Self {
        Self::Address(addr)
    }
}

impl FromStr for NameOrAddress {
    type Err = <Address as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Address::from_str(s) {
            Ok(addr) => Ok(Self::Address(addr)),
            Err(err) => {
                if s.contains('.') {
                    Ok(Self::Name(s.to_string()))
                } else {
                    Err(err)
                }
            }
        }
    }
}

#[cfg(feature = "contract")]
mod contract {
    use alloy_sol_types::sol;

    sol! {
        /// ENS Registry contract.
        ///
        /// Deprecated: retained for backwards compatibility. Resolution now routes
        /// through the [`UniversalResolver`], which handles wildcard resolvers and
        /// CCIP Read.
        #[sol(rpc)]
        contract EnsRegistry {
            /// Returns the resolver for the specified node.
            function resolver(bytes32 node) view returns (address);

            /// returns the owner of this node
            function owner(bytes32 node) view returns (address);
        }

        /// ENS Reverse Registrar contract.
        ///
        /// Deprecated: retained for backwards compatibility. Reverse resolution now
        /// routes through the [`UniversalResolver`].
        #[sol(rpc)]
        contract ReverseRegistrar {}

        /// ENS Resolver interface (ENSIP-1).
        #[sol(rpc)]
        contract EnsResolver {
            /// Returns the Ethereum address associated with the specified node.
            function addr(bytes32 node) view returns (address);

            /// Returns the name associated with an ENS node, for reverse records.
            function name(bytes32 node) view returns (string);

            /// Returns the text record value for the specified key.
            function text(bytes32 node, string calldata key) view virtual returns (string memory);
        }

        /// ENS Multicoin Resolver interface (ENSIP-11).
        ///
        /// Provides multichain address resolution. Use with the Universal Resolver and
        /// coin types from ENSIP-9 or [`coin_type::evm_chain`][crate::coin_type::evm_chain].
        #[sol(rpc)]
        contract EnsMulticoinResolver {
            /// Returns the address for `node` on the chain identified by `coin_type`.
            ///
            /// The returned bytes are the raw address encoding for that coin type
            /// (e.g., 20 raw bytes for EVM chains, script bytes for Bitcoin).
            function addr(bytes32 node, uint256 coin_type) view returns (bytes memory);
        }

        /// ENS Universal Resolver (ENSIP-23).
        ///
        /// The single entry-point for all ENS resolution. Handles routing to wildcard
        /// resolvers and CCIP Read (ERC-3668).
        ///
        /// Spec: <https://docs.ens.domains/ensip/23>
        ///
        /// `resolve` keeps unnamed returns so downstream `resolveReturn._0` / `_1`
        /// still compile. Reverse uses the deployed ENSIP-23 ABI
        /// `reverse(bytes,uint256)`; the historical 1-arg `reverse(bytes reverseName)`
        /// binding did not match the chain and is not preserved.
        ///
        /// Provider-based ENS helpers handle `OffchainLookup` reverts using ERC-3668 and
        /// the ENSIP-21 batch gateway protocol.
        #[sol(rpc)]
        contract UniversalResolver {
            error ResolverNotFound(bytes name);
            error ResolverNotContract(bytes name, address resolver);
            error ReverseAddressMismatch(string primary, bytes primaryAddress);
            error UnsupportedResolverProfile(bytes4 selector);
            error ResolverError(bytes errorData);

            struct ResolverInfo {
                bytes name;
                uint256 offset;
                bytes32 node;
                address resolver;
                bool extended;
            }

            /// Like `findResolver`, but reverts with `ResolverNotFound` if no resolver exists.
            function requireResolver(bytes memory name) public view returns (ResolverInfo memory info);

            /// Returns the resolver for `name` (DNS wire-format) without performing resolution.
            ///
            /// Returns `(resolver, node, offset)` where `resolver` is the contract address,
            /// `node` is the namehash, and `offset` is the byte offset into `name` at which
            /// the resolver was found (for wildcard/parent resolution).
            function findResolver(bytes memory name) public view returns (address, bytes32, uint256);

            /// Resolves `name` (DNS wire-format) using the encoded `data` call.
            ///
            /// Returns the ABI-encoded result of the resolver call and the resolver address.
            /// Reverts with `OffchainLookup` when CCIP Read (ERC-3668) is required.
            function resolve(bytes calldata name, bytes calldata data) external view returns (bytes memory, address);

            /// Like `resolve`, but uses `gateways` for CCIP Read instead of the default.
            function resolveWithGateways(bytes calldata name, bytes calldata data, string[] memory gateways) public view returns (bytes memory, address);

            /// Reverse-resolves an address to its primary ENS name.
            ///
            /// `lookupAddress` is the raw byte encoding of the address.
            /// `coinType` specifies the chain (use [`coin_type::ETH`] for Ethereum).
            function reverse(bytes calldata lookupAddress, uint256 coinType) external view returns (string memory name, address resolver, address reverseResolver);

            /// Like `reverse`, but uses `gateways` for CCIP Read instead of the default.
            function reverseWithGateways(bytes calldata lookupAddress, uint256 coinType, string[] memory gateways) public view returns (string memory name, address resolver, address reverseResolver);
        }
    }

    /// Returns the resolver address when it is safe to call `addr`/`text`/`name` directly.
    ///
    /// Extended (ENSIP-10) and parent/wildcard resolvers must be queried through the
    /// Universal Resolver instead.
    pub(crate) fn direct_resolver_address(
        info: &UniversalResolver::ResolverInfo,
        name: &str,
    ) -> Result<alloy_primitives::Address, EnsError> {
        if info.extended || !info.offset.is_zero() {
            Err(EnsError::RequiresUniversalResolver(name.to_string()))
        } else {
            Ok(info.resolver)
        }
    }

    /// Error type for ENS resolution.
    ///
    /// New variants (`RequiresUniversalResolver`, `DnsEncode`, `InvalidResponse`) are
    /// an intentional semver break relative to Alloy 1.x `EnsError`.
    #[derive(Debug, thiserror::Error)]
    #[non_exhaustive]
    pub enum EnsError {
        /// Failed to get the resolver for this name.
        #[error("Failed to get ENS resolver: {0}")]
        Resolver(alloy_contract::Error),
        /// No resolver found for the given name.
        #[error("ENS resolver not found for name {0:?}")]
        ResolverNotFound(String),
        /// Direct resolver calls are unsafe for this name; use Universal Resolver helpers.
        ///
        /// Returned when the name resolves through an ENSIP-10 extended resolver or a
        /// parent/wildcard resolver. Calling `addr`/`text`/`name` on the raw resolver
        /// instance can return incorrect data — use `resolve_name`, `lookup_txt`, or
        /// related helpers instead.
        #[error(
            "ENS name {0:?} requires Universal Resolver resolution (extended or wildcard resolver)"
        )]
        RequiresUniversalResolver(String),
        /// Failed to get the reverse registrar from the ENS registry.
        #[deprecated(
            note = "only produced by the deprecated registry-based `get_reverse_registrar` helper"
        )]
        #[error("Failed to get reverse registrar from the ENS registry: {0}")]
        RevRegistrar(alloy_contract::Error),
        /// No reverse registrar found for `addr.reverse`.
        #[deprecated(
            note = "only produced by the deprecated registry-based `get_reverse_registrar` helper"
        )]
        #[error("ENS reverse registrar not found for addr.reverse")]
        ReverseRegistrarNotFound,
        /// Failed to perform a reverse lookup.
        #[error("Failed to lookup ENS name from an address: {0}")]
        Lookup(alloy_contract::Error),
        /// Failed to resolve ENS name to an address.
        #[error("Failed to resolve ENS name to an address: {0}")]
        Resolve(alloy_contract::Error),
        /// Failed to get a text record for an ENS name.
        #[error("Failed to resolve text record: {0}")]
        ResolveTxtRecord(alloy_contract::Error),
        /// Failed to DNS-encode the ENS name for the Universal Resolver.
        #[error("Failed to DNS-encode ENS name: {0}")]
        DnsEncode(#[from] crate::DnsEncodeError),
        /// Failed to decode the Universal Resolver response.
        #[error("Failed to decode Universal Resolver response")]
        InvalidResponse,
        /// Failed while performing a CCIP Read request.
        ///
        /// Constructed only for non-transport CCIP failures. Transport errors are
        /// mapped to [`EnsError::Resolve`], [`EnsError::Lookup`], or
        /// [`EnsError::ResolveTxtRecord`] via context-aware helpers — do not use
        /// `EnsError::from(CcipReadError)`.
        #[cfg(feature = "provider")]
        #[error("Failed to perform CCIP Read: {0}")]
        CcipRead(#[source] alloy_provider::CcipReadError),
    }
}

#[cfg(feature = "provider")]
mod provider {
    #[allow(deprecated)]
    use crate::{
        coin_type, namehash, reverse_address, try_dns_encode, EnsError, EnsMulticoinResolver,
        EnsRegistry, EnsResolver, EnsResolver::EnsResolverInstance,
        ReverseRegistrar::ReverseRegistrarInstance, UniversalResolver, ENS_ADDRESS,
        ENS_REVERSE_REGISTRAR_DOMAIN, UNIVERSAL_RESOLVER_ADDRESS,
    };
    use alloy_eips::BlockId;
    use alloy_primitives::{Address, Bytes, B256, U256};
    use alloy_provider::{CcipReadClient, CcipReadError, CcipReadGateway, Network, Provider};
    use alloy_sol_types::{SolCall, SolValue};

    /// Extension trait for ENS contract calls.
    #[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
    #[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
    pub trait ProviderEnsExt<N: alloy_provider::Network, P: Provider<N>> {
        /// Returns the resolver contract instance for the given ENS name.
        ///
        /// Determines the resolver address via the Universal Resolver.
        ///
        /// Returns [`EnsError::RequiresUniversalResolver`] when the name uses an
        /// ENSIP-10 extended resolver or a parent/wildcard resolver. In those cases
        /// direct `addr`/`text`/`name` calls on the resolver are not equivalent to
        /// Universal Resolver resolution — use [`Self::resolve_name`],
        /// [`Self::lookup_txt`], or the other helpers instead.
        async fn get_resolver_for_name(
            &self,
            name: &str,
        ) -> Result<EnsResolverInstance<&P, N>, EnsError>;

        /// Returns the resolver for the specified node.
        ///
        /// The `&str` is only used for error messages. Looks up the resolver via the
        /// ENS registry, matching the historical Alloy API.
        ///
        /// Prefer [`Self::get_resolver_for_name`] or the Universal Resolver helpers
        /// (`resolve_name`, `lookup_txt`, …). Registry-based resolver discovery does
        /// not handle ENSIP-10 extended or wildcard resolvers correctly.
        #[deprecated(
            note = "registry-based resolver discovery; use `get_resolver_for_name` or Universal Resolver helpers"
        )]
        async fn get_resolver(
            &self,
            node: B256,
            error_name: &str,
        ) -> Result<EnsResolverInstance<&P, N>, EnsError>;

        /// Returns the reverse registrar for the specified node.
        #[deprecated(
            note = "reverse resolution now routes through the Universal Resolver; use `lookup_address` instead"
        )]
        async fn get_reverse_registrar(&self) -> Result<ReverseRegistrarInstance<&P, N>, EnsError>;

        /// Performs a forward lookup of an ENS name to an Ethereum address.
        ///
        /// Routes through the [Universal Resolver], which handles wildcard resolvers
        /// and CCIP Read (ERC-3668).
        ///
        /// Uses the shared default HTTP CCIP Read client. For a custom gateway, URL
        /// policy, or limits, use [`Self::resolve_name_with_ccip_read`].
        ///
        /// [Universal Resolver]: https://docs.ens.domains/web/ensv2-readiness
        async fn resolve_name(&self, name: &str) -> Result<Address, EnsError>;

        /// Like [`Self::resolve_name`], but uses the provided CCIP Read client.
        ///
        /// Pass a custom [`CcipReadClient`] to proxy, allowlist/blocklist, or otherwise
        /// constrain contract-controlled gateway URLs as recommended by ERC-3668.
        async fn resolve_name_with_ccip_read<G: CcipReadGateway>(
            &self,
            name: &str,
            client: &CcipReadClient<G>,
        ) -> Result<Address, EnsError>;

        /// Resolves an ENS name to a multichain address for the given coin type (ENSIP-11).
        ///
        /// Returns the raw address bytes as stored in the resolver. The encoding varies
        /// by coin type: 20 raw address bytes for EVM chains, script bytes for
        /// Bitcoin, etc. Use [`coin_type::evm_chain`][crate::coin_type::evm_chain] for
        /// EVM chains; for non-EVM chains, pass the static SLIP-0044 coin type.
        async fn resolve_name_for_coin_type(
            &self,
            name: &str,
            coin_type: u64,
        ) -> Result<Bytes, EnsError>;

        /// Like [`Self::resolve_name_for_coin_type`], but uses the provided CCIP Read client.
        async fn resolve_name_for_coin_type_with_ccip_read<G: CcipReadGateway>(
            &self,
            name: &str,
            coin_type: u64,
            client: &CcipReadClient<G>,
        ) -> Result<Bytes, EnsError>;

        /// Performs a reverse lookup of an address to its primary ENS name.
        async fn lookup_address(&self, address: &Address) -> Result<String, EnsError>;

        /// Like [`Self::lookup_address`], but uses the provided CCIP Read client.
        async fn lookup_address_with_ccip_read<G: CcipReadGateway>(
            &self,
            address: &Address,
            client: &CcipReadClient<G>,
        ) -> Result<String, EnsError>;

        /// Looks up a text record for an ENS name.
        async fn lookup_txt(&self, name: &str, key: &str) -> Result<String, EnsError>;

        /// Like [`Self::lookup_txt`], but uses the provided CCIP Read client.
        async fn lookup_txt_with_ccip_read<G: CcipReadGateway>(
            &self,
            name: &str,
            key: &str,
            client: &CcipReadClient<G>,
        ) -> Result<String, EnsError>;
    }

    #[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
    #[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
    #[allow(deprecated)]
    impl<N, P> ProviderEnsExt<N, P> for P
    where
        P: Provider<N>,
        N: Network,
    {
        async fn get_resolver_for_name(
            &self,
            name: &str,
        ) -> Result<EnsResolverInstance<&P, N>, EnsError> {
            let dns = try_dns_encode(name)?;

            let ur = UniversalResolver::new(UNIVERSAL_RESOLVER_ADDRESS, self);
            let info = ur
                .requireResolver(dns.into())
                .call()
                .await
                .map_err(|error| map_ur_error(error, name, EnsError::Resolver))?;

            let resolver = crate::direct_resolver_address(&info, name)?;
            Ok(EnsResolverInstance::new(resolver, self))
        }

        async fn get_resolver(
            &self,
            node: B256,
            error_name: &str,
        ) -> Result<EnsResolverInstance<&P, N>, EnsError> {
            let registry = EnsRegistry::new(ENS_ADDRESS, self);
            let address = registry.resolver(node).call().await.map_err(EnsError::Resolver)?;
            if address == Address::ZERO {
                return Err(EnsError::ResolverNotFound(error_name.to_string()));
            }
            Ok(EnsResolverInstance::new(address, self))
        }

        async fn get_reverse_registrar(&self) -> Result<ReverseRegistrarInstance<&P, N>, EnsError> {
            let registry = EnsRegistry::new(ENS_ADDRESS, self);
            let address = registry
                .owner(namehash(ENS_REVERSE_REGISTRAR_DOMAIN))
                .call()
                .await
                .map_err(EnsError::RevRegistrar)?;
            if address == Address::ZERO {
                return Err(EnsError::ReverseRegistrarNotFound);
            }
            Ok(ReverseRegistrarInstance::new(address, self))
        }

        async fn resolve_name(&self, name: &str) -> Result<Address, EnsError> {
            self.resolve_name_with_ccip_read(name, alloy_provider::shared_http_ccip_read_client())
                .await
        }

        async fn resolve_name_with_ccip_read<G: CcipReadGateway>(
            &self,
            name: &str,
            client: &CcipReadClient<G>,
        ) -> Result<Address, EnsError> {
            let node = namehash(name);
            let dns = try_dns_encode(name)?;
            let calldata = EnsResolver::addrCall { node }.abi_encode();

            let transaction = UniversalResolver::new(UNIVERSAL_RESOLVER_ADDRESS, self)
                .resolve(dns.into(), calldata.into())
                .into_transaction_request();
            let output = client
                .call_at(self, transaction, BlockId::latest())
                .await
                .map_err(|error| map_ccip_error(error, name, EnsError::Resolve))?;
            let ret = UniversalResolver::resolveCall::abi_decode_returns(&output)
                .map_err(|_| EnsError::InvalidResponse)?;

            Address::abi_decode(ret._0.as_ref()).map_err(|_| EnsError::InvalidResponse)
        }

        async fn resolve_name_for_coin_type(
            &self,
            name: &str,
            coin_type: u64,
        ) -> Result<Bytes, EnsError> {
            self.resolve_name_for_coin_type_with_ccip_read(
                name,
                coin_type,
                alloy_provider::shared_http_ccip_read_client(),
            )
            .await
        }

        async fn resolve_name_for_coin_type_with_ccip_read<G: CcipReadGateway>(
            &self,
            name: &str,
            coin_type: u64,
            client: &CcipReadClient<G>,
        ) -> Result<Bytes, EnsError> {
            let node = namehash(name);
            let dns = try_dns_encode(name)?;
            let calldata = EnsMulticoinResolver::addrCall {
                node,
                coin_type: U256::from(coin_type),
            }
            .abi_encode();

            let transaction = UniversalResolver::new(UNIVERSAL_RESOLVER_ADDRESS, self)
                .resolve(dns.into(), calldata.into())
                .into_transaction_request();
            let output = client
                .call_at(self, transaction, BlockId::latest())
                .await
                .map_err(|error| map_ccip_error(error, name, EnsError::Resolve))?;
            let ret = UniversalResolver::resolveCall::abi_decode_returns(&output)
                .map_err(|_| EnsError::InvalidResponse)?;

            Bytes::abi_decode(ret._0.as_ref()).map_err(|_| EnsError::InvalidResponse)
        }

        async fn lookup_address(&self, address: &Address) -> Result<String, EnsError> {
            self.lookup_address_with_ccip_read(
                address,
                alloy_provider::shared_http_ccip_read_client(),
            )
            .await
        }

        async fn lookup_address_with_ccip_read<G: CcipReadGateway>(
            &self,
            address: &Address,
            client: &CcipReadClient<G>,
        ) -> Result<String, EnsError> {
            let reverse_name = reverse_address(address);
            let transaction = UniversalResolver::new(UNIVERSAL_RESOLVER_ADDRESS, self)
                .reverse(Bytes::copy_from_slice(address.as_slice()), U256::from(coin_type::ETH))
                .into_transaction_request();
            let output = client
                .call_at(self, transaction, BlockId::latest())
                .await
                .map_err(|error| map_ccip_error(error, &reverse_name, EnsError::Lookup))?;
            let ret = UniversalResolver::reverseCall::abi_decode_returns(&output)
                .map_err(|_| EnsError::InvalidResponse)?;

            Ok(ret.name)
        }

        async fn lookup_txt(&self, name: &str, key: &str) -> Result<String, EnsError> {
            self.lookup_txt_with_ccip_read(name, key, alloy_provider::shared_http_ccip_read_client())
                .await
        }

        async fn lookup_txt_with_ccip_read<G: CcipReadGateway>(
            &self,
            name: &str,
            key: &str,
            client: &CcipReadClient<G>,
        ) -> Result<String, EnsError> {
            let node = namehash(name);
            let dns = try_dns_encode(name)?;
            let calldata = EnsResolver::textCall { node, key: key.to_string() }.abi_encode();

            let transaction = UniversalResolver::new(UNIVERSAL_RESOLVER_ADDRESS, self)
                .resolve(dns.into(), calldata.into())
                .into_transaction_request();
            let output = client
                .call_at(self, transaction, BlockId::latest())
                .await
                .map_err(|error| map_ccip_error(error, name, EnsError::ResolveTxtRecord))?;
            let ret = UniversalResolver::resolveCall::abi_decode_returns(&output)
                .map_err(|_| EnsError::InvalidResponse)?;

            String::abi_decode(ret._0.as_ref()).map_err(|_| EnsError::InvalidResponse)
        }
    }

    /// Maps Universal Resolver contract errors, preserving [`EnsError::ResolverNotFound`].
    fn map_ur_error(
        error: alloy_contract::Error,
        name: &str,
        fallback: impl FnOnce(alloy_contract::Error) -> EnsError,
    ) -> EnsError {
        if error.as_decoded_error::<UniversalResolver::ResolverNotFound>().is_some() {
            EnsError::ResolverNotFound(name.to_string())
        } else {
            fallback(error)
        }
    }

    fn map_ccip_error(
        error: CcipReadError,
        name: &str,
        map_transport: impl FnOnce(alloy_contract::Error) -> EnsError,
    ) -> EnsError {
        match error {
            CcipReadError::Transport(error) => map_ur_error(error.into(), name, map_transport),
            error => EnsError::CcipRead(error),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use alloy_provider::transport::TransportErrorKind;

        #[test]
        fn preserves_existing_ens_transport_error_category() {
            let error = CcipReadError::Transport(TransportErrorKind::custom_str("test"));
            assert!(matches!(
                map_ccip_error(error, "test.eth", EnsError::Resolve),
                EnsError::Resolve(_)
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_invalid_address() {
        for addr in [
            "0x314618",
            "0x000000000000000000000000000000000000000", // 41
            "0x00000000000000000000000000000000000000000", // 43
            "0x28679A1a632125fbBf7A68d850E50623194A709E123", // 44
        ] {
            assert!(NameOrAddress::from_str(addr).is_err());
        }
    }

    #[test]
    fn test_name_or_address_dns_detection() {
        assert!(matches!(NameOrAddress::from_str("foo.eth"), Ok(NameOrAddress::Name(_))));
        assert!(matches!(NameOrAddress::from_str("ensfairy.xyz"), Ok(NameOrAddress::Name(_))));
        assert!(matches!(NameOrAddress::from_str("sub.foo.eth"), Ok(NameOrAddress::Name(_))));
    }

    #[test]
    fn test_coin_type_evm_chain() {
        assert_eq!(coin_type::evm_chain(1), Some(coin_type::ETH)); // mainnet special case
        assert_eq!(coin_type::evm_chain(8453), Some(0x8000_2105)); // Base
        assert_eq!(coin_type::evm_chain(10), Some(0x8000_000A)); // Optimism
        assert_eq!(coin_type::evm_chain(42161), Some(0x8000_A4B1)); // Arbitrum One
    }

    #[test]
    fn test_coin_type_evm_chain_rejects_out_of_range_ids() {
        // ENSIP-11 coin types are `0x80000000 | chainId` over a 31-bit chain ID
        // space; IDs with the high bit set would collide with smaller IDs.
        assert_eq!(coin_type::evm_chain(0x8000_0000), None);
        assert_eq!(coin_type::evm_chain(u64::MAX), None);
    }
}

#[cfg(all(test, feature = "contract"))]
mod resolver_info_tests {
    use super::*;
    use alloy_primitives::{address, Address, Bytes, B256, U256};

    fn info(extended: bool, offset: u64, resolver: Address) -> UniversalResolver::ResolverInfo {
        UniversalResolver::ResolverInfo {
            name: Bytes::new(),
            offset: U256::from(offset),
            node: B256::ZERO,
            resolver,
            extended,
        }
    }

    #[test]
    fn preserves_legacy_universal_resolver_binding_shape() {
        let _ = UniversalResolver::resolveReturn { _0: Default::default(), _1: Address::ZERO };
        let _ = UniversalResolver::reverseCall {
            lookupAddress: Default::default(),
            coinType: Default::default(),
        };
    }

    #[test]
    fn direct_resolver_accepts_unextended_zero_offset() {
        let resolver = address!("0x231b0Ee14048e9dCcD1d247744d114a4EB5E8E63");
        assert_eq!(
            direct_resolver_address(&info(false, 0, resolver), "vitalik.eth").unwrap(),
            resolver
        );
    }

    #[test]
    fn direct_resolver_rejects_extended() {
        let err = direct_resolver_address(
            &info(true, 0, address!("0x1111111111111111111111111111111111111111")),
            "ur.integration-tests.eth",
        )
        .unwrap_err();
        assert!(matches!(err, EnsError::RequiresUniversalResolver(_)));
    }

    #[test]
    fn direct_resolver_rejects_wildcard_offset() {
        let err = direct_resolver_address(
            &info(false, 4, address!("0x1111111111111111111111111111111111111111")),
            "sub.wildcard.eth",
        )
        .unwrap_err();
        assert!(matches!(err, EnsError::RequiresUniversalResolver(_)));
    }
}

#[cfg(all(test, feature = "provider"))]
mod provider_tests {
    #![allow(deprecated)]

    use super::*;
    use alloy_primitives::address;
    use alloy_provider::ProviderBuilder;

    const MAINNET_RPC_URL: &str = "https://ethereum.reth.rs/rpc";

    fn provider() -> impl alloy_provider::Provider {
        ProviderBuilder::new().connect_http(MAINNET_RPC_URL.parse().unwrap())
    }

    #[tokio::test]
    async fn test_reverse_registrar_fetching_mainnet() {
        let provider = provider();
        let res = provider.get_reverse_registrar().await;
        assert_eq!(*res.unwrap().address(), address!("0xa58E81fe9b61B5c3fE2AFD33CF304c454AbFc7Cb"));
    }

    #[tokio::test]
    async fn test_pub_resolver_fetching_mainnet() {
        let provider = provider();
        let name = "vitalik.eth";
        let node = namehash(name);
        let res = provider.get_resolver(node, name).await;
        assert_eq!(*res.unwrap().address(), address!("0x231b0Ee14048e9dCcD1d247744d114a4EB5E8E63"));
    }

    #[tokio::test]
    async fn test_get_resolver_for_name_mainnet() {
        let provider = provider();
        let res = provider.get_resolver_for_name("vitalik.eth").await;
        assert_eq!(*res.unwrap().address(), address!("0x231b0Ee14048e9dCcD1d247744d114a4EB5E8E63"));
    }

    #[tokio::test]
    async fn test_pub_resolver_text() {
        let provider = provider();
        let name = "vitalik.eth";
        let node = namehash(name);
        let res = provider.get_resolver(node, name).await.unwrap();
        let text = res.text(node, "avatar".to_string()).call().await.unwrap();
        assert_eq!(text, "https://euc.li/vitalik.eth")
    }

    // Canonical fixtures:
    // https://github.com/ensdomains/resolution-tests/blob/main/test-cases.json

    #[tokio::test]
    async fn test_universal_resolver() {
        let provider = provider();
        let res = provider.resolve_name("ur.integration-tests.eth").await.unwrap();
        assert_eq!(res, address!("0x2222222222222222222222222222222222222222"));
    }

    #[tokio::test]
    async fn test_get_resolver_for_name_rejects_extended_resolver() {
        let provider = provider();
        let err = provider.get_resolver_for_name("ur.integration-tests.eth").await.unwrap_err();
        assert!(matches!(err, EnsError::RequiresUniversalResolver(_)));
    }

    #[tokio::test]
    async fn test_forward_base_onchain() {
        let provider = provider();
        let res = provider
            .resolve_name_for_coin_type(
                "coins.integration-tests.eth",
                coin_type::evm_chain(8453).unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            res.as_ref(),
            address!("0xa66E90D515F576f49Af2dF40952476D56F72A420").as_slice()
        );
    }

    #[tokio::test]
    async fn test_forward_wildcard() {
        let provider = provider();
        let res = provider.resolve_name("moo331.nft-owner.eth").await.unwrap();
        assert_eq!(res, address!("0x51050ec063d393217B436747617aD1C2285Aeeee"));
    }

    #[tokio::test]
    async fn test_forward_eth_offchain() {
        let provider = provider();
        let res = provider.resolve_name("test.offchaindemo.eth").await.unwrap();
        assert_eq!(res, address!("0x779981590E7Ccc0CFAe8040Ce7151324747cDb97"));
    }

    #[tokio::test]
    async fn test_forward_text_onchain() {
        let provider = provider();
        let res = provider.lookup_txt("integration-tests.eth", "avatar").await.unwrap();
        assert_eq!(
            res,
            "https://raw.githubusercontent.com/ensdomains/resolution-tests/refs/heads/main/assets/avatar.svg"
        );
    }

    #[tokio::test]
    async fn test_forward_text_offchain() {
        let provider = provider();
        let res = provider.lookup_txt("test.offchaindemo.eth", "description").await.unwrap();
        assert_eq!(res, "asdflkjasdflkjasdf");
    }

    #[tokio::test]
    async fn test_reverse_eth() {
        let provider = provider();
        let res = provider
            .lookup_address(&address!("0xeE9eeaAB0Bb7D9B969D701f6f8212609EDeA252E"))
            .await
            .unwrap();
        assert_eq!(res, "devrel.enslabs.eth");
    }

    #[tokio::test]
    async fn test_forward_dns_offchain() {
        let provider = provider();
        let res = provider.resolve_name("pokersback.com").await.unwrap();
        assert_eq!(res, address!("0x534631Bcf33BDb069fB20A93d2fdb9e4D4dD42CF"));
    }
}
