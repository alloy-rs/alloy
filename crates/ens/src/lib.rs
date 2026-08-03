#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! ENS Name resolving utilities.

mod utils;
pub use utils::{dns_encode, namehash, reverse_address, DnsEncodeError};

use alloy_primitives::{address, Address};
use std::str::FromStr;

/// ENS Universal Resolver address (`0xeEeEEEeE14D718C2B47D9923Deab1335E144EeEe`).
///
/// The primary entry-point for ENS name resolution. Supports wildcard resolvers
/// and CCIP Read (ERC-3668).
pub const UNIVERSAL_RESOLVER_ADDRESS: Address =
    address!("0xeEeEEEeE14D718C2B47D9923Deab1335E144EeEe");

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
    pub fn evm_chain(chain_id: u32) -> u64 {
        if chain_id == 1 {
            return ETH;
        }
        0x8000_0000u64 | u64::from(chain_id)
    }
}

#[cfg(feature = "contract")]
pub use contract::*;

#[cfg(feature = "provider")]
pub use provider::*;

/// ENS name or Ethereum Address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NameOrAddress {
    /// An ENS Name (format does not get checked)
    Name(String),
    /// An Ethereum Address
    Address(Address),
}

impl NameOrAddress {
    /// Resolves the name to an Ethereum Address.
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
        /// Note: CCIP Read requires client-side handling of the `OffchainLookup` revert
        /// (ERC-3668). Alloy does not currently implement this; calls to names that require
        /// CCIP Read will surface as [`EnsError::Resolve`].
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
            function resolve(bytes calldata name, bytes calldata data) external view returns (bytes memory result, address resolver);

            /// Like `resolve`, but uses `gateways` for CCIP Read instead of the default.
            function resolveWithGateways(bytes calldata name, bytes calldata data, string[] memory gateways) public view returns (bytes memory result, address resolver);

            /// Reverse-resolves an address to its primary ENS name.
            ///
            /// `lookupAddress` is the raw byte encoding of the address.
            /// `coinType` specifies the chain (use [`coin_type::ETH`] for Ethereum).
            function reverse(bytes calldata lookupAddress, uint256 coinType) external view returns (string memory name, address resolver, address reverseResolver);

            /// Like `reverse`, but uses `gateways` for CCIP Read instead of the default.
            function reverseWithGateways(bytes calldata lookupAddress, uint256 coinType, string[] memory gateways) public view returns (string memory name, address resolver, address reverseResolver);
        }
    }

    /// Error type for ENS resolution.
    #[derive(Debug, thiserror::Error)]
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
    }
}

#[cfg(feature = "provider")]
mod provider {
    use crate::{
        coin_type, dns_encode, namehash, EnsError, EnsMulticoinResolver, EnsResolver,
        EnsResolver::EnsResolverInstance, UniversalResolver, UNIVERSAL_RESOLVER_ADDRESS,
    };
    use alloy_primitives::{Address, Bytes, U256};
    use alloy_provider::{Network, Provider};
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
        async fn get_resolver(&self, name: &str) -> Result<EnsResolverInstance<&P, N>, EnsError>;

        /// Performs a forward lookup of an ENS name to an Ethereum address.
        ///
        /// Routes through the [Universal Resolver], which handles wildcard resolvers
        /// and CCIP Read (ERC-3668).
        ///
        /// [Universal Resolver]: https://docs.ens.domains/web/ensv2-readiness
        async fn resolve_name(&self, name: &str) -> Result<Address, EnsError>;

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

        /// Performs a reverse lookup of an address to its primary ENS name.
        async fn lookup_address(&self, address: &Address) -> Result<String, EnsError>;

        /// Looks up a text record for an ENS name.
        async fn lookup_txt(&self, name: &str, key: &str) -> Result<String, EnsError>;
    }

    #[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
    #[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
    impl<N, P> ProviderEnsExt<N, P> for P
    where
        P: Provider<N>,
        N: Network,
    {
        async fn get_resolver(&self, name: &str) -> Result<EnsResolverInstance<&P, N>, EnsError> {
            let dns = dns_encode(name)?;

            let ur = UniversalResolver::new(UNIVERSAL_RESOLVER_ADDRESS, self);
            let info = ur
                .requireResolver(dns.into())
                .call()
                .await
                .map_err(EnsError::Resolver)?;

            // Extended and parent/wildcard resolvers must be queried through the
            // Universal Resolver. Returning a raw instance invites incorrect direct
            // `addr`/`text` calls (e.g. ur.integration-tests.eth).
            if info.extended || !info.offset.is_zero() {
                return Err(EnsError::RequiresUniversalResolver(name.to_string()));
            }

            Ok(EnsResolverInstance::new(info.resolver, self))
        }

        async fn resolve_name(&self, name: &str) -> Result<Address, EnsError> {
            let node = namehash(name);
            let dns = dns_encode(name)?;
            let calldata = EnsResolver::addrCall { node }.abi_encode();

            let ur = UniversalResolver::new(UNIVERSAL_RESOLVER_ADDRESS, self);
            let ret = ur
                .resolve(dns.into(), calldata.into())
                .call()
                .await
                .map_err(EnsError::Resolve)?;

            Address::abi_decode(ret.result.as_ref()).map_err(|_| EnsError::InvalidResponse)
        }

        async fn resolve_name_for_coin_type(
            &self,
            name: &str,
            coin_type: u64,
        ) -> Result<Bytes, EnsError> {
            let node = namehash(name);
            let dns = dns_encode(name)?;
            let calldata = EnsMulticoinResolver::addrCall {
                node,
                coin_type: U256::from(coin_type),
            }
            .abi_encode();

            let ur = UniversalResolver::new(UNIVERSAL_RESOLVER_ADDRESS, self);
            let ret = ur
                .resolve(dns.into(), calldata.into())
                .call()
                .await
                .map_err(EnsError::Resolve)?;

            Bytes::abi_decode(ret.result.as_ref()).map_err(|_| EnsError::InvalidResponse)
        }

        async fn lookup_address(&self, address: &Address) -> Result<String, EnsError> {
            let ur = UniversalResolver::new(UNIVERSAL_RESOLVER_ADDRESS, self);
            let ret = ur
                .reverse(Bytes::copy_from_slice(address.as_slice()), U256::from(coin_type::ETH))
                .call()
                .await
                .map_err(EnsError::Lookup)?;

            Ok(ret.name)
        }

        async fn lookup_txt(&self, name: &str, key: &str) -> Result<String, EnsError> {
            let node = namehash(name);
            let dns = dns_encode(name)?;
            let calldata = EnsResolver::textCall { node, key: key.to_string() }.abi_encode();

            let ur = UniversalResolver::new(UNIVERSAL_RESOLVER_ADDRESS, self);
            let ret = ur
                .resolve(dns.into(), calldata.into())
                .call()
                .await
                .map_err(EnsError::ResolveTxtRecord)?;

            String::abi_decode(ret.result.as_ref()).map_err(|_| EnsError::InvalidResponse)
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
        assert_eq!(coin_type::evm_chain(1), coin_type::ETH); // mainnet special case
        assert_eq!(coin_type::evm_chain(8453), 0x8000_2105); // Base
        assert_eq!(coin_type::evm_chain(10), 0x8000_000A); // Optimism
        assert_eq!(coin_type::evm_chain(42161), 0x8000_A4B1); // Arbitrum One
    }

    #[test]
    fn test_coin_type_evm_chain_does_not_reject_high_bit() {
        assert_eq!(coin_type::evm_chain(0x8000_0000), 0x8000_0000);
    }
}

#[cfg(all(test, feature = "provider"))]
mod provider_tests {
    use super::*;
    use alloy_primitives::address;
    use alloy_provider::ProviderBuilder;

    #[tokio::test]
    async fn test_pub_resolver_fetching_mainnet() {
        let provider = ProviderBuilder::new()
            .connect_http("https://ethereum.reth.rs/rpc".parse().unwrap());

        let res = provider.get_resolver("vitalik.eth").await;
        assert_eq!(*res.unwrap().address(), address!("0x231b0Ee14048e9dCcD1d247744d114a4EB5E8E63"));
    }

    #[tokio::test]
    async fn test_pub_resolver_text() {
        let provider = ProviderBuilder::new()
            .connect_http("https://ethereum.reth.rs/rpc".parse().unwrap());

        let name = "vitalik.eth";
        let node = namehash(name);
        let res = provider.get_resolver(name).await.unwrap();
        let text = res.text(node, "avatar".to_string()).call().await.unwrap();
        assert_eq!(text, "https://euc.li/vitalik.eth")
    }

    #[tokio::test]
    async fn test_pub_resolver_fetching_text() {
        let provider = ProviderBuilder::new()
            .connect_http("https://ethereum.reth.rs/rpc".parse().unwrap());

        let res = provider.lookup_txt("vitalik.eth", "avatar").await.unwrap();
        assert_eq!(res, "https://euc.li/vitalik.eth")
    }

    #[tokio::test]
    async fn test_universal_resolver_integration() {
        let provider = ProviderBuilder::new()
            .connect_http("https://ethereum.reth.rs/rpc".parse().unwrap());

        let res = provider.resolve_name("ur.integration-tests.eth").await.unwrap();
        assert_eq!(res, address!("0x2222222222222222222222222222222222222222"));
    }

    #[tokio::test]
    async fn test_get_resolver_rejects_extended_resolver() {
        let provider = ProviderBuilder::new()
            .connect_http("https://ethereum.reth.rs/rpc".parse().unwrap());

        let err = provider.get_resolver("ur.integration-tests.eth").await.unwrap_err();
        assert!(matches!(err, EnsError::RequiresUniversalResolver(_)));
    }

    #[tokio::test]
    async fn test_multicoin_resolver_integration() {
        let provider = ProviderBuilder::new()
            .connect_http("https://ethereum.reth.rs/rpc".parse().unwrap());

        let res = provider
            .resolve_name_for_coin_type(
                "coins.integration-tests.eth",
                coin_type::evm_chain(8453),
            )
            .await
            .unwrap();
        assert_eq!(
            res.as_ref(),
            address!("0xa66E90D515F576f49Af2dF40952476D56F72A420").as_slice()
        );
    }

    #[tokio::test]
    async fn test_reverse_resolver_integration() {
        let provider = ProviderBuilder::new()
            .connect_http("https://ethereum.reth.rs/rpc".parse().unwrap());

        let res = provider
            .lookup_address(&address!("0xeE9eeaAB0Bb7D9B969D701f6f8212609EDeA252E"))
            .await
            .unwrap();
        assert_eq!(res, "devrel.enslabs.eth");
    }
}
