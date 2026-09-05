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

/// ENS registry address (`0x00000000000C2E074eC69A0dFb2997BA6C7d2e1e`)
pub const ENS_ADDRESS: Address = address!("0x00000000000C2E074eC69A0dFb2997BA6C7d2e1e");

/// ENS Universal Resolver address on Ethereum Mainnet
/// (`0xeEeEEEeE14D718C2B47D9923Deab1335E144EeEe`)
///
/// The Universal Resolver is the canonical entry point for all ENS resolution. It routes to
/// wildcard (ENSIP-10) resolvers and signals CCIP Read (ERC-3668) with an `OffchainLookup` revert.
pub const UNIVERSAL_RESOLVER_ADDRESS: Address =
    address!("0xeEeEEEeE14D718C2B47D9923Deab1335E144EeEe");

/// ENS const for registrar domain
pub const ENS_REVERSE_REGISTRAR_DOMAIN: &str = "addr.reverse";

#[cfg(feature = "contract")]
pub use contract::*;

#[cfg(feature = "provider")]
pub use provider::*;

/// Helpers for ENS multichain coin types.
///
/// Non-EVM chains use their static SLIP-0044 coin type according to
/// [ENSIP-9](https://docs.ens.domains/ensip/9). EVM chains follow
/// [ENSIP-11](https://docs.ens.domains/ensip/11): `coinType = 0x80000000 | chainId`.
pub mod coin_type {
    /// Ethereum mainnet (SLIP-0044 coin type 60).
    pub const ETH: u64 = 60;

    /// Converts an EVM chain ID to its ENSIP-11 coin type.
    ///
    /// Chain ID `1` (Ethereum mainnet) maps to [`ETH`]; all other chains use
    /// `0x80000000 | chain_id`.
    ///
    /// Returns `None` for chain IDs of `0x80000000` and above: ENSIP-11 reserves a 31-bit chain ID
    /// space, and larger IDs would collide with the coin types of smaller ones.
    pub const fn evm_chain(chain_id: u64) -> Option<u64> {
        if chain_id == 1 {
            Some(ETH)
        } else if chain_id < 0x8000_0000 {
            Some(0x8000_0000 | chain_id)
        } else {
            None
        }
    }
}

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
    use alloy_primitives::Address;
    use alloy_sol_types::sol;

    sol! {
        /// ENS Registry contract.
        ///
        /// The resolution helpers in this crate route through the [`UniversalResolver`] instead of
        /// querying the registry directly, since the registry is unaware of wildcard resolvers.
        #[sol(rpc)]
        contract EnsRegistry {
            /// Returns the resolver for the specified node.
            function resolver(bytes32 node) view returns (address);

            /// returns the owner of this node
            function owner(bytes32 node) view returns (address);
        }

        /// ENS Reverse Registrar contract
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
        /// Use with the Universal Resolver and coin types from ENSIP-9 or
        /// [`coin_type::evm_chain`](crate::coin_type::evm_chain).
        #[sol(rpc)]
        contract EnsMulticoinResolver {
            /// Returns the address for `node` on the chain identified by `coinType`.
            ///
            /// The returned bytes are the raw address encoding for that coin type, e.g. 20 raw
            /// bytes for EVM chains or script bytes for Bitcoin.
            function addr(bytes32 node, uint256 coinType) view returns (bytes memory);
        }

        /// ENS Universal Resolver ([ENSIP-23](https://docs.ens.domains/ensip/23)).
        ///
        /// The single entry point for ENS resolution. It routes to wildcard (ENSIP-10) resolvers
        /// and signals CCIP Read (ERC-3668) with an `OffchainLookup` revert, which this binding
        /// does not follow. The provider helpers in this crate follow those reverts with the
        /// `alloy-provider` CCIP Read client.
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

            /// Returns the resolver for `name` (DNS wire format) without performing resolution.
            ///
            /// Returns `(resolver, node, offset)`, where `offset` is the byte offset into `name`
            /// at which the resolver was found (non-zero for parent/wildcard resolution).
            function findResolver(bytes memory name) public view returns (address, bytes32, uint256);

            /// Resolves `name` (DNS wire format) using the encoded resolver call `data`.
            ///
            /// Returns the ABI-encoded result of the resolver call and the resolver address.
            /// Reverts with `OffchainLookup` when CCIP Read (ERC-3668) is required.
            function resolve(bytes calldata name, bytes calldata data) external view returns (bytes memory, address);

            /// Like `resolve`, but uses `gateways` for CCIP Read instead of the default.
            function resolveWithGateways(bytes calldata name, bytes calldata data, string[] memory gateways) public view returns (bytes memory, address);

            /// Reverse-resolves the raw address bytes `lookupAddress` on the chain identified by
            /// `coinType` (see [`coin_type`](crate::coin_type)) to its primary ENS name.
            function reverse(bytes calldata lookupAddress, uint256 coinType) external view returns (string memory name, address resolver, address reverseResolver);

            /// Like `reverse`, but uses `gateways` for CCIP Read instead of the default.
            function reverseWithGateways(bytes calldata lookupAddress, uint256 coinType, string[] memory gateways) public view returns (string memory name, address resolver, address reverseResolver);
        }
    }

    impl UniversalResolver::ResolverInfo {
        /// Returns the resolver address if `addr`/`text`/`name` may be called on it directly.
        ///
        /// Extended (ENSIP-10) and parent/wildcard resolvers must be queried through the Universal
        /// Resolver instead, so this returns [`EnsError::RequiresUniversalResolver`] for them.
        pub(crate) fn direct_resolver(&self, name: &str) -> Result<Address, EnsError> {
            if self.extended || !self.offset.is_zero() {
                Err(EnsError::RequiresUniversalResolver(name.to_string()))
            } else {
                Ok(self.resolver)
            }
        }
    }

    /// Error type for ENS resolution.
    #[derive(Debug, thiserror::Error)]
    #[non_exhaustive]
    pub enum EnsError {
        /// Failed to get the resolver for this name.
        #[error("Failed to get ENS resolver: {0}")]
        Resolver(alloy_contract::Error),
        /// No resolver found for the given name.
        #[error("ENS resolver not found for name {0:?}")]
        ResolverNotFound(String),
        /// The name resolves through an ENSIP-10 extended resolver or a parent/wildcard resolver,
        /// so calling `addr`/`text`/`name` on the resolver directly can return incorrect data.
        ///
        /// Use `resolve_name`, `lookup_txt`, or the other Universal Resolver helpers instead.
        #[error(
            "ENS name {0:?} requires Universal Resolver resolution (extended or wildcard resolver)"
        )]
        RequiresUniversalResolver(String),
        /// Failed to get the reverse registrar from the ENS registry.
        #[error("Failed to get reverse registrar from the ENS registry: {0}")]
        RevRegistrar(alloy_contract::Error),
        /// No reverse registrar found for `addr.reverse`.
        #[error("ENS reverse registrar not found for addr.reverse")]
        ReverseRegistrarNotFound,
        /// Failed to lookup ENS name from an address.
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
        /// Failed to perform a CCIP Read request.
        ///
        /// Transport errors of a CCIP Read call are mapped to [`Resolve`](Self::Resolve),
        /// [`Lookup`](Self::Lookup), or [`ResolveTxtRecord`](Self::ResolveTxtRecord) like plain
        /// calls; this variant covers gateway and protocol failures.
        #[cfg(feature = "provider")]
        #[error("Failed to perform CCIP Read: {0}")]
        CcipRead(#[source] alloy_provider::CcipReadError),
    }
}

#[cfg(feature = "provider")]
mod provider {
    use crate::{
        coin_type, namehash, reverse_address, try_dns_encode, EnsError, EnsMulticoinResolver,
        EnsRegistry, EnsResolver, EnsResolver::EnsResolverInstance,
        ReverseRegistrar::ReverseRegistrarInstance, UniversalResolver, ENS_ADDRESS,
        ENS_REVERSE_REGISTRAR_DOMAIN, UNIVERSAL_RESOLVER_ADDRESS,
    };
    use alloy_primitives::{Address, Bytes, B256, U256};
    use alloy_provider::{
        shared_http_ccip_read_client, CcipReadClient, CcipReadError, CcipReadGateway, Network,
        Provider,
    };
    use alloy_sol_types::{SolCall, SolValue};

    /// Extension trait for ENS contract calls.
    ///
    /// All ENS name strings must already be normalized according to ENSIP-15. These helpers do not
    /// perform complete normalization or validation before hashing or DNS-encoding them;
    /// [`namehash`] and [`try_dns_encode`] only remove `U+FE0F`.
    ///
    /// Resolution routes through the [Universal Resolver](UNIVERSAL_RESOLVER_ADDRESS), which
    /// handles wildcard resolvers, and follows CCIP Read (ERC-3668) redirects with the shared
    /// default HTTP [`CcipReadClient`]. The `*_with_ccip_read` variants accept a custom client to
    /// constrain, proxy, or replace the contract-controlled gateway URLs.
    ///
    /// CCIP Read calls are issued against the latest block, so the initial `eth_call`, the
    /// gateway request, and the callback `eth_call` can observe different chain heads. ERC-3668
    /// resolvers typically bind signed gateway payloads to the lookup; that is verified by the
    /// callback on chain, not by these helpers.
    #[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
    #[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
    pub trait ProviderEnsExt<N: Network, P: Provider<N>> {
        /// Returns the resolver contract instance for the given ENS name.
        ///
        /// The resolver is discovered through the Universal Resolver. Returns
        /// [`EnsError::RequiresUniversalResolver`] when the name uses an ENSIP-10 extended
        /// resolver or a parent/wildcard resolver: direct `addr`/`text`/`name` calls on such
        /// resolvers are not equivalent to Universal Resolver resolution, so use
        /// [`Self::resolve_name`], [`Self::lookup_txt`], or the other helpers instead.
        async fn get_resolver_for_name(
            &self,
            name: &str,
        ) -> Result<EnsResolverInstance<&P, N>, EnsError>;

        /// Returns the resolver for the specified node. The `&str` is only used for error messages.
        ///
        /// This looks up the resolver in the ENS registry, which does not account for ENSIP-10
        /// extended or wildcard resolvers.
        #[deprecated(note = "use `get_resolver_for_name` or the Universal Resolver helpers")]
        async fn get_resolver(
            &self,
            node: B256,
            error_name: &str,
        ) -> Result<EnsResolverInstance<&P, N>, EnsError>;

        /// Returns the reverse registrar for the specified node.
        #[deprecated(
            note = "reverse resolution routes through the Universal Resolver; use `lookup_address`"
        )]
        async fn get_reverse_registrar(&self) -> Result<ReverseRegistrarInstance<&P, N>, EnsError>;

        /// Performs a forward lookup of an ENS name to an Ethereum address.
        async fn resolve_name(&self, name: &str) -> Result<Address, EnsError>;

        /// Like [`Self::resolve_name`], but follows CCIP Read redirects with `client`.
        async fn resolve_name_with_ccip_read<G: CcipReadGateway>(
            &self,
            name: &str,
            client: &CcipReadClient<G>,
        ) -> Result<Address, EnsError>;

        /// Resolves an ENS name to a multichain address for the given coin type (ENSIP-11).
        ///
        /// Returns the raw address bytes as stored in the resolver, whose encoding depends on the
        /// coin type: 20 raw bytes for EVM chains, script bytes for Bitcoin, etc. Use
        /// [`coin_type::evm_chain`] for EVM chains and the static SLIP-0044 coin type otherwise.
        async fn resolve_name_for_coin_type(
            &self,
            name: &str,
            coin_type: u64,
        ) -> Result<Bytes, EnsError>;

        /// Like [`Self::resolve_name_for_coin_type`], but follows CCIP Read redirects with
        /// `client`.
        async fn resolve_name_for_coin_type_with_ccip_read<G: CcipReadGateway>(
            &self,
            name: &str,
            coin_type: u64,
            client: &CcipReadClient<G>,
        ) -> Result<Bytes, EnsError>;

        /// Performs a reverse lookup of an address to its primary ENS name.
        ///
        /// The Universal Resolver verifies that the raw primary name resolves back to `address`,
        /// and returns an empty string if no primary name is set. Neither the Universal Resolver
        /// nor this helper performs ENSIP-15 normalization of the returned name.
        ///
        /// Before using a non-empty result as an authenticated identity, normalize it according to
        /// ENSIP-15 and reject it if normalization fails or changes the name. Normalizing and using
        /// a changed name is insufficient: the address check applied to the original name.
        async fn lookup_address(&self, address: &Address) -> Result<String, EnsError>;

        /// Like [`Self::lookup_address`], but follows CCIP Read redirects with `client`.
        async fn lookup_address_with_ccip_read<G: CcipReadGateway>(
            &self,
            address: &Address,
            client: &CcipReadClient<G>,
        ) -> Result<String, EnsError>;

        /// Looks up a text record for an ENS name.
        async fn lookup_txt(&self, name: &str, key: &str) -> Result<String, EnsError>;

        /// Like [`Self::lookup_txt`], but follows CCIP Read redirects with `client`.
        async fn lookup_txt_with_ccip_read<G: CcipReadGateway>(
            &self,
            name: &str,
            key: &str,
            client: &CcipReadClient<G>,
        ) -> Result<String, EnsError>;
    }

    #[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
    #[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
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
            let info = UniversalResolver::new(UNIVERSAL_RESOLVER_ADDRESS, self)
                .requireResolver(dns.into())
                .call()
                .await
                .map_err(|error| map_ur_error(error, name, EnsError::Resolver))?;
            Ok(EnsResolverInstance::new(info.direct_resolver(name)?, self))
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
            self.resolve_name_with_ccip_read(name, shared_http_ccip_read_client()).await
        }

        async fn resolve_name_with_ccip_read<G: CcipReadGateway>(
            &self,
            name: &str,
            client: &CcipReadClient<G>,
        ) -> Result<Address, EnsError> {
            let data = EnsResolver::addrCall { node: namehash(name) }.abi_encode();
            let ret = resolve(self, client, name, data, EnsError::Resolve).await?;
            Address::abi_decode(&ret).map_err(|_| EnsError::InvalidResponse)
        }

        async fn resolve_name_for_coin_type(
            &self,
            name: &str,
            coin_type: u64,
        ) -> Result<Bytes, EnsError> {
            self.resolve_name_for_coin_type_with_ccip_read(
                name,
                coin_type,
                shared_http_ccip_read_client(),
            )
            .await
        }

        async fn resolve_name_for_coin_type_with_ccip_read<G: CcipReadGateway>(
            &self,
            name: &str,
            coin_type: u64,
            client: &CcipReadClient<G>,
        ) -> Result<Bytes, EnsError> {
            let data = EnsMulticoinResolver::addrCall {
                node: namehash(name),
                coinType: U256::from(coin_type),
            }
            .abi_encode();
            let ret = resolve(self, client, name, data, EnsError::Resolve).await?;
            Bytes::abi_decode(&ret).map_err(|_| EnsError::InvalidResponse)
        }

        async fn lookup_address(&self, address: &Address) -> Result<String, EnsError> {
            self.lookup_address_with_ccip_read(address, shared_http_ccip_read_client()).await
        }

        async fn lookup_address_with_ccip_read<G: CcipReadGateway>(
            &self,
            address: &Address,
            client: &CcipReadClient<G>,
        ) -> Result<String, EnsError> {
            let call = UniversalResolver::new(UNIVERSAL_RESOLVER_ADDRESS, self)
                .reverse(Bytes::copy_from_slice(address.as_slice()), U256::from(coin_type::ETH))
                .into_transaction_request();
            let output = client.call(self, call).await.map_err(|error| {
                map_ccip_error(error, &reverse_address(address), EnsError::Lookup)
            })?;
            let ret = UniversalResolver::reverseCall::abi_decode_returns(&output)
                .map_err(|_| EnsError::InvalidResponse)?;
            Ok(ret.name)
        }

        async fn lookup_txt(&self, name: &str, key: &str) -> Result<String, EnsError> {
            self.lookup_txt_with_ccip_read(name, key, shared_http_ccip_read_client()).await
        }

        async fn lookup_txt_with_ccip_read<G: CcipReadGateway>(
            &self,
            name: &str,
            key: &str,
            client: &CcipReadClient<G>,
        ) -> Result<String, EnsError> {
            let data =
                EnsResolver::textCall { node: namehash(name), key: key.to_string() }.abi_encode();
            let ret = resolve(self, client, name, data, EnsError::ResolveTxtRecord).await?;
            String::abi_decode(&ret).map_err(|_| EnsError::InvalidResponse)
        }
    }

    /// Resolves the encoded resolver call `data` for `name` through the Universal Resolver,
    /// following CCIP Read redirects with `client`, and returns the resolver's return data.
    async fn resolve<N: Network, P: Provider<N>, G: CcipReadGateway>(
        provider: &P,
        client: &CcipReadClient<G>,
        name: &str,
        data: Vec<u8>,
        map_err: impl FnOnce(alloy_contract::Error) -> EnsError,
    ) -> Result<Bytes, EnsError> {
        let dns = try_dns_encode(name)?;
        let call = UniversalResolver::new(UNIVERSAL_RESOLVER_ADDRESS, provider)
            .resolve(dns.into(), data.into())
            .into_transaction_request();
        let output = client
            .call(provider, call)
            .await
            .map_err(|error| map_ccip_error(error, name, map_err))?;
        let ret = UniversalResolver::resolveCall::abi_decode_returns(&output)
            .map_err(|_| EnsError::InvalidResponse)?;
        Ok(ret._0)
    }

    /// Maps CCIP Read errors, keeping the error categories of plain calls for transport errors.
    fn map_ccip_error(
        error: CcipReadError,
        name: &str,
        fallback: impl FnOnce(alloy_contract::Error) -> EnsError,
    ) -> EnsError {
        match error {
            CcipReadError::Transport(error) => map_ur_error(error.into(), name, fallback),
            error => EnsError::CcipRead(error),
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

    #[cfg(test)]
    mod tests {
        use super::*;
        use alloy_json_rpc::ErrorPayload;
        use alloy_primitives::{address, bytes, fixed_bytes};
        use alloy_provider::{
            mock::Asserter,
            transport::{TransportError, TransportErrorKind},
            CcipReadGatewayError, CcipReadRequest, ProviderBuilder,
        };
        use alloy_sol_types::{sol, SolError};

        sol! {
            error OffchainLookup(
                address sender,
                string[] urls,
                bytes callData,
                bytes4 callbackFunction,
                bytes extraData
            );
        }

        /// Gateway that answers every request with the same bytes.
        struct MockGateway(Bytes);

        #[async_trait::async_trait]
        impl CcipReadGateway for MockGateway {
            async fn request(
                &self,
                _request: &CcipReadRequest,
                _max_response_size: usize,
            ) -> Result<Bytes, CcipReadGatewayError> {
                Ok(self.0.clone())
            }
        }

        fn revert(data: &Bytes) -> ErrorPayload {
            ErrorPayload::internal_error_with_message_and_obj(
                "execution reverted".into(),
                serde_json::value::to_raw_value(data).unwrap(),
            )
        }

        #[tokio::test]
        async fn resolve_name_follows_offchain_lookup() {
            let expected = address!("0x779981590E7Ccc0CFAe8040Ce7151324747cDb97");
            let lookup: Bytes = OffchainLookup {
                sender: UNIVERSAL_RESOLVER_ADDRESS,
                urls: vec!["https://gateway.test/{sender}/{data}".into()],
                callData: bytes!("01"),
                callbackFunction: fixed_bytes!("11223344"),
                extraData: bytes!("02"),
            }
            .abi_encode()
            .into();
            let output: Bytes = UniversalResolver::resolveCall::abi_encode_returns(
                &UniversalResolver::resolveReturn {
                    _0: expected.abi_encode().into(),
                    _1: Address::ZERO,
                },
            )
            .into();

            let asserter = Asserter::new();
            asserter.push_failure(revert(&lookup));
            asserter.push_success(&output);
            let provider = ProviderBuilder::new().connect_mocked_client(asserter);
            let client = CcipReadClient::new(MockGateway(bytes!("abcd")));

            let resolved = provider
                .resolve_name_with_ccip_read("test.offchaindemo.eth", &client)
                .await
                .unwrap();
            assert_eq!(resolved, expected);
        }

        #[test]
        fn map_ccip_error_keeps_plain_call_categories() {
            let transport = CcipReadError::Transport(TransportErrorKind::custom_str("test"));
            assert!(matches!(
                map_ccip_error(transport, "test.eth", EnsError::Resolve),
                EnsError::Resolve(_)
            ));

            let not_found: Bytes =
                UniversalResolver::ResolverNotFound { name: bytes!("03666f6f0365746800") }
                    .abi_encode()
                    .into();
            let transport = CcipReadError::Transport(TransportError::ErrorResp(revert(&not_found)));
            assert!(matches!(
                map_ccip_error(transport, "foo.eth", EnsError::Resolve),
                EnsError::ResolverNotFound(name) if name == "foo.eth"
            ));

            for error in [
                CcipReadError::Gateway(CcipReadGatewayError::new("gateway down")),
                CcipReadError::TooManyRedirects(4),
                CcipReadError::SenderMismatch { sender: Address::ZERO, target: Address::ZERO },
            ] {
                assert!(matches!(
                    map_ccip_error(error, "test.eth", EnsError::Resolve),
                    EnsError::CcipRead(_)
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_name_or_address_from_str_name() {
        for name in ["foo.eth", "ensfairy.xyz", "sub.foo.eth"] {
            assert_eq!(NameOrAddress::from_str(name).unwrap(), NameOrAddress::Name(name.into()));
        }
    }

    #[test]
    fn test_coin_type_evm_chain() {
        assert_eq!(coin_type::evm_chain(1), Some(coin_type::ETH));
        assert_eq!(coin_type::evm_chain(8453), Some(0x8000_2105)); // Base
        assert_eq!(coin_type::evm_chain(10), Some(0x8000_000A)); // Optimism
        assert_eq!(coin_type::evm_chain(42161), Some(0x8000_A4B1)); // Arbitrum One
                                                                    // Chain IDs with the high bit
                                                                    // set would collide with
                                                                    // smaller IDs.
        assert_eq!(coin_type::evm_chain(0x8000_0000), None);
        assert_eq!(coin_type::evm_chain(u64::MAX), None);
    }
}

#[cfg(all(test, feature = "contract"))]
mod resolver_info_tests {
    use super::*;
    use alloy_primitives::{address, Bytes, B256, U256};

    #[test]
    fn direct_resolver_rejects_extended_and_wildcard_resolvers() {
        let resolver = address!("0x231b0Ee14048e9dCcD1d247744d114a4EB5E8E63");
        let info = |extended: bool, offset: u64| UniversalResolver::ResolverInfo {
            name: Bytes::new(),
            offset: U256::from(offset),
            node: B256::ZERO,
            resolver,
            extended,
        };

        assert_eq!(info(false, 0).direct_resolver("vitalik.eth").unwrap(), resolver);
        for (extended, offset) in [(true, 0), (false, 4), (true, 4)] {
            let err = info(extended, offset).direct_resolver("sub.wildcard.eth").unwrap_err();
            assert!(
                matches!(err, EnsError::RequiresUniversalResolver(name) if name == "sub.wildcard.eth")
            );
        }
    }
}

#[cfg(all(test, feature = "provider"))]
mod provider_tests {
    #![allow(deprecated)]

    use super::*;
    use alloy_primitives::address;
    use alloy_provider::{Provider, ProviderBuilder};

    fn provider() -> impl Provider {
        ProviderBuilder::new().connect_http("https://ethereum.reth.rs/rpc".parse().unwrap())
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
        let res = provider.get_resolver(namehash(name), name).await;
        assert_eq!(*res.unwrap().address(), address!("0x231b0Ee14048e9dCcD1d247744d114a4EB5E8E63"));
    }

    #[tokio::test]
    async fn test_get_resolver_for_name_mainnet() {
        let provider = provider();
        let res = provider.get_resolver_for_name("vitalik.eth").await;
        assert_eq!(*res.unwrap().address(), address!("0x231b0Ee14048e9dCcD1d247744d114a4EB5E8E63"));
    }

    #[tokio::test]
    async fn test_get_resolver_for_name_rejects_extended_resolver() {
        let err = provider().get_resolver_for_name("ur.integration-tests.eth").await.unwrap_err();
        assert!(matches!(err, EnsError::RequiresUniversalResolver(_)));
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

    #[tokio::test]
    async fn test_resolve_name_via_universal_resolver() {
        let addr = provider().resolve_name("ur.integration-tests.eth").await.unwrap();
        assert_eq!(addr, address!("0x2222222222222222222222222222222222222222"));
    }

    #[tokio::test]
    async fn test_resolve_name_for_coin_type_via_universal_resolver() {
        let res = provider()
            .resolve_name_for_coin_type(
                "coins.integration-tests.eth",
                coin_type::evm_chain(8453).unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.as_ref(), address!("0xa66E90D515F576f49Af2dF40952476D56F72A420").as_slice());
    }

    #[tokio::test]
    async fn test_lookup_address_via_universal_resolver() {
        let name = provider()
            .lookup_address(&address!("0xeE9eeaAB0Bb7D9B969D701f6f8212609EDeA252E"))
            .await
            .unwrap();
        assert_eq!(name, "devrel.enslabs.eth");
    }

    #[tokio::test]
    async fn test_lookup_txt_via_universal_resolver() {
        let avatar = provider().lookup_txt("integration-tests.eth", "avatar").await.unwrap();
        assert_eq!(
            avatar,
            "https://raw.githubusercontent.com/ensdomains/resolution-tests/refs/heads/main/assets/avatar.svg"
        );
    }

    #[tokio::test]
    async fn test_pub_resolver_fetching_txt() {
        let res = provider().lookup_txt("vitalik.eth", "avatar").await.unwrap();
        assert_eq!(res, "https://euc.li/vitalik.eth")
    }

    // Fixtures from <https://github.com/ensdomains/resolution-tests/blob/main/test-cases.json>.

    #[tokio::test]
    async fn test_resolve_wildcard_name() {
        let addr = provider().resolve_name("moo331.nft-owner.eth").await.unwrap();
        assert_eq!(addr, address!("0x51050ec063d393217B436747617aD1C2285Aeeee"));
    }

    #[tokio::test]
    async fn test_resolve_offchain_name() {
        let addr = provider().resolve_name("test.offchaindemo.eth").await.unwrap();
        assert_eq!(addr, address!("0x779981590E7Ccc0CFAe8040Ce7151324747cDb97"));
    }

    #[tokio::test]
    async fn test_lookup_txt_offchain_name() {
        let res = provider().lookup_txt("test.offchaindemo.eth", "description").await.unwrap();
        assert_eq!(res, "asdflkjasdflkjasdf");
    }

    #[tokio::test]
    async fn test_resolve_dns_name() {
        let addr = provider().resolve_name("pokersback.com").await.unwrap();
        assert_eq!(addr, address!("0x534631Bcf33BDb069fB20A93d2fdb9e4D4dD42CF"));
    }
}
