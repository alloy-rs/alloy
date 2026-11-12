use crate::{ContractInstance, Error, Result};
use alloy_dyn_abi::{
    DecodedError, DecodedEvent, DynSolValue, ErrorExt, EventExt, FunctionExt, JsonAbiExt,
};
use alloy_json_abi::{Error as JsonAbiError, Event, Function, JsonAbi};
use alloy_primitives::{
    map::{B256HashMap, FbHashMap, SelectorHashMap},
    Address, FixedBytes, LogData, Selector, B256,
};
use std::collections::BTreeMap;

/// A smart contract interface.
#[derive(Clone, Debug)]
pub struct Interface {
    abi: JsonAbi,
    functions: SelectorHashMap<(String, usize)>,
    events: B256HashMap<(String, usize)>,
    errors: SelectorHashMap<(String, usize)>,
}

impl Interface {
    /// Creates a new contract interface from the provided ABI.
    pub fn new(abi: JsonAbi) -> Self {
        let functions = create_mapping(&abi.functions, Function::selector);
        let events = create_mapping(&abi.events, Event::selector);
        let errors = create_mapping(&abi.errors, JsonAbiError::selector);
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

    /// ABI-decodes argument data according to the function's input types.
    ///
    /// `data` must not include the four-byte function selector. To decode full calldata, pass the
    /// bytes after the selector.
    ///
    /// # Note
    ///
    /// If the function exists multiple times and you want to use one of the overloaded versions,
    /// consider using [`Self::decode_input_with_selector`].
    ///
    /// # Examples
    ///
    /// ```
    /// use alloy_contract::Interface;
    /// use alloy_dyn_abi::DynSolValue;
    /// use alloy_json_abi::JsonAbi;
    /// use alloy_primitives::U256;
    ///
    /// let interface = Interface::new(JsonAbi::parse(["function setValue(uint256 value)"])?);
    /// let args = [DynSolValue::Uint(U256::from(42), 256)];
    /// let calldata = interface.encode_input("setValue", &args)?;
    ///
    /// let decoded = interface.decode_input("setValue", &calldata[4..])?;
    /// assert_eq!(decoded, args);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn decode_input(&self, name: &str, data: &[u8]) -> Result<Vec<DynSolValue>> {
        self.get_from_name(name)?.abi_decode_input(data).map_err(Into::into)
    }

    /// Decodes argument data using the function identified by `selector`.
    ///
    /// `data` must not include the four-byte function selector. The `selector` argument selects the
    /// function from the ABI; it is not decoded from or validated against `data`.
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
    ///
    /// # Examples
    ///
    /// ```
    /// use alloy_contract::Interface;
    /// use alloy_dyn_abi::DynSolValue;
    /// use alloy_json_abi::JsonAbi;
    /// use alloy_primitives::{address, keccak256, LogData, U256};
    ///
    /// let abi = JsonAbi::parse(["event Counted(address indexed caller, uint128 value)"])?;
    /// let interface = Interface::new(abi);
    ///
    /// let caller = address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
    /// let value = DynSolValue::Uint(U256::from(42), 128);
    /// let log = LogData::new_unchecked(
    ///     vec![keccak256("Counted(address,uint128)"), caller.into_word()],
    ///     value.abi_encode().into(),
    /// );
    ///
    /// let decoded = interface.decode_event("Counted", &log)?;
    /// assert_eq!(decoded.indexed, [DynSolValue::Address(caller)]);
    /// assert_eq!(decoded.body, [value]);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn decode_event(&self, name: &str, log: &LogData) -> Result<DecodedEvent> {
        self.get_event_from_name(name)?.decode_log(log).map_err(Into::into)
    }

    /// Decodes the given log data as the event with the given signature hash.
    ///
    /// For non-anonymous events, `selector` must match the first topic. For anonymous events, it
    /// only selects the event from the ABI.
    pub fn decode_event_with_selector(
        &self,
        selector: &B256,
        log: &LogData,
    ) -> Result<DecodedEvent> {
        self.get_event_from_selector(selector)?.decode_log(log).map_err(Into::into)
    }

    /// Decodes the given revert data as the first error with the given name.
    ///
    /// `data` must start with the matching four-byte error selector.
    ///
    /// # Note
    ///
    /// If there are multiple errors with the same name, consider using
    /// [`Self::decode_error_with_selector`].
    pub fn decode_error(&self, name: &str, data: &[u8]) -> Result<DecodedError> {
        self.get_error_from_name(name)?.decode_error(data).map_err(Into::into)
    }

    /// Decodes the given revert data as the error with the given selector.
    ///
    /// `data` must start with the four-byte `selector`.
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

    pub(crate) fn get_error_from_name(&self, name: &str) -> Result<&JsonAbiError> {
        self.abi
            .error(name)
            .and_then(|r| r.first())
            .ok_or_else(|| Error::UnknownAbiError(name.to_string()))
    }

    pub(crate) fn get_error_from_selector(&self, selector: &Selector) -> Result<&JsonAbiError> {
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
    use alloy_dyn_abi::Error as DynAbiError;
    use alloy_primitives::{address, keccak256, U256};
    use alloy_sol_types::{sol, SolError, SolEvent};

    fn test_abi() -> JsonAbi {
        JsonAbi::parse([
            "event Counted(address indexed caller, uint128 value)",
            "error Overflow(uint128 limit)",
        ])
        .unwrap()
    }

    #[test]
    fn unknown_event() {
        let interface = Interface::new(test_abi());
        let log = LogData::default();

        let err = interface.decode_event("NonExistent", &log).unwrap_err();
        assert!(matches!(err, Error::UnknownEvent(_)));
        let err = interface.decode_event_with_selector(&B256::ZERO, &log).unwrap_err();
        assert!(matches!(err, Error::UnknownEventSelector(_)));
    }

    #[test]
    fn unknown_error() {
        let interface = Interface::new(test_abi());

        let err = interface.decode_error("NonExistent", &[]).unwrap_err();
        assert!(matches!(err, Error::UnknownAbiError(_)));
        let err = interface.decode_error_with_selector(&Selector::ZERO, &[]).unwrap_err();
        assert!(matches!(err, Error::UnknownAbiErrorSelector(_)));
    }

    #[test]
    fn decode_event_roundtrip() {
        sol! {
            event Counted(address indexed caller, uint128 value);
        }

        let caller = address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
        let value: u128 = 42;
        let log_data = Counted { caller, value }.encode_log_data();

        let interface = Interface::new(test_abi());

        let decoded = interface.decode_event("Counted", &log_data).unwrap();
        assert_eq!(decoded.selector, Some(Counted::SIGNATURE_HASH));
        assert_eq!(decoded.indexed, [DynSolValue::Address(caller)]);
        assert_eq!(decoded.body, [DynSolValue::Uint(U256::from(value), 128)]);

        let decoded_by_sel =
            interface.decode_event_with_selector(&Counted::SIGNATURE_HASH, &log_data).unwrap();
        assert_eq!(decoded, decoded_by_sel);
    }

    #[test]
    fn decode_error_roundtrip() {
        sol! {
            error Overflow(uint128 limit);
        }

        let limit: u128 = u128::MAX;
        let encoded = Overflow { limit }.abi_encode();

        let interface = Interface::new(test_abi());
        let decoded = interface.decode_error("Overflow", &encoded).unwrap();
        assert_eq!(decoded.body, [DynSolValue::Uint(U256::from(limit), 128)]);

        let selector = Selector::from(Overflow::SELECTOR);
        let decoded_by_sel = interface.decode_error_with_selector(&selector, &encoded).unwrap();
        assert_eq!(decoded, decoded_by_sel);
    }

    #[test]
    fn decode_event_malformed() {
        let interface = Interface::new(test_abi());
        let selector = interface.get_event_from_name("Counted").unwrap().selector();
        let caller = address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045");

        // missing the indexed `caller` topic
        let log = LogData::new_unchecked(vec![selector], vec![0u8; 32].into());
        let err = interface.decode_event("Counted", &log).unwrap_err();
        assert!(matches!(
            err,
            Error::AbiError(DynAbiError::TopicLengthMismatch { expected: 2, actual: 1 })
        ));
        let err = interface.decode_event_with_selector(&selector, &log).unwrap_err();
        assert!(matches!(
            err,
            Error::AbiError(DynAbiError::TopicLengthMismatch { expected: 2, actual: 1 })
        ));

        // correct topics, the body is one byte short of a word
        let log = LogData::new_unchecked(vec![selector, caller.into_word()], vec![0u8; 31].into());
        let err = interface.decode_event("Counted", &log).unwrap_err();
        assert!(matches!(err, Error::AbiError(DynAbiError::SolTypes(_))));
    }

    #[test]
    fn decode_error_malformed() {
        let interface = Interface::new(test_abi());
        let selector = interface.get_error_from_name("Overflow").unwrap().selector();

        // selector only, no params
        let err = interface.decode_error("Overflow", selector.as_slice()).unwrap_err();
        assert!(matches!(err, Error::AbiError(DynAbiError::SolTypes(_))));

        // selector + 1 garbage byte instead of 32
        let mut truncated_body = selector.to_vec();
        truncated_body.push(0xFF);
        let err = interface.decode_error_with_selector(&selector, &truncated_body).unwrap_err();
        assert!(matches!(err, Error::AbiError(DynAbiError::SolTypes(_))));

        // well-formed body, but the selector does not match Overflow
        let mut unknown_selector_data = vec![0xde, 0xad, 0xbe, 0xef];
        unknown_selector_data.extend_from_slice(&[0u8; 32]);
        let err = interface.decode_error("Overflow", &unknown_selector_data).unwrap_err();
        assert!(matches!(err, Error::AbiError(DynAbiError::SelectorMismatch { .. })));

        // empty input should return an error without panicking
        let err = interface.decode_error("Overflow", &[]).unwrap_err();
        assert!(matches!(err, Error::AbiError(DynAbiError::SelectorMismatch { .. })));
    }

    #[test]
    fn decode_overloaded_event() {
        let abi = JsonAbi::parse(["event Dup(uint256 a)", "event Dup(address a)"]).unwrap();
        let interface = Interface::new(abi);

        let selector = interface.abi().events["Dup"]
            .iter()
            .find(|e| e.signature() == "Dup(address)")
            .unwrap()
            .selector();
        let log = LogData::new_unchecked(vec![selector], vec![0u8; 32].into());

        // by name resolves to the first overload, but its selector does not match the log
        let err = interface.decode_event("Dup", &log).unwrap_err();
        assert!(matches!(err, Error::AbiError(DynAbiError::EventSignatureMismatch { .. })));

        let decoded = interface.decode_event_with_selector(&selector, &log).unwrap();
        assert_eq!(decoded.body, [DynSolValue::Address(Address::ZERO)]);
    }

    #[test]
    fn decode_overloaded_error() {
        let abi = JsonAbi::parse(["error Dup(uint256 a)", "error Dup(address a)"]).unwrap();
        let interface = Interface::new(abi);

        let selector = interface.abi().errors["Dup"]
            .iter()
            .find(|e| e.signature() == "Dup(address)")
            .unwrap()
            .selector();
        let mut data = selector.to_vec();
        data.extend_from_slice(&[0u8; 32]);

        // by name resolves to the first overload, but its selector does not match the data
        let err = interface.decode_error("Dup", &data).unwrap_err();
        assert!(matches!(err, Error::AbiError(DynAbiError::SelectorMismatch { .. })));

        let decoded = interface.decode_error_with_selector(&selector, &data).unwrap();
        assert_eq!(decoded.body, [DynSolValue::Address(Address::ZERO)]);
    }

    #[test]
    fn decode_anonymous_event() {
        let abi = JsonAbi::parse(["event Anon(address indexed caller) anonymous"]).unwrap();
        let interface = Interface::new(abi);

        let caller = address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
        let log = LogData::new_unchecked(vec![caller.into_word()], Default::default());

        // the selector only identifies the event in the ABI, it is not part of the log
        let selector = keccak256("Anon(address)");
        let decoded = interface.decode_event_with_selector(&selector, &log).unwrap();
        assert_eq!(decoded.selector, None);
        assert_eq!(decoded.indexed, [DynSolValue::Address(caller)]);
        assert!(decoded.body.is_empty());
        assert_eq!(decoded, interface.decode_event("Anon", &log).unwrap());
    }
}
