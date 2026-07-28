use crate::{ContractInstance, Error, Result};
use alloy_dyn_abi::{
    DecodedError, DecodedEvent, DynSolValue, ErrorExt, EventExt, FunctionExt, JsonAbiExt,
};
use alloy_json_abi::{Error as AbiError, Event, Function, JsonAbi};
use alloy_primitives::{
    map::{FbHashMap, SelectorHashMap},
    Address, FixedBytes, LogData, Selector, B256,
};
use std::collections::BTreeMap;

/// A smart contract interface.
#[derive(Clone, Debug)]
pub struct Interface {
    abi: JsonAbi,
    functions: SelectorHashMap<(String, usize)>,
    events: FbHashMap<32, (String, usize)>,
    errors: SelectorHashMap<(String, usize)>,
}

impl Interface {
    /// Creates a new contract interface from the provided ABI.
    pub fn new(abi: JsonAbi) -> Self {
        let functions = create_mapping(&abi.functions, Function::selector);
        let events = create_mapping(&abi.events, Event::selector);
        let errors = create_mapping(&abi.errors, AbiError::selector);
        Self { abi, functions, events, errors }
    }

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
    pub fn decode_input(&self, name: &str, data: &[u8]) -> Result<Vec<DynSolValue>> {
        self.get_from_name(name)?.abi_decode_input(data).map_err(Into::into)
    }

    /// Decode the provided ABI encoded bytes as the input of the provided function selector.
    pub fn decode_input_with_selector(
        &self,
        selector: &Selector,
        data: &[u8],
    ) -> Result<Vec<DynSolValue>> {
        self.get_from_selector(selector)?.abi_decode_input(data).map_err(Into::into)
    }

    /// Decode the provided ABI encoded bytes as the output of the first function with the given
    /// name.
    ///
    /// # Note
    ///
    /// If there are multiple functions with the same name, consider using
    /// [`Self::decode_output_with_selector`]
    pub fn decode_output(&self, name: &str, data: &[u8]) -> Result<Vec<DynSolValue>> {
        self.get_from_name(name)?.abi_decode_output(data).map_err(Into::into)
    }

    /// Decode the provided ABI encoded bytes as the output of the provided function selector.
    pub fn decode_output_with_selector(
        &self,
        selector: &Selector,
        data: &[u8],
    ) -> Result<Vec<DynSolValue>> {
        self.get_from_selector(selector)?.abi_decode_output(data).map_err(Into::into)
    }

    /// Decodes the given log data as the first event with the given name.
    ///
    /// # Note
    ///
    /// If there are multiple events with the same name, consider using
    /// [`Self::decode_event_with_selector`].
    pub fn decode_event(&self, name: &str, log: &LogData) -> Result<DecodedEvent> {
        self.get_event_from_name(name)?.decode_log(log).map_err(Into::into)
    }

    /// Decodes the given log data as the event with the given selector.
    pub fn decode_event_with_selector(
        &self,
        selector: &B256,
        log: &LogData,
    ) -> Result<DecodedEvent> {
        self.get_event_from_selector(selector)?.decode_log(log).map_err(Into::into)
    }

    /// Decodes the given revert data as the first error with the given name.
    ///
    /// # Note
    ///
    /// If there are multiple errors with the same name, consider using
    /// [`Self::decode_error_with_selector`].
    pub fn decode_error(&self, name: &str, data: &[u8]) -> Result<DecodedError> {
        self.get_error_from_name(name)?.decode_error(data).map_err(Into::into)
    }

    /// Decodes the given revert data as the error with the given selector.
    pub fn decode_error_with_selector(
        &self,
        selector: &Selector,
        data: &[u8],
    ) -> Result<DecodedError> {
        self.get_error_from_selector(selector)?.decode_error(data).map_err(Into::into)
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

    pub(crate) fn get_event_from_name(&self, name: &str) -> Result<&Event> {
        self.abi
            .event(name)
            .and_then(|r| r.first())
            .ok_or_else(|| Error::UnknownEvent(name.to_string()))
    }

    pub(crate) fn get_event_from_selector(&self, selector: &B256) -> Result<&Event> {
        self.events
            .get(selector)
            .map(|(name, index)| &self.abi.events[name][*index])
            .ok_or_else(|| Error::UnknownEventSelector(*selector))
    }

    pub(crate) fn get_error_from_name(&self, name: &str) -> Result<&AbiError> {
        self.abi
            .error(name)
            .and_then(|r| r.first())
            .ok_or_else(|| Error::UnknownAbiError(name.to_string()))
    }

    pub(crate) fn get_error_from_selector(&self, selector: &Selector) -> Result<&AbiError> {
        self.errors
            .get(selector)
            .map(|(name, index)| &self.abi.errors[name][*index])
            .ok_or_else(|| Error::UnknownAbiErrorSelector(*selector))
    }

    /// Create a [`ContractInstance`] from this ABI for a contract at the given address.
    pub const fn connect<P, N>(self, address: Address, provider: P) -> ContractInstance<P, N> {
        ContractInstance::new(address, provider, self)
    }
}

/// Utility function for creating a mapping between a unique signature and a
/// name-index pair for accessing contract ABI items.
fn create_mapping<const N: usize, T, F>(
    elements: &BTreeMap<String, Vec<T>>,
    signature: F,
) -> FbHashMap<N, (String, usize)>
where
    F: Fn(&T) -> FixedBytes<N> + Copy,
{
    elements
        .iter()
        .flat_map(|(name, sub_elements)| {
            sub_elements
                .iter()
                .enumerate()
                .map(move |(index, element)| (signature(element), (name.to_owned(), index)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, U256};
    use alloy_sol_types::{sol, SolError, SolEvent};

    fn test_abi() -> JsonAbi {
        serde_json::from_str(
            r#"[
            {
                "type": "function",
                "name": "increment",
                "inputs": [{"name": "amount", "type": "uint128"}],
                "outputs": [{"name": "", "type": "uint128"}],
                "stateMutability": "nonpayable"
            },
            {
                "type": "event",
                "name": "Counted",
                "inputs": [
                    {"name": "caller", "type": "address", "indexed": true},
                    {"name": "value", "type": "uint128", "indexed": false}
                ],
                "anonymous": false
            },
            {
                "type": "error",
                "name": "Overflow",
                "inputs": [
                    {"name": "limit", "type": "uint128"}
                ]
            }
        ]"#,
        )
        .unwrap()
    }

    #[test]
    fn unknown_event_error() {
        let interface = Interface::new(test_abi());
        let log = LogData::new_unchecked(vec![], Default::default());
        let err = interface.decode_event("NonExistent", &log).unwrap_err();
        assert!(matches!(err, Error::UnknownEvent(_)));
    }

    #[test]
    fn unknown_event_selector_error() {
        let interface = Interface::new(test_abi());
        let log = LogData::new_unchecked(vec![], Default::default());
        let err = interface.decode_event_with_selector(&B256::ZERO, &log).unwrap_err();
        assert!(matches!(err, Error::UnknownEventSelector(_)));
    }

    #[test]
    fn unknown_abi_error() {
        let interface = Interface::new(test_abi());
        let err = interface.decode_error("NonExistent", &[]).unwrap_err();
        assert!(matches!(err, Error::UnknownAbiError(_)));
    }

    #[test]
    fn unknown_abi_error_selector() {
        let interface = Interface::new(test_abi());
        let err = interface.decode_error_with_selector(&Selector::ZERO, &[]).unwrap_err();
        assert!(matches!(err, Error::UnknownAbiErrorSelector(_)));
    }

    #[test]
    fn event_lookup_by_selector() {
        let interface = Interface::new(test_abi());
        let by_name = interface.get_event_from_name("Counted").unwrap();
        let selector = by_name.selector();
        let by_selector = interface.get_event_from_selector(&selector).unwrap();
        assert_eq!(by_name.name, by_selector.name);
    }

    #[test]
    fn error_lookup_by_selector() {
        let interface = Interface::new(test_abi());
        let by_name = interface.get_error_from_name("Overflow").unwrap();
        let selector = by_name.selector();
        let by_selector = interface.get_error_from_selector(&selector).unwrap();
        assert_eq!(by_name.name, by_selector.name);
    }

    #[test]
    fn decode_event_roundtrip() {
        sol! {
            #[derive(Debug, PartialEq)]
            event Counted(address indexed caller, uint128 value);
        }

        let caller = address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
        let value: u128 = 42;
        let log_data = Counted { caller, value }.encode_log_data();

        let interface = Interface::new(test_abi());
        let decoded = interface.decode_event("Counted", &log_data).unwrap();
        assert_eq!(decoded.indexed.len(), 1);
        assert_eq!(decoded.body.len(), 1);
        assert_eq!(decoded.indexed[0], DynSolValue::Address(caller));
        assert_eq!(decoded.body[0], DynSolValue::Uint(U256::from(value), 128));

        let selector = interface.get_event_from_name("Counted").unwrap().selector();
        let decoded_by_sel = interface.decode_event_with_selector(&selector, &log_data).unwrap();
        assert_eq!(decoded.body, decoded_by_sel.body);
        assert_eq!(decoded.indexed, decoded_by_sel.indexed);
    }

    #[test]
    fn decode_error_roundtrip() {
        sol! {
            #[derive(Debug, PartialEq)]
            error Overflow(uint128 limit);
        }

        let limit: u128 = u128::MAX;
        let encoded = Overflow { limit }.abi_encode();

        let interface = Interface::new(test_abi());
        let decoded = interface.decode_error("Overflow", &encoded).unwrap();
        assert_eq!(decoded.body.len(), 1);
        assert_eq!(decoded.body[0], DynSolValue::Uint(U256::from(limit), 128));

        let selector = interface.get_error_from_name("Overflow").unwrap().selector();
        let decoded_by_sel = interface.decode_error_with_selector(&selector, &encoded).unwrap();
        assert_eq!(decoded.body, decoded_by_sel.body);
    }

    #[test]
    fn decode_event_malformed_log() {
        let interface = Interface::new(test_abi());
        let selector = interface.get_event_from_name("Counted").unwrap().selector();
        // only selector topic, no indexed address or body
        let log = LogData::new_unchecked(vec![selector], Default::default());
        let err = interface.decode_event("Counted", &log).unwrap_err();
        assert!(matches!(err, Error::AbiError(_)));
    }

    #[test]
    fn decode_event_with_selector_malformed_log() {
        let interface = Interface::new(test_abi());
        let selector = interface.get_event_from_name("Counted").unwrap().selector();
        let log = LogData::new_unchecked(vec![selector], Default::default());
        let err = interface.decode_event_with_selector(&selector, &log).unwrap_err();
        assert!(matches!(err, Error::AbiError(_)));
    }

    #[test]
    fn decode_error_malformed_data() {
        let interface = Interface::new(test_abi());
        let selector = interface.get_error_from_name("Overflow").unwrap().selector();
        // selector only, no params
        let err = interface.decode_error("Overflow", selector.as_slice()).unwrap_err();
        assert!(matches!(err, Error::AbiError(_)));
    }

    #[test]
    fn decode_error_with_selector_malformed_data() {
        let interface = Interface::new(test_abi());
        let selector = interface.get_error_from_name("Overflow").unwrap().selector();
        // selector + 1 garbage byte instead of 32
        let mut bad_data = selector.to_vec();
        bad_data.push(0xFF);
        let err = interface.decode_error_with_selector(&selector, &bad_data).unwrap_err();
        assert!(matches!(err, Error::AbiError(_)));
    }
}
