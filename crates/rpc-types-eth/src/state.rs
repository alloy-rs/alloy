//! bindings for state overrides in eth_call

use crate::BlockOverrides;
use alloc::boxed::Box;
use alloy_eips::eip7702::constants::EIP7702_DELEGATION_DESIGNATOR;
use alloy_primitives::{
    map::{AddressHashMap, B256HashMap},
    Address, Bytes, B256, U256,
};

/// A builder type for [`StateOverride`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StateOverridesBuilder {
    overrides: StateOverride,
}

impl StateOverridesBuilder {
    /// Create a new StateOverridesBuilder.
    pub const fn new(map: AddressHashMap<AccountOverride>) -> Self {
        Self { overrides: map }
    }

    /// Creates a new [`StateOverridesBuilder`] with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self::new(StateOverride::with_capacity_and_hasher(capacity, Default::default()))
    }

    /// Adds an account override for a specific address.
    pub fn append(mut self, address: Address, account_override: AccountOverride) -> Self {
        self.overrides.insert(address, account_override);
        self
    }

    /// Helper `append` function that appends an optional override.
    pub fn append_opt<F>(self, f: F) -> Self
    where
        F: FnOnce() -> Option<(Address, AccountOverride)>,
    {
        if let Some((add, acc)) = f() {
            self.append(add, acc)
        } else {
            self
        }
    }

    /// Apply a function to the builder, returning the modified builder.
    pub fn apply<F>(self, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        f(self)
    }

    /// Adds multiple account overrides from an iterator.
    pub fn extend<I>(mut self, account_overrides: I) -> Self
    where
        I: IntoIterator<Item = (Address, AccountOverride)>,
    {
        self.overrides.extend(account_overrides);
        self
    }

    /// Get the underlying `StateOverride`.
    pub fn build(self) -> StateOverride {
        self.overrides
    }

    /// Configures an account override with a balance.
    pub fn with_balance(mut self, address: Address, balance: U256) -> Self {
        self.overrides.entry(address).or_default().set_balance(balance);
        self
    }

    /// Configures an account override with a nonce.
    pub fn with_nonce(mut self, address: Address, nonce: u64) -> Self {
        self.overrides.entry(address).or_default().set_nonce(nonce);
        self
    }

    /// Configures an account override with bytecode.
    pub fn with_code(mut self, address: Address, code: impl Into<Bytes>) -> Self {
        self.overrides.entry(address).or_default().set_code(code);
        self
    }

    /// Convenience function that sets overrides the `address` code with the EIP-7702 delegation
    /// designator for `delegation_address`
    pub fn with_7702_delegation_designator(
        self,
        address: Address,
        delegation_address: Address,
    ) -> Self {
        self.with_code(
            address,
            Bytes::from([&EIP7702_DELEGATION_DESIGNATOR, delegation_address.as_slice()].concat()),
        )
    }

    /// Configures an account override with state overrides.
    pub fn with_state(
        mut self,
        address: Address,
        state: impl IntoIterator<Item = (B256, B256)>,
    ) -> Self {
        self.overrides.entry(address).or_default().set_state(state);
        self
    }

    /// Configures an account override with state diffs.
    pub fn with_state_diff(
        mut self,
        address: Address,
        state_diff: impl IntoIterator<Item = (B256, B256)>,
    ) -> Self {
        self.overrides.entry(address).or_default().set_state_diff(state_diff);
        self
    }
}

impl FromIterator<(Address, AccountOverride)> for StateOverridesBuilder {
    fn from_iter<T: IntoIterator<Item = (Address, AccountOverride)>>(iter: T) -> Self {
        Self::new(StateOverride::from_iter(iter))
    }
}

/// A set of account overrides
pub type StateOverride = AddressHashMap<AccountOverride>;

/// Allows converting `StateOverridesBuilder` directly into `StateOverride`.
impl From<StateOverridesBuilder> for StateOverride {
    fn from(builder: StateOverridesBuilder) -> Self {
        builder.overrides
    }
}
/// Custom account override used in call
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default, rename_all = "camelCase", deny_unknown_fields))]
pub struct AccountOverride {
    /// Fake balance to set for the account before executing the call.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub balance: Option<U256>,
    /// Fake nonce to set for the account before executing the call.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "alloy_serde::quantity::opt"
        )
    )]
    pub nonce: Option<u64>,
    /// Fake EVM bytecode to inject into the account before executing the call.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub code: Option<Bytes>,
    /// Fake key-value mapping to override all slots in the account storage before executing the
    /// call.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub state: Option<B256HashMap<B256>>,
    /// Fake key-value mapping to override individual slots in the account storage before executing
    /// the call.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub state_diff: Option<B256HashMap<B256>>,
    /// Moves addresses precompile into the specified address. This move is done before the 'code'
    /// override is set. When the specified address is not a precompile, the behaviour is undefined
    /// and different clients might behave differently.
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            rename = "movePrecompileToAddress",
            alias = "MovePrecompileToAddress"
        )
    )]
    pub move_precompile_to: Option<Address>,
}

impl AccountOverride {
    /// Configures the bytecode override
    pub fn with_code(mut self, code: impl Into<Bytes>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Convenience function that sets overrides the code with the EIP-7702 delegation designator
    /// for `delegation_address`
    pub fn with_7702_delegation_designator(self, delegation_address: Address) -> Self {
        self.with_code(Bytes::from(
            [&EIP7702_DELEGATION_DESIGNATOR, delegation_address.as_slice()].concat(),
        ))
    }

    /// Configures the state overrides
    pub fn with_state(mut self, state: impl IntoIterator<Item = (B256, B256)>) -> Self {
        self.state = Some(state.into_iter().collect());
        self
    }

    /// Configures the state diffs
    pub fn with_state_diff(mut self, state_diff: impl IntoIterator<Item = (B256, B256)>) -> Self {
        self.state_diff = Some(state_diff.into_iter().collect());
        self
    }

    /// Configures the balance override
    pub const fn with_balance(mut self, balance: U256) -> Self {
        self.balance = Some(balance);
        self
    }

    /// Configures the nonce override
    pub const fn with_nonce(mut self, nonce: u64) -> Self {
        self.nonce = Some(nonce);
        self
    }

    /// Sets the bytecode override in place.
    pub fn set_code(&mut self, code: impl Into<Bytes>) {
        self.code = Some(code.into());
    }

    /// Sets the state overrides in place.
    pub fn set_state(&mut self, state: impl IntoIterator<Item = (B256, B256)>) {
        self.state = Some(state.into_iter().collect());
    }

    /// Sets the state diffs in place.
    pub fn set_state_diff(&mut self, state_diff: impl IntoIterator<Item = (B256, B256)>) {
        self.state_diff = Some(state_diff.into_iter().collect());
    }

    /// Sets the balance override in place.
    pub const fn set_balance(&mut self, balance: U256) {
        self.balance = Some(balance);
    }

    /// Sets the nonce override in place.
    pub const fn set_nonce(&mut self, nonce: u64) {
        self.nonce = Some(nonce);
    }

    /// Sets the move precompile address in place.
    pub const fn set_move_precompile_to(&mut self, address: Address) {
        self.move_precompile_to = Some(address);
    }

    /// Conditionally sets the bytecode override and returns self.
    pub fn with_code_opt(mut self, code: Option<impl Into<Bytes>>) -> Self {
        if let Some(code) = code {
            self.code = Some(code.into());
        }
        self
    }

    /// Convenience function that sets overrides the code with the EIP-7702 delegation designator
    /// for `delegation_address` if it is provided
    pub fn with_7702_delegation_designator_opt(self, delegation_address: Option<Address>) -> Self {
        if let Some(delegation_address) = delegation_address {
            self.with_7702_delegation_designator(delegation_address)
        } else {
            self
        }
    }

    /// Conditionally sets the balance override and returns self.
    pub const fn with_balance_opt(mut self, balance: Option<U256>) -> Self {
        if let Some(balance) = balance {
            self.balance = Some(balance);
        }
        self
    }

    /// Conditionally sets the nonce override and returns self.
    pub const fn with_nonce_opt(mut self, nonce: Option<u64>) -> Self {
        if let Some(nonce) = nonce {
            self.nonce = Some(nonce);
        }
        self
    }

    /// Conditionally sets the move precompile address and returns self.
    pub const fn with_move_precompile_to_opt(mut self, address: Option<Address>) -> Self {
        if let Some(address) = address {
            self.move_precompile_to = Some(address);
        }
        self
    }
}

/// Helper type that bundles various overrides for EVM Execution.
///
/// By `Default`, no overrides are included.
#[derive(Debug, Clone, Default)]
pub struct EvmOverrides {
    /// Applies overrides to the state before execution.
    pub state: Option<StateOverride>,
    /// Applies overrides to the block before execution.
    ///
    /// This is a `Box` because less common and only available in debug trace endpoints.
    pub block: Option<Box<BlockOverrides>>,
}

impl EvmOverrides {
    /// Creates a new instance with the given overrides
    pub const fn new(state: Option<StateOverride>, block: Option<Box<BlockOverrides>>) -> Self {
        Self { state, block }
    }

    /// Creates a new instance with the given state overrides.
    pub const fn state(state: Option<StateOverride>) -> Self {
        Self { state, block: None }
    }

    /// Creates a new instance with the given block overrides.
    pub const fn block(block: Option<Box<BlockOverrides>>) -> Self {
        Self { state: None, block }
    }

    /// Returns `true` if the overrides contain state overrides.
    pub const fn has_state(&self) -> bool {
        self.state.is_some()
    }

    /// Returns `true` if the overrides contain block overrides.
    pub const fn has_block(&self) -> bool {
        self.block.is_some()
    }

    /// Adds state overrides to an existing instance.
    pub fn with_state(mut self, state: StateOverride) -> Self {
        self.state = Some(state);
        self
    }

    /// Adds block overrides to an existing instance.
    pub fn with_block(mut self, block: Box<BlockOverrides>) -> Self {
        self.block = Some(block);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, map::B256HashMap, Bytes, B256, U256};
    use similar_asserts::assert_eq;

    #[test]
    fn test_default_account_override() {
        let acc_override = AccountOverride::default();
        assert!(acc_override.balance.is_none());
        assert!(acc_override.nonce.is_none());
        assert!(acc_override.code.is_none());
        assert!(acc_override.state.is_none());
        assert!(acc_override.state_diff.is_none());
    }

    #[test]
    #[cfg(feature = "serde")]
    #[should_panic(expected = "invalid type")]
    fn test_invalid_json_structure() {
        let invalid_json = r#"{
            "0x1234567890123456789012345678901234567890": {
                "balance": true
            }
        }"#;

        let _: StateOverride = serde_json::from_str(invalid_json).unwrap();
    }

    #[test]
    #[cfg(feature = "serde")]
    fn test_large_values_in_override() {
        let large_values_json = r#"{
            "0x1234567890123456789012345678901234567890": {
                "balance": "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
                "nonce": "0xffffffffffffffff"
            }
        }"#;

        let state_override: StateOverride = serde_json::from_str(large_values_json).unwrap();
        let acc =
            state_override.get(&address!("1234567890123456789012345678901234567890")).unwrap();
        assert_eq!(acc.balance, Some(U256::MAX));
        assert_eq!(acc.nonce, Some(u64::MAX));
    }

    #[test]
    #[cfg(feature = "serde")]
    fn test_state_override() {
        let s = r#"{
            "0x0000000000000000000000000000000000000124": {
                "code": "0x6080604052348015600e575f80fd5b50600436106026575f3560e01c80632096525514602a575b5f80fd5b60306044565b604051901515815260200160405180910390f35b5f604e600242605e565b5f0360595750600190565b505f90565b5f82607757634e487b7160e01b5f52601260045260245ffd5b50069056fea2646970667358221220287f77a4262e88659e3fb402138d2ee6a7ff9ba86bae487a95aa28156367d09c64736f6c63430008140033"
            }
        }"#;
        let state_override: StateOverride = serde_json::from_str(s).unwrap();
        let acc =
            state_override.get(&address!("0000000000000000000000000000000000000124")).unwrap();
        assert!(acc.code.is_some());
    }

    #[test]
    #[cfg(feature = "serde")]
    fn test_state_override_state_diff() {
        let s = r#"{
                "0x1b5212AF6b76113afD94cD2B5a78a73B7d7A8222": {
                    "balance": "0x39726378b58c400000",
                    "stateDiff": {}
                },
                "0xdAC17F958D2ee523a2206206994597C13D831ec7": {
                    "stateDiff": {
                        "0xede27e4e7f3676edbf125879f17a896d6507958df3d57bda6219f1880cae8a41": "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
                    }
                }
            }"#;
        let state_override: StateOverride = serde_json::from_str(s).unwrap();
        let acc =
            state_override.get(&address!("1b5212AF6b76113afD94cD2B5a78a73B7d7A8222")).unwrap();
        assert!(acc.state_diff.is_some());
    }

    #[test]
    fn test_set_code_in_place() {
        let mut account_override = AccountOverride::default();
        let code = Bytes::from(vec![0x60, 0x60, 0x60, 0x60]);
        account_override.set_code(code.clone());
        assert_eq!(account_override.code, Some(code));
    }

    #[test]
    fn test_set_state_in_place() {
        let mut account_override = AccountOverride::default();
        let state: B256HashMap<B256> = vec![(B256::ZERO, B256::ZERO)].into_iter().collect();
        account_override.set_state(state.clone());
        assert_eq!(account_override.state, Some(state));
    }

    #[test]
    fn test_set_state_diff_in_place() {
        let mut account_override = AccountOverride::default();
        let state_diff: B256HashMap<B256> = vec![(B256::ZERO, B256::ZERO)].into_iter().collect();
        account_override.set_state_diff(state_diff.clone());
        assert_eq!(account_override.state_diff, Some(state_diff));
    }

    #[test]
    fn test_set_balance_in_place() {
        let mut account_override = AccountOverride::default();
        let balance = U256::from(1000);
        account_override.set_balance(balance);
        assert_eq!(account_override.balance, Some(balance));
    }

    #[test]
    fn test_set_nonce_in_place() {
        let mut account_override = AccountOverride::default();
        let nonce = 42;
        account_override.set_nonce(nonce);
        assert_eq!(account_override.nonce, Some(nonce));
    }

    #[test]
    fn test_set_move_precompile_to_in_place() {
        let mut account_override = AccountOverride::default();
        let address = address!("0000000000000000000000000000000000000001");
        account_override.set_move_precompile_to(address);
        assert_eq!(account_override.move_precompile_to, Some(address));
    }

    #[test]
    fn test_evm_overrides_new() {
        let state = StateOverride::default();
        let block: Box<BlockOverrides> = Box::default();

        let evm_overrides = EvmOverrides::new(Some(state.clone()), Some(block.clone()));

        assert!(evm_overrides.has_state());
        assert!(evm_overrides.has_block());
        assert_eq!(evm_overrides.state.unwrap(), state);
        assert_eq!(*evm_overrides.block.unwrap(), *block);
    }

    #[test]
    fn test_evm_overrides_state() {
        let state = StateOverride::default();
        let evm_overrides = EvmOverrides::state(Some(state.clone()));

        assert!(evm_overrides.has_state());
        assert!(!evm_overrides.has_block());
        assert_eq!(evm_overrides.state.unwrap(), state);
    }

    #[test]
    fn test_evm_overrides_block() {
        let block: Box<BlockOverrides> = Box::default();
        let evm_overrides = EvmOverrides::block(Some(block.clone()));

        assert!(!evm_overrides.has_state());
        assert!(evm_overrides.has_block());
        assert_eq!(*evm_overrides.block.unwrap(), *block);
    }

    #[test]
    fn test_evm_overrides_with_state() {
        let state = StateOverride::default();
        let mut evm_overrides = EvmOverrides::default();

        assert!(!evm_overrides.has_state());

        evm_overrides = evm_overrides.with_state(state.clone());

        assert!(evm_overrides.has_state());
        assert_eq!(evm_overrides.state.unwrap(), state);
    }

    #[test]
    fn test_evm_overrides_with_block() {
        let block: Box<BlockOverrides> = Box::default();
        let mut evm_overrides = EvmOverrides::default();

        assert!(!evm_overrides.has_block());

        evm_overrides = evm_overrides.with_block(block.clone());

        assert!(evm_overrides.has_block());
        assert_eq!(*evm_overrides.block.unwrap(), *block);
    }
}
