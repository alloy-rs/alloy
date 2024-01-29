use crate::{ContractInstance, Error, Result};
use alloy_dyn_abi::{DynSolValue, FunctionExt, JsonAbiExt};
use alloy_json_abi::{Function, JsonAbi};
use alloy_primitives::{Address, Selector};
use std::collections::HashMap;

/// A smart contract interface.
#[derive(Debug, Clone)]
pub struct Interface {
    abi: JsonAbi,
    functions: HashMap<Selector, (String, usize)>,
}

impl From<JsonAbi> for Interface {
    fn from(abi: JsonAbi) -> Self {
        Self { abi, functions: Default::default() }
    }
}

// TODO: events/errors
impl Interface {
    /// Returns the ABI encoded data (including the selector) for the provided function and
    /// arguments.
    ///
    /// # Note
    ///
    /// If the function exists multiple times and you want to use one of the overloaded versions,
    /// consider using [`Self::encode_input_with_selector`].
    pub fn encode_input(&self, name: &str, args: &[DynSolValue]) -> Result<Vec<u8>> {
        self.get_from_name(name)?.abi_encode_input(args).map_err(Into::into)
    }

    /// Returns the ABI encoded data (including the selector) for the function with the provided
    /// selector and arguments.
    pub fn encode_input_with_selector(
        &self,
        selector: &Selector,
        args: &[DynSolValue],
    ) -> Result<Vec<u8>> {
        self.get_from_selector(selector)?.abi_encode_input(args).map_err(Into::into)
    }

    /// ABI-decodes the given data according to the function's types.
    ///
    /// # Note
    ///
    /// If the function exists multiple times and you want to use one of the overloaded versions,
    /// consider using [`Self::decode_input_with_selector`].
    pub fn decode_input(
        &self,
        name: &str,
        data: &[u8],
        validate: bool,
    ) -> Result<Vec<DynSolValue>> {
        self.get_from_name(name)?.abi_decode_input(data, validate).map_err(Into::into)
    }

    /// Decode the provided ABI encoded bytes as the input of the provided function selector.
    pub fn decode_input_with_selector(
        &self,
        selector: &Selector,
        data: &[u8],
        validate: bool,
    ) -> Result<Vec<DynSolValue>> {
        self.get_from_selector(selector)?.abi_decode_input(data, validate).map_err(Into::into)
    }

    /// Decode the provided ABI encoded bytes as the output of the first function with the given
    /// name.
    ///
    /// # Note
    ///
    /// If there are multiple functions with the same name, consider using
    /// [`Self::decode_output_with_selector`]
    pub fn decode_output(
        &self,
        name: &str,
        data: &[u8],
        validate: bool,
    ) -> Result<Vec<DynSolValue>> {
        self.get_from_name(name)?.abi_decode_output(data, validate).map_err(Into::into)
    }

    /// Decode the provided ABI encoded bytes as the output of the provided function selector.
    pub fn decode_output_with_selector(
        &self,
        selector: &Selector,
        data: &[u8],
        validate: bool,
    ) -> Result<Vec<DynSolValue>> {
        self.get_from_selector(selector)?.abi_decode_output(data, validate).map_err(Into::into)
    }

    /// Returns a reference to the contract's ABI.
    pub const fn abi(&self) -> &JsonAbi {
        &self.abi
    }

    /// Consumes the interface, returning the inner ABI.
    pub fn into_abi(self) -> JsonAbi {
        self.abi
    }

    pub(crate) fn get_from_name(&self, name: &str) -> Result<&Function> {
        self.abi
            .function(name)
            .and_then(|r| r.first())
            .ok_or_else(|| Error::UnknownFunction(name.to_string()))
    }

    pub(crate) fn get_from_selector(&self, selector: &Selector) -> Result<&Function> {
        self.functions
            .get(selector)
            .map(|(name, index)| &self.abi.functions[name][*index])
            .ok_or_else(|| Error::UnknownSelector(*selector))
    }

    /// Create a [`ContractInstance`] from this ABI for a contract at the given address.
    pub const fn connect<P>(self, address: Address, provider: P) -> ContractInstance<P> {
        ContractInstance::new(address, provider, self)
    }
}
