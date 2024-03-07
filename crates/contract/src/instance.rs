use crate::{CallBuilder, Interface, Result};
use alloy_dyn_abi::DynSolValue;
use alloy_json_abi::{Function, JsonAbi};
use alloy_network::Network;
use alloy_primitives::{Address, Selector};
use alloy_providers::Provider;
use std::marker::PhantomData;

/// A handle to an Ethereum contract at a specific address.
///
/// A contract is an abstraction of an executable program on Ethereum. Every deployed contract has
/// an address, which is used to connect to it so that it may receive messages (transactions).
#[derive(Clone)]
pub struct ContractInstance<N, P> {
    address: Address,
    provider: P,
    interface: Interface,
    network: PhantomData<N>,
}

impl<N, P> ContractInstance<N, P> {
    /// Creates a new contract from the provided address, provider, and interface.
    #[inline]
    pub const fn new(address: Address, provider: P, interface: Interface) -> Self {
        Self { address, provider, interface, network: PhantomData }
    }

    /// Returns a reference to the contract's address.
    #[inline]
    pub const fn address(&self) -> &Address {
        &self.address
    }

    /// Sets the contract's address.
    #[inline]
    pub fn set_address(&mut self, address: Address) {
        self.address = address;
    }

    /// Returns a new contract instance at `address`.
    #[inline]
    pub fn at(mut self, address: Address) -> ContractInstance<N, P> {
        self.set_address(address);
        self
    }

    /// Returns a reference to the contract's ABI.
    #[inline]
    pub const fn abi(&self) -> &JsonAbi {
        self.interface.abi()
    }

    /// Returns a reference to the contract's provider.
    #[inline]
    pub const fn provider(&self) -> &P {
        &self.provider
    }
}

impl<N, P: Clone> ContractInstance<N, &P> {
    /// Clones the provider and returns a new contract instance with the cloned provider.
    #[inline]
    pub fn with_cloned_provider(self) -> ContractInstance<N, P> {
        ContractInstance {
            address: self.address,
            provider: self.provider.clone(),
            interface: self.interface,
            network: PhantomData,
        }
    }
}

impl<N: Network, P: Provider<N>> ContractInstance<N, P> {
    /// Returns a transaction builder for the provided function name.
    ///
    /// If there are multiple functions with the same name due to overloading, consider using
    /// the [`ContractInstance::function_from_selector`] method instead, since this will use the
    /// first match.
    pub fn function(
        &self,
        name: &str,
        args: &[DynSolValue],
    ) -> Result<CallBuilder<N, &P, Function>> {
        let function = self.interface.get_from_name(name)?;
        CallBuilder::new_dyn(&self.provider, function, args)
    }

    /// Returns a transaction builder for the provided function selector.
    pub fn function_from_selector(
        &self,
        selector: &Selector,
        args: &[DynSolValue],
    ) -> Result<CallBuilder<N, &P, Function>> {
        let function = self.interface.get_from_selector(selector)?;
        CallBuilder::new_dyn(&self.provider, function, args)
    }
}

impl<N, P> std::ops::Deref for ContractInstance<N, P> {
    type Target = Interface;

    fn deref(&self) -> &Self::Target {
        &self.interface
    }
}

impl<N, P> std::fmt::Debug for ContractInstance<N, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContractInstance").field("address", &self.address).finish()
    }
}
