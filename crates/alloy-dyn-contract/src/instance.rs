use alloy_dyn_abi::{DynSolValue, JsonAbiExt};
use alloy_json_abi::JsonAbi;
use alloy_primitives::{Address, Selector};
use alloy_providers::provider::TempProvider;

use crate::{error::Result, interface::Interface, CallBuilder};

/// A handle to an Ethereum contract at a specific address.
///
/// A contract is an abstraction of an executable program on Ethereum. Every deployed contract has
/// an address, which is used to connect to it so that it may receive messages (transactions).
pub struct ContractInstance<P> {
    address: Address,
    provider: P,
    interface: Interface,
}

impl<P> ContractInstance<P> {
    /// Creates a new contract from the provided address, provider, and interface.
    pub fn new(address: Address, provider: P, interface: Interface) -> Self {
        Self { address, provider, interface }
    }

    /// Returns the contract's address.
    pub fn address(&self) -> Address {
        self.address
    }

    /// Returns a reference to the contract's ABI.
    pub fn abi(&self) -> &JsonAbi {
        self.interface.abi()
    }

    /// Returns a reference to the contract's provider.
    pub fn provider_ref(&self) -> &P {
        &self.provider
    }
}

impl<P> ContractInstance<P>
where
    P: Clone,
{
    /// Returns a clone of the contract's provider.
    pub fn provider(&self) -> P {
        self.provider.clone()
    }

    /// Returns a new contract instance at `address`.
    ///
    /// Clones `self` internally
    #[must_use]
    pub fn at(&self, address: Address) -> ContractInstance<P> {
        let mut this = self.clone();
        this.address = address;
        this
    }
}

impl<P: TempProvider + Clone> ContractInstance<P> {
    /// Returns a transaction builder for the provided function name.
    /// If there are  multiple functions with the same name due to overloading, consider using
    /// the [`ContractInstance::function_from_selector`] method instead, since this will use the
    /// first match.
    pub fn function(&self, name: &str, args: &[DynSolValue]) -> Result<CallBuilder<P>> {
        let func = self.interface.get_from_name(name)?;
        let data = func.abi_encode_input(args)?;
        Ok(CallBuilder::new(self.provider.clone(), func.clone(), data.into()))
    }

    /// Returns a transaction builder for the provided function selector.
    pub fn function_from_selector(
        &self,
        selector: &Selector,
        args: &[DynSolValue],
    ) -> Result<CallBuilder<P>> {
        let func = self.interface.get_from_selector(selector)?;
        let data = func.abi_encode_input(args)?;
        Ok(CallBuilder::new(self.provider.clone(), func.clone(), data.into()))
    }
}

impl<P> Clone for ContractInstance<P>
where
    P: Clone,
{
    fn clone(&self) -> Self {
        ContractInstance {
            address: self.address.clone(),
            provider: self.provider.clone(),
            interface: self.interface.clone(),
        }
    }
}

impl<P> std::ops::Deref for ContractInstance<P> {
    type Target = Interface;

    fn deref(&self) -> &Self::Target {
        &self.interface
    }
}

impl<P> std::fmt::Debug for ContractInstance<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContractInstance").field("address", &self.address).finish()
    }
}
