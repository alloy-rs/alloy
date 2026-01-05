#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! ENS Name resolving utilities.

use alloy_primitives::{address, Address, Keccak256, B256};
use std::{borrow::Cow, str::FromStr};

/// ENS registry address (`0x00000000000C2E074eC69A0dFb2997BA6C7d2e1e`)
pub const ENS_ADDRESS: Address = address!("0x00000000000C2E074eC69A0dFb2997BA6C7d2e1e");

/// ENS const for registrar domain
pub const ENS_REVERSE_REGISTRAR_DOMAIN: &str = "addr.reverse";

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

    // ENS Registry and Resolver contracts.
    sol! {
        /// ENS Registry contract.
        #[sol(rpc)]
        contract EnsRegistry {
            /// Returns the resolver for the specified node.
            function resolver(bytes32 node) view returns (address);

            /// returns the owner of this node
            function owner(bytes32 node) view returns (address);
        }

        /// ENS Resolver interface.
        #[sol(rpc)]
        contract EnsResolver {
            /// Returns the address associated with the specified node.
            function addr(bytes32 node) view returns (address);

            /// Returns the name associated with an ENS node, for reverse records.
            function name(bytes32 node) view returns (string);

            /// Returns the txt associated with an ENS node
            function text(bytes32 node,string calldata key) view virtual returns (string memory);
        }

        /// ENS Reverse Registrar contract
        #[sol(rpc)]
        contract ReverseRegistrar {}
    }

    /// Error type for ENS resolution.
    #[derive(Debug, thiserror::Error)]
    pub enum EnsError {
        /// Failed to get resolver from the ENS registry.
        #[error("Failed to get resolver from the ENS registry: {0}")]
        Resolver(alloy_contract::Error),
        /// Failed to get resolver from the ENS registry.
        #[error("ENS resolver not found for name {0:?}")]
        ResolverNotFound(String),
        /// Failed to get reverse registrar from the ENS registry.
        #[error("Failed to get reverse registrar from the ENS registry: {0}")]
        RevRegistrar(alloy_contract::Error),
        /// Failed to get reverse registrar from the ENS registry.
        #[error("ENS reverse registrar not found for addr.reverse")]
        ReverseRegistrarNotFound,
        /// Failed to lookup ENS name from an address.
        #[error("Failed to lookup ENS name from an address: {0}")]
        Lookup(alloy_contract::Error),
        /// Failed to resolve ENS name to an address.
        #[error("Failed to resolve ENS name to an address: {0}")]
        Resolve(alloy_contract::Error),
        /// Failed to get txt records of ENS name.
        #[error("Failed to resolve txt record: {0}")]
        ResolveTxtRecord(alloy_contract::Error),
    }
}

#[cfg(feature = "provider")]
mod provider {
    use crate::{
        namehash, reverse_address, EnsError, EnsRegistry, EnsResolver::EnsResolverInstance,
        ReverseRegistrar::ReverseRegistrarInstance, ENS_ADDRESS, ENS_REVERSE_REGISTRAR_DOMAIN,
    };
    use alloy_primitives::{Address, B256};
    use alloy_provider::{Network, Provider};

    /// Extension trait for ENS contract calls.
    #[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
    #[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
    pub trait ProviderEnsExt<N: alloy_provider::Network, P: Provider<N>> {
        /// Returns the resolver for the specified node. The `&str` is only used for error messages.
        async fn get_resolver(
            &self,
            node: B256,
            error_name: &str,
        ) -> Result<EnsResolverInstance<&P, N>, EnsError>;

        /// Returns the reverse registrar for the specified node.
        async fn get_reverse_registrar(&self) -> Result<ReverseRegistrarInstance<&P, N>, EnsError>;

        /// Performs a forward lookup of an ENS name to an address.
        async fn resolve_name(&self, name: &str) -> Result<Address, EnsError> {
            let node = namehash(name);
            let resolver = self.get_resolver(node, name).await?;
            let addr = resolver.addr(node).call().await.map_err(EnsError::Resolve)?;

            Ok(addr)
        }

        /// Performs a reverse lookup of an address to an ENS name.
        async fn lookup_address(&self, address: &Address) -> Result<String, EnsError> {
            let name = reverse_address(address);
            let node = namehash(&name);
            let resolver = self.get_resolver(node, &name).await?;
            let name = resolver.name(node).call().await.map_err(EnsError::Lookup)?;
            Ok(name)
        }

        /// Performs a txt lookup of an address to an ENS name.
        async fn lookup_txt(&self, name: &str, key: &str) -> Result<String, EnsError> {
            let node = namehash(name);
            let resolver = self.get_resolver(node, name).await?;
            let txt_value = resolver
                .text(node, key.to_string())
                .call()
                .await
                .map_err(EnsError::ResolveTxtRecord)?;
            Ok(txt_value)
        }
    }

    #[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
    #[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
    impl<N, P> ProviderEnsExt<N, P> for P
    where
        P: Provider<N>,
        N: Network,
    {
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
    }
}

/// Returns the ENS namehash as specified in [EIP-137](https://eips.ethereum.org/EIPS/eip-137)
pub fn namehash(name: &str) -> B256 {
    if name.is_empty() {
        return B256::ZERO;
    }

    // Remove the variation selector `U+FE0F` if present.
    const VARIATION_SELECTOR: char = '\u{fe0f}';
    let name = if name.contains(VARIATION_SELECTOR) {
        Cow::Owned(name.replace(VARIATION_SELECTOR, ""))
    } else {
        Cow::Borrowed(name)
    };

    // Generate the node starting from the right.
    // This buffer is `[node @ [u8; 32], label_hash @ [u8; 32]]`.
    let mut buffer = [0u8; 64];
    for label in name.rsplit('.') {
        // node = keccak256([node, keccak256(label)])

        // Hash the label.
        let mut label_hasher = Keccak256::new();
        label_hasher.update(label.as_bytes());
        label_hasher.finalize_into(&mut buffer[32..]);

        // Hash both the node and the label hash, writing into the node.
        let mut buffer_hasher = Keccak256::new();
        buffer_hasher.update(buffer.as_slice());
        buffer_hasher.finalize_into(&mut buffer[..32]);
    }
    buffer[..32].try_into().unwrap()
}

/// Returns the reverse-registrar name of an address.
pub fn reverse_address(addr: &Address) -> String {
    format!("{addr:x}.{ENS_REVERSE_REGISTRAR_DOMAIN}")
}

#[cfg(test)]
mod test {
    use super::*;
    use alloy_primitives::hex;

    fn assert_hex(hash: B256, val: &str) {
        assert_eq!(hash.0[..], hex::decode(val).unwrap()[..]);
    }

    #[test]
    fn test_namehash() {
        for (name, expected) in &[
            ("", "0x0000000000000000000000000000000000000000000000000000000000000000"),
            ("eth", "0x93cdeb708b7545dc668eb9280176169d1c33cfd8ed6f04690a0bcc88a93fc4ae"),
            ("foo.eth", "0xde9b09fd7c5f901e23a3f19fecc54828e9c848539801e86591bd9801b019f84f"),
            ("alice.eth", "0x787192fc5378cc32aa956ddfdedbf26b24e8d78e40109add0eea2c1a012c3dec"),
            ("ret↩️rn.eth", "0x3de5f4c02db61b221e7de7f1c40e29b6e2f07eb48d65bf7e304715cd9ed33b24"),
        ] {
            assert_hex(namehash(name), expected);
        }
    }

    #[test]
    fn test_reverse_address() {
        for (addr, expected) in [
            (
                "0x314159265dd8dbb310642f98f50c066173c1259b",
                "314159265dd8dbb310642f98f50c066173c1259b.addr.reverse",
            ),
            (
                "0x28679A1a632125fbBf7A68d850E50623194A709E",
                "28679a1a632125fbbf7a68d850e50623194a709e.addr.reverse",
            ),
        ] {
            assert_eq!(reverse_address(&addr.parse().unwrap()), expected, "{addr}");
        }
    }

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
}

#[cfg(all(test, feature = "provider"))]
mod tests {
    use super::*;
    use alloy_primitives::address;
    use alloy_provider::ProviderBuilder;

    #[tokio::test]
    async fn test_reverse_registrar_fetching_mainnet() {
        let provider = ProviderBuilder::new()
            .connect_http("https://reth-ethereum.ithaca.xyz/rpc".parse().unwrap());

        let res = provider.get_reverse_registrar().await;
        assert_eq!(*res.unwrap().address(), address!("0xa58E81fe9b61B5c3fE2AFD33CF304c454AbFc7Cb"));
    }

    #[tokio::test]
    async fn test_pub_resolver_fetching_mainnet() {
        let provider = ProviderBuilder::new()
            .connect_http("https://reth-ethereum.ithaca.xyz/rpc".parse().unwrap());

        let name = "vitalik.eth";
        let node = namehash(name);
        let res = provider.get_resolver(node, name).await;
        assert_eq!(*res.unwrap().address(), address!("0x231b0Ee14048e9dCcD1d247744d114a4EB5E8E63"));
    }
    #[tokio::test]
    async fn test_pub_resolver_text() {
        let provider = ProviderBuilder::new()
            .connect_http("http://reth-ethereum.ithaca.xyz/rpc".parse().unwrap());

        let name = "vitalik.eth";
        let node = namehash(name);
        let res = provider.get_resolver(node, name).await.unwrap();
        let txt = res.text(node, "avatar".to_string()).call().await.unwrap();
        assert_eq!(txt, "https://euc.li/vitalik.eth")
    }

    #[tokio::test]
    async fn test_pub_resolver_fetching_txt() {
        let provider = ProviderBuilder::new()
            .connect_http("http://reth-ethereum.ithaca.xyz/rpc".parse().unwrap());

        let name = "vitalik.eth";
        let res = provider.lookup_txt(name, "avatar").await.unwrap();
        assert_eq!(res, "https://euc.li/vitalik.eth")
    }
}
