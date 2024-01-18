/// Keccak256 over empty array.
pub const KECCAK_EMPTY: B256 =
    b256!("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470");
use alloy_primitives::{b256, B256, U256};

// An Ethereum account.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Account {
    /// Account nonce.
    pub nonce: u64,
    /// Account balance.
    pub balance: U256,
    /// Hash of the account's bytecode.
    pub bytecode_hash: Option<B256>,
}

impl Account {
    /// Whether the account has bytecode.
    pub fn has_bytecode(&self) -> bool {
        self.bytecode_hash.is_some()
    }

    /// After SpuriousDragon empty account is defined as account with nonce == 0 && balance == 0 &&
    /// bytecode = None.
    pub fn is_empty(&self) -> bool {
        let is_bytecode_empty = match self.bytecode_hash {
            None => true,
            Some(hash) => hash == KECCAK_EMPTY,
        };

        self.nonce == 0 && self.balance.is_zero() && is_bytecode_empty
    }

    /// Returns an account bytecode's hash.
    /// In case of no bytecode, returns [`KECCAK_EMPTY`].
    pub fn get_bytecode_hash(&self) -> B256 {
        match self.bytecode_hash {
            Some(hash) => hash,
            None => KECCAK_EMPTY,
        }
    }
}
